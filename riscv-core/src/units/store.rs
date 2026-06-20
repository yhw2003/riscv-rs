use rhdl::prelude::*;

use crate::{StoreKind, is_store_misaligned, store_req};

use super::ExecResult;

#[derive(Copy, Clone, Debug, Default, PartialEq, Digital)]
pub struct StoreInput {
    pub funct3: b3,
    pub pc_plus_4: b32,
    pub rs1: b32,
    pub rs2: b32,
    pub imm_s: b32,
}

#[kernel]
pub fn store_exec(input: StoreInput) -> ExecResult {
    let addr = input.rs1 + input.imm_s;
    let mut illegal = false;
    let kind = if input.funct3 == b3(0b000) {
        StoreKind::Byte
    } else if input.funct3 == b3(0b001) {
        StoreKind::Half
    } else if input.funct3 == b3(0b010) {
        StoreKind::Word
    } else {
        illegal = true;
        StoreKind::Byte
    };
    if is_store_misaligned(kind, addr) {
        illegal = true;
    }
    let mut dmem_req = store_req(kind, addr, input.rs2);
    if illegal {
        dmem_req.valid = false;
    }
    ExecResult {
        next_pc: input.pc_plus_4,
        rd_write: false,
        rd_wdata: b32(0),
        illegal,
        dmem_req,
    }
}

#[derive(Clone, Debug, Default, Circuit)]
pub struct StoreCircuit;

impl CircuitDQ for StoreCircuit {
    type D = ();
    type Q = ();
}

impl CircuitIO for StoreCircuit {
    type I = Signal<StoreInput, Red>;
    type O = Signal<ExecResult, Red>;
    type Kernel = store_circuit;
}

#[kernel]
pub fn store_circuit(input: Signal<StoreInput, Red>, _q: ()) -> (Signal<ExecResult, Red>, ()) {
    (signal(store_exec(input.val())), ())
}

#[derive(Clone, Debug, Default, Synchronous)]
pub struct StoreUnit;

impl SynchronousIO for StoreUnit {
    type I = StoreInput;
    type O = ExecResult;
    type Kernel = store_sync;
}

impl SynchronousDQ for StoreUnit {
    type D = ();
    type Q = ();
}

#[kernel]
pub fn store_sync(_cr: ClockReset, input: StoreInput, _q: ()) -> (ExecResult, ()) {
    (store_exec(input), ())
}
