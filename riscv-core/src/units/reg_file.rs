use rhdl::prelude::*;

use crate::{RegBus, RegFile, RegReadResp};

use super::{Register32, Register32Input};

#[derive(Copy, Clone, Debug, Default, PartialEq, Digital)]
pub struct RegFileInput {
    pub bus: RegBus,
}

#[derive(Copy, Clone, Debug, Default, PartialEq, Digital)]
pub struct RegFileOutput {
    pub rdata: RegReadResp,
}

#[kernel]
pub fn reg_file_apply(regs: RegFile, input: RegFileInput) -> RegFileOutput {
    let mut current = regs;
    current[0] = b32(0);

    RegFileOutput {
        rdata: RegReadResp {
            rs1: current[input.bus.read.rs1_addr],
            rs2: current[input.bus.read.rs2_addr],
        },
    }
}

#[derive(Clone, Debug, Synchronous, SynchronousDQ)]
#[rhdl(dq_no_prefix)]
pub struct RegFileUnit {
    regs: [Register32; 32],
}

impl Default for RegFileUnit {
    fn default() -> Self {
        Self {
            regs: std::array::from_fn(|_| Register32::default()),
        }
    }
}

impl SynchronousIO for RegFileUnit {
    type I = RegFileInput;
    type O = RegFileOutput;
    type Kernel = reg_file_kernel;
}

#[kernel]
pub fn reg_write_input(bus: RegBus, index: b5) -> Register32Input {
    Register32Input {
        write: bus.write.valid && bus.write.rd == index && index != b5(0),
        data: bus.write.data,
    }
}

#[kernel]
pub fn reg_file_kernel(_cr: ClockReset, input: RegFileInput, q: Q) -> (RegFileOutput, D) {
    let out = reg_file_apply(q.regs, input);
    (
        out,
        D {
            regs: [
                Register32Input {
                    write: false,
                    data: b32(0),
                },
                reg_write_input(input.bus, b5(1)),
                reg_write_input(input.bus, b5(2)),
                reg_write_input(input.bus, b5(3)),
                reg_write_input(input.bus, b5(4)),
                reg_write_input(input.bus, b5(5)),
                reg_write_input(input.bus, b5(6)),
                reg_write_input(input.bus, b5(7)),
                reg_write_input(input.bus, b5(8)),
                reg_write_input(input.bus, b5(9)),
                reg_write_input(input.bus, b5(10)),
                reg_write_input(input.bus, b5(11)),
                reg_write_input(input.bus, b5(12)),
                reg_write_input(input.bus, b5(13)),
                reg_write_input(input.bus, b5(14)),
                reg_write_input(input.bus, b5(15)),
                reg_write_input(input.bus, b5(16)),
                reg_write_input(input.bus, b5(17)),
                reg_write_input(input.bus, b5(18)),
                reg_write_input(input.bus, b5(19)),
                reg_write_input(input.bus, b5(20)),
                reg_write_input(input.bus, b5(21)),
                reg_write_input(input.bus, b5(22)),
                reg_write_input(input.bus, b5(23)),
                reg_write_input(input.bus, b5(24)),
                reg_write_input(input.bus, b5(25)),
                reg_write_input(input.bus, b5(26)),
                reg_write_input(input.bus, b5(27)),
                reg_write_input(input.bus, b5(28)),
                reg_write_input(input.bus, b5(29)),
                reg_write_input(input.bus, b5(30)),
                reg_write_input(input.bus, b5(31)),
            ],
        },
    )
}
