use rhdl::prelude::*;

use crate::MemReq;

use super::ExecResult;

#[derive(Copy, Clone, Debug, Default, PartialEq, Digital)]
pub struct FenceInput {
    pub funct3: b3,
    pub pc_plus_4: b32,
}

#[kernel]
pub fn fence_exec(input: FenceInput) -> ExecResult {
    ExecResult {
        next_pc: input.pc_plus_4,
        rd_write: false,
        rd_wdata: b32(0),
        illegal: input.funct3 != b3(0),
        dmem_req: MemReq::default(),
    }
}

#[derive(Clone, Debug, Default, Circuit)]
pub struct FenceCircuit;

impl CircuitDQ for FenceCircuit {
    type D = ();
    type Q = ();
}

impl CircuitIO for FenceCircuit {
    type I = Signal<FenceInput, Red>;
    type O = Signal<ExecResult, Red>;
    type Kernel = fence_circuit;
}

#[kernel]
pub fn fence_circuit(input: Signal<FenceInput, Red>, _q: ()) -> (Signal<ExecResult, Red>, ()) {
    (signal(fence_exec(input.val())), ())
}

#[derive(Clone, Debug, Default, Synchronous)]
pub struct FenceUnit;

impl SynchronousIO for FenceUnit {
    type I = FenceInput;
    type O = ExecResult;
    type Kernel = fence_sync;
}

impl SynchronousDQ for FenceUnit {
    type D = ();
    type Q = ();
}

#[kernel]
pub fn fence_sync(_cr: ClockReset, input: FenceInput, _q: ()) -> (ExecResult, ()) {
    (fence_exec(input), ())
}
