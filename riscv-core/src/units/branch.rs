use rhdl::prelude::*;

use crate::branch_taken;

use super::ExecResult;

#[derive(Copy, Clone, Debug, Default, PartialEq, Digital)]
pub struct BranchInput {
    pub funct3: b3,
    pub pc: b32,
    pub pc_plus_4: b32,
    pub rs1: b32,
    pub rs2: b32,
    pub imm_b: b32,
}

#[kernel]
pub fn branch_exec(input: BranchInput) -> ExecResult {
    let (take_branch, bad_branch) = branch_taken(input.funct3, input.rs1, input.rs2);
    let mut out = ExecResult::default();
    out.next_pc = input.pc_plus_4;
    out.illegal = bad_branch;
    if take_branch {
        out.next_pc = input.pc + input.imm_b;
    }
    out
}

#[derive(Clone, Debug, Default, Circuit)]
pub struct BranchCircuit;

impl CircuitDQ for BranchCircuit {
    type D = ();
    type Q = ();
}

impl CircuitIO for BranchCircuit {
    type I = Signal<BranchInput, Red>;
    type O = Signal<ExecResult, Red>;
    type Kernel = branch_circuit;
}

#[kernel]
pub fn branch_circuit(input: Signal<BranchInput, Red>, _q: ()) -> (Signal<ExecResult, Red>, ()) {
    (signal(branch_exec(input.val())), ())
}

#[derive(Clone, Debug, Default, Synchronous)]
pub struct BranchUnit;

impl SynchronousIO for BranchUnit {
    type I = BranchInput;
    type O = ExecResult;
    type Kernel = branch_sync;
}

impl SynchronousDQ for BranchUnit {
    type D = ();
    type Q = ();
}

#[kernel]
pub fn branch_sync(_cr: ClockReset, input: BranchInput, _q: ()) -> (ExecResult, ()) {
    (branch_exec(input), ())
}
