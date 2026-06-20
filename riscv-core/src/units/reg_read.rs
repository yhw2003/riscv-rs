use rhdl::prelude::*;

use crate::{InstFields, RegFile};

#[derive(Copy, Clone, Debug, Default, PartialEq, Digital)]
pub struct RegReadInput {
    pub regs: RegFile,
    pub fields: InstFields,
}

#[derive(Copy, Clone, Debug, Default, PartialEq, Digital)]
pub struct RegReadOutput {
    pub rs1: b32,
    pub rs2: b32,
}

#[kernel]
pub fn read_regs(input: RegReadInput) -> RegReadOutput {
    RegReadOutput {
        rs1: input.regs[input.fields.rs1],
        rs2: input.regs[input.fields.rs2],
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
