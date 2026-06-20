use rhdl::prelude::*;
use riscv_core::Rv32iBramSoc;

fn main() -> anyhow::Result<()> {
    let core = Rv32iBramSoc::default();
    let descriptor = core.descriptor("rv32i_bram_soc".into())?;
    let hdl = descriptor.hdl()?;
    std::fs::create_dir_all("output")?;
    std::fs::write("output/o.v", hdl.modules.to_string())?;
    Ok(())
}
