use rhdl::prelude::*;

use crate::MemReq;

use super::ExecResult;

#[derive(Copy, Clone, Debug, Default, PartialEq, Digital)]
pub enum StoreKind {
    #[default]
    Byte,
    Half,
    Word,
}

#[derive(Copy, Clone, Debug, Default, PartialEq, Digital)]
pub struct StoreInput {
    pub funct3: b3,
    pub pc_plus_4: b32,
    pub rs1: b32,
    pub rs2: b32,
    pub imm_s: b32,
}

#[kernel]
pub fn store_req(kind: StoreKind, addr: b32, rs2: b32) -> MemReq {
    let byte_lane: b2 = addr.resize();
    let half_lane: b1 = (addr >> 1).resize();
    let aligned_addr = addr & b32(0xffff_fffc);
    let byte = rs2.resize::<8>().resize::<32>();
    let half = rs2.resize::<16>().resize::<32>();
    match kind {
        StoreKind::Byte => {
            let wdata = if byte_lane == b2(0) {
                byte
            } else if byte_lane == b2(1) {
                byte << 8
            } else if byte_lane == b2(2) {
                byte << 16
            } else {
                byte << 24
            };
            let wstrb = if byte_lane == b2(0) {
                b4(0b0001)
            } else if byte_lane == b2(1) {
                b4(0b0010)
            } else if byte_lane == b2(2) {
                b4(0b0100)
            } else {
                b4(0b1000)
            };
            MemReq {
                valid: true,
                is_write: true,
                addr: aligned_addr,
                wdata,
                wstrb,
            }
        }
        StoreKind::Half => {
            let wdata = if half_lane == b1(0) { half } else { half << 16 };
            let wstrb = if half_lane == b1(0) {
                b4(0b0011)
            } else {
                b4(0b1100)
            };
            MemReq {
                valid: true,
                is_write: true,
                addr: aligned_addr,
                wdata,
                wstrb,
            }
        }
        StoreKind::Word => MemReq {
            valid: true,
            is_write: true,
            addr: aligned_addr,
            wdata: rs2,
            wstrb: b4(0b1111),
        },
    }
}

#[kernel]
pub fn is_store_misaligned(kind: StoreKind, addr: b32) -> bool {
    let byte_lane: b2 = addr.resize();
    match kind {
        StoreKind::Byte => false,
        StoreKind::Half => (byte_lane & b2(1)) != b2(0),
        StoreKind::Word => byte_lane != b2(0),
    }
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
