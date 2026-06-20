use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Mutex;

use rhdl::prelude::*;
use riscv_core::{BRAM_BYTES, BramAddr, Rv32iBramSoc, Rv32iSoc};

static VERILATOR_TEST_LOCK: Mutex<()> = Mutex::new(());

fn run(command: &mut Command) {
    let output = command.output().expect("failed to spawn command");
    if !output.status.success() {
        panic!(
            "command failed: {:?}\nstatus: {}\nstdout:\n{}\nstderr:\n{}",
            command,
            output.status,
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
}

fn manifest_path(path: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join(path)
}

fn command_exists(name: &str) -> bool {
    Command::new("sh")
        .arg("-c")
        .arg(format!("command -v {name}"))
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

fn build_firmware(name: &str, out_dir: &Path) -> PathBuf {
    let firmware_elf = out_dir.join(format!("{name}.elf"));
    let firmware_bin = out_dir.join(format!("{name}.bin"));
    let linker = manifest_path(&format!("verilator/{name}/linker.ld"));
    let firmware_c = manifest_path(&format!("verilator/{name}/{name}.c"));

    let gcc = if command_exists("riscv64-linux-gnu-gcc") {
        "riscv64-linux-gnu-gcc"
    } else if command_exists("riscv32-unknown-elf-gcc") {
        "riscv32-unknown-elf-gcc"
    } else {
        "clang"
    };
    let mut cc = Command::new(gcc);
    if gcc == "clang" {
        cc.arg("--target=riscv32-unknown-elf");
    }
    run(cc
        .arg("-march=rv32i")
        .arg("-mabi=ilp32")
        .arg("-ffreestanding")
        .arg("-fno-builtin")
        .arg("-fno-pic")
        .arg("-fno-pie")
        .arg("-fno-unroll-loops")
        .arg("-O2")
        .arg("-nostdlib")
        .arg("-Wl,--no-relax")
        .arg("-Wl,--build-id=none")
        .arg("-Wl,-T")
        .arg(&linker)
        .arg(&firmware_c)
        .arg("-o")
        .arg(&firmware_elf));

    let objcopy = if command_exists("riscv64-linux-gnu-objcopy") {
        "riscv64-linux-gnu-objcopy"
    } else if command_exists("riscv32-unknown-elf-objcopy") {
        "riscv32-unknown-elf-objcopy"
    } else {
        "llvm-objcopy"
    };
    run(Command::new(objcopy)
        .arg("-O")
        .arg("binary")
        .arg(&firmware_elf)
        .arg(&firmware_bin));

    firmware_bin
}

fn firmware_words(path: &Path) -> Vec<(BramAddr, b32)> {
    let firmware = std::fs::read(path).unwrap();
    assert!(
        firmware.len() <= BRAM_BYTES,
        "firmware too large: {} bytes",
        firmware.len()
    );
    firmware
        .chunks(4)
        .enumerate()
        .map(|(index, chunk)| {
            let mut bytes = [0_u8; 4];
            bytes[..chunk.len()].copy_from_slice(chunk);
            (bits(index as u128), b32(u32::from_le_bytes(bytes) as u128))
        })
        .collect()
}

fn run_verilator_example(name: &str) {
    let _guard = VERILATOR_TEST_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let out_dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("{name}_verilator"));
    std::fs::create_dir_all(&out_dir).unwrap();

    let verilog = out_dir.join("rv32i_soc.v");
    let descriptor = Rv32iSoc::default().descriptor("rv32i_soc".into()).unwrap();
    let hdl = descriptor.hdl().unwrap();
    std::fs::write(&verilog, hdl.modules.to_string()).unwrap();

    let firmware_bin = build_firmware(name, &out_dir);

    let sim_main = manifest_path(&format!("verilator/{name}/sim_main.cpp"));
    let obj_dir = out_dir.join("obj_dir");
    run(Command::new("verilator")
        .arg("--cc")
        .arg("--exe")
        .arg("--build")
        .arg("--Mdir")
        .arg(&obj_dir)
        .arg("--top-module")
        .arg("rv32i_soc")
        .arg(&verilog)
        .arg(&sim_main)
        .arg("-CFLAGS")
        .arg("-std=c++17")
        .arg("-CFLAGS")
        .arg("-O2")
        .arg("--Wno-fatal"));

    run(Command::new(obj_dir.join("Vrv32i_soc")).arg(&firmware_bin));
}

fn run_bram_verilator_example(name: &str) {
    let _guard = VERILATOR_TEST_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let out_dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("{name}_bram_verilator"));
    std::fs::create_dir_all(&out_dir).unwrap();

    let firmware_bin = build_firmware(name, &out_dir);
    let verilog = out_dir.join("rv32i_bram_soc.v");
    let descriptor = Rv32iBramSoc::new(firmware_words(&firmware_bin))
        .descriptor("rv32i_bram_soc".into())
        .unwrap();
    let hdl = descriptor.hdl().unwrap();
    std::fs::write(&verilog, hdl.modules.to_string()).unwrap();

    let sim_main = manifest_path("verilator/bram_soc/sim_main.cpp");
    let obj_dir = out_dir.join("obj_dir");
    run(Command::new("verilator")
        .arg("--cc")
        .arg("--exe")
        .arg("--build")
        .arg("--Mdir")
        .arg(&obj_dir)
        .arg("--top-module")
        .arg("rv32i_bram_soc")
        .arg(&verilog)
        .arg(&sim_main)
        .arg("-CFLAGS")
        .arg("-std=c++17")
        .arg("-CFLAGS")
        .arg("-O2")
        .arg("--Wno-fatal"));

    run(Command::new(obj_dir.join("Vrv32i_bram_soc")).arg(name));
}

#[test]
fn gpio_program_passes_under_verilator() {
    run_verilator_example("gpio");
}

#[test]
fn control_flow_program_passes_under_verilator() {
    run_verilator_example("control_flow");
}

#[test]
fn gpio_program_passes_under_bram_verilator() {
    run_bram_verilator_example("gpio");
}

#[test]
fn control_flow_program_passes_under_bram_verilator() {
    run_bram_verilator_example("control_flow");
}
