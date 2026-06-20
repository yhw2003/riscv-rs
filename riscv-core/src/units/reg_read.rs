use rhdl::prelude::*;

use crate::{InstFields, RegReadReq, RegReadResp};

#[derive(Copy, Clone, Debug, Default, PartialEq, Digital)]
pub struct RegReadInput {
    pub fields: InstFields,
    pub rdata: RegReadResp,
}

#[derive(Copy, Clone, Debug, Default, PartialEq, Digital)]
pub struct RegReadOutput {
    pub req: RegReadReq,
    pub rs1: b32,
    pub rs2: b32,
}

#[kernel]
pub fn read_regs(input: RegReadInput) -> RegReadOutput {
    RegReadOutput {
        req: RegReadReq {
            rs1_addr: input.fields.rs1,
            rs2_addr: input.fields.rs2,
        },
        rs1: input.rdata.rs1,
        rs2: input.rdata.rs2,
    }
}

#[derive(Clone, Debug, Default, Circuit)]
pub struct RegReadCircuit;

impl CircuitDQ for RegReadCircuit {
    type D = ();
    type Q = ();
}

impl CircuitIO for RegReadCircuit {
    type I = Signal<RegReadInput, Red>;
    type O = Signal<RegReadOutput, Red>;
    type Kernel = reg_read_circuit;
}

#[kernel]
pub fn reg_read_circuit(
    input: Signal<RegReadInput, Red>,
    _q: (),
) -> (Signal<RegReadOutput, Red>, ()) {
    (signal(read_regs(input.val())), ())
}

#[derive(Clone, Debug, Default, Synchronous)]
pub struct RegRead;

impl SynchronousIO for RegRead {
    type I = RegReadInput;
    type O = RegReadOutput;
    type Kernel = reg_read_sync;
}

impl SynchronousDQ for RegRead {
    type D = ();
    type Q = ();
}

#[kernel]
pub fn reg_read_sync(_cr: ClockReset, input: RegReadInput, _q: ()) -> (RegReadOutput, ()) {
    (read_regs(input), ())
}
