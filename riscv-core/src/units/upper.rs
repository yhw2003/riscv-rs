use rhdl::prelude::*;

use crate::{OPCODE_AUIPC, OPCODE_LUI};

use super::ExecResult;

#[derive(Copy, Clone, Debug, Default, PartialEq, Digital)]
pub struct UpperInput {
    pub opcode: b7,
    pub pc: b32,
    pub pc_plus_4: b32,
    pub imm_u: b32,
}

#[kernel]
pub fn upper_exec(input: UpperInput) -> ExecResult {
    let mut out = ExecResult::default();
    out.next_pc = input.pc_plus_4;
    if input.opcode == OPCODE_LUI {
        out.rd_write = true;
        out.rd_wdata = input.imm_u;
    } else if input.opcode == OPCODE_AUIPC {
        out.rd_write = true;
        out.rd_wdata = input.pc + input.imm_u;
    } else {
        out.illegal = true;
    }
    out
}

#[derive(Clone, Debug, Default, Circuit)]
pub struct UpperCircuit;

impl CircuitDQ for UpperCircuit {
    type D = ();
    type Q = ();
}

impl CircuitIO for UpperCircuit {
    type I = Signal<UpperInput, Red>;
    type O = Signal<ExecResult, Red>;
    type Kernel = upper_circuit;
}

#[kernel]
pub fn upper_circuit(input: Signal<UpperInput, Red>, _q: ()) -> (Signal<ExecResult, Red>, ()) {
    (signal(upper_exec(input.val())), ())
}

#[derive(Clone, Debug, Default, Synchronous)]
pub struct UpperUnit;

impl SynchronousIO for UpperUnit {
    type I = UpperInput;
    type O = ExecResult;
    type Kernel = upper_sync;
}

impl SynchronousDQ for UpperUnit {
    type D = ();
    type Q = ();
}

#[kernel]
pub fn upper_sync(_cr: ClockReset, input: UpperInput, _q: ()) -> (ExecResult, ()) {
    (upper_exec(input), ())
}
