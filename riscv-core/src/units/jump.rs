use rhdl::prelude::*;

use crate::{OPCODE_JAL, OPCODE_JALR};

use super::ExecResult;

#[derive(Copy, Clone, Debug, Default, PartialEq, Digital)]
pub struct JumpInput {
    pub opcode: b7,
    pub funct3: b3,
    pub pc: b32,
    pub pc_plus_4: b32,
    pub rs1: b32,
    pub imm_i: b32,
    pub imm_j: b32,
}

#[kernel]
pub fn jump_exec(input: JumpInput) -> ExecResult {
    let mut out = ExecResult::default();
    out.next_pc = input.pc_plus_4;
    if input.opcode == OPCODE_JAL {
        out.rd_write = true;
        out.rd_wdata = input.pc_plus_4;
        out.next_pc = input.pc + input.imm_j;
    } else if input.opcode == OPCODE_JALR {
        if input.funct3 == b3(0) {
            out.rd_write = true;
            out.rd_wdata = input.pc_plus_4;
            out.next_pc = (input.rs1 + input.imm_i) & b32(0xffff_fffe);
        } else {
            out.illegal = true;
        }
    } else {
        out.illegal = true;
    }
    out
}

#[derive(Clone, Debug, Default, Circuit)]
pub struct JumpCircuit;

impl CircuitDQ for JumpCircuit {
    type D = ();
    type Q = ();
}

impl CircuitIO for JumpCircuit {
    type I = Signal<JumpInput, Red>;
    type O = Signal<ExecResult, Red>;
    type Kernel = jump_circuit;
}

#[kernel]
pub fn jump_circuit(input: Signal<JumpInput, Red>, _q: ()) -> (Signal<ExecResult, Red>, ()) {
    (signal(jump_exec(input.val())), ())
}

#[derive(Clone, Debug, Default, Synchronous)]
pub struct JumpUnit;

impl SynchronousIO for JumpUnit {
    type I = JumpInput;
    type O = ExecResult;
    type Kernel = jump_sync;
}

impl SynchronousDQ for JumpUnit {
    type D = ();
    type Q = ();
}

#[kernel]
pub fn jump_sync(_cr: ClockReset, input: JumpInput, _q: ()) -> (ExecResult, ()) {
    (jump_exec(input), ())
}
