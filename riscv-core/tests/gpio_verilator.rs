use std::path::{Path, PathBuf};
use std::process::Command;

use rhdl::prelude::*;
use riscv_core::Rv32iSoc;

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

#[test]
fn gpio_program_passes_under_verilator() {
    let out_dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join("gpio_verilator");
    std::fs::create_dir_all(&out_dir).unwrap();

    let verilog = out_dir.join("rv32i_soc.v");
    let descriptor = Rv32iSoc::default().descriptor("rv32i_soc".into()).unwrap();
    let hdl = descriptor.hdl().unwrap();
    std::fs::write(&verilog, hdl.modules.to_string()).unwrap();

    let firmware_elf = out_dir.join("gpio.elf");
    let firmware_bin = out_dir.join("gpio.bin");
    let linker = manifest_path("verilator/gpio/linker.ld");
    let firmware_c = manifest_path("verilator/gpio/gpio.c");

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

    let sim_main = manifest_path("verilator/gpio/sim_main.cpp");
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
