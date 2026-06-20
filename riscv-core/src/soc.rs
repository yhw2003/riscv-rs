use rhdl::prelude::*;
use rhdl_fpga::core::dff::DFF;

use crate::{
    CommitTrace, CoreState, Gpio, GpioInput, MemReq, RegFileUnit, Rv32iStep, StepInput, StepOutput,
};

#[derive(Copy, Clone, Debug, Default, PartialEq, Digital)]
pub struct SocInput {
    pub inst: b32,
    pub dmem_rdata: b32,
}

#[derive(Copy, Clone, Debug, Default, PartialEq, Digital)]
pub struct SocOutput {
    pub imem_addr: b32,
    pub dmem_req: MemReq,
    pub gpio_pins: b32,
    pub trace: CommitTrace,
    pub trap: bool,
}

#[derive(Clone, Debug, Synchronous, SynchronousDQ)]
#[rhdl(dq_no_prefix)]
pub struct Rv32iSoc {
    state: DFF<CoreState>,
    reg_file: RegFileUnit,
    step: Rv32iStep,
    gpio: Gpio,
}

impl Default for Rv32iSoc {
    fn default() -> Self {
        Self {
            state: DFF::new(CoreState::default()),
            reg_file: RegFileUnit::default(),
            step: Rv32iStep::default(),
            gpio: Gpio::default(),
        }
    }
}

impl SynchronousIO for Rv32iSoc {
    type I = SocInput;
    type O = SocOutput;
    type Kernel = soc_kernel;
}

#[kernel]
pub fn soc_kernel(_cr: ClockReset, input: SocInput, q: Q) -> (SocOutput, D) {
    let step_input = StepInput {
        state: q.state,
        reg_rdata: q.reg_file.rdata,
        inst: input.inst,
        dmem_rdata: input.dmem_rdata,
    };
    let step_out: StepOutput = q.step;
    let gpio_out = q.gpio;
    let mut d = D::dont_care();
    d.step = step_input;
    d.reg_file = crate::RegFileInput {
        bus: step_out.reg_bus,
    };
    d.gpio = GpioInput {
        dmem_req: step_out.dmem_req,
    };
    d.state = step_out.state;

    (
        SocOutput {
            imem_addr: step_out.imem_addr,
            dmem_req: gpio_out.dmem_req,
            gpio_pins: gpio_out.pins,
            trace: step_out.trace,
            trap: step_out.state.trap,
        },
        d,
    )
}
