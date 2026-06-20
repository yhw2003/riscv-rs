use rhdl::prelude::*;

use crate::MemReq;

use super::ExecResult;

#[derive(Copy, Clone, Debug, Default, PartialEq, Digital)]
pub enum LoadKind {
    #[default]
    Byte,
    Half,
    Word,
    ByteUnsigned,
    HalfUnsigned,
}

#[derive(Copy, Clone, Debug, Default, PartialEq, Digital)]
pub struct LoadInput {
    pub funct3: b3,
    pub pc_plus_4: b32,
    pub rs1: b32,
    pub imm_i: b32,
    pub dmem_rdata: b32,
}

#[kernel]
pub fn load_value(kind: LoadKind, addr: b32, rdata: b32) -> b32 {
    let byte_lane: b2 = addr.resize();
    let half_lane: b1 = (addr >> 1).resize();
    let byte = if byte_lane == b2(0) {
        rdata.resize::<8>()
    } else if byte_lane == b2(1) {
        (rdata >> 8).resize::<8>()
    } else if byte_lane == b2(2) {
        (rdata >> 16).resize::<8>()
    } else {
        (rdata >> 24).resize::<8>()
    };
    let half = if half_lane == b1(0) {
        rdata.resize::<16>()
    } else {
        (rdata >> 16).resize::<16>()
    };
    match kind {
        LoadKind::Byte => byte.as_signed().resize::<32>().as_unsigned(),
        LoadKind::Half => half.as_signed().resize::<32>().as_unsigned(),
        LoadKind::Word => rdata,
        LoadKind::ByteUnsigned => byte.resize(),
        LoadKind::HalfUnsigned => half.resize(),
    }
}

#[kernel]
pub fn is_load_misaligned(kind: LoadKind, addr: b32) -> bool {
    let byte_lane: b2 = addr.resize();
    match kind {
        LoadKind::Byte => false,
        LoadKind::Half => (byte_lane & b2(1)) != b2(0),
        LoadKind::Word => byte_lane != b2(0),
        LoadKind::ByteUnsigned => false,
        LoadKind::HalfUnsigned => (byte_lane & b2(1)) != b2(0),
    }
}

#[kernel]
pub fn load_exec(input: LoadInput) -> ExecResult {
    let addr = input.rs1 + input.imm_i;
    let mut illegal = false;
    let kind = if input.funct3 == b3(0b000) {
        LoadKind::Byte
    } else if input.funct3 == b3(0b001) {
        LoadKind::Half
    } else if input.funct3 == b3(0b010) {
        LoadKind::Word
    } else if input.funct3 == b3(0b100) {
        LoadKind::ByteUnsigned
    } else if input.funct3 == b3(0b101) {
        LoadKind::HalfUnsigned
    } else {
        illegal = true;
        LoadKind::Byte
    };
    if is_load_misaligned(kind, addr) {
        illegal = true;
    }
    ExecResult {
        next_pc: input.pc_plus_4,
        rd_write: !illegal,
        rd_wdata: load_value(kind, addr, input.dmem_rdata),
        illegal,
        dmem_req: MemReq {
            valid: !illegal,
            is_write: false,
            addr: addr & b32(0xffff_fffc),
            wdata: b32(0),
            wstrb: b4(0),
        },
    }
}

#[derive(Clone, Debug, Default, Circuit)]
pub struct LoadCircuit;

impl CircuitDQ for LoadCircuit {
    type D = ();
    type Q = ();
}

impl CircuitIO for LoadCircuit {
    type I = Signal<LoadInput, Red>;
    type O = Signal<ExecResult, Red>;
    type Kernel = load_circuit;
}

#[kernel]
pub fn load_circuit(input: Signal<LoadInput, Red>, _q: ()) -> (Signal<ExecResult, Red>, ()) {
    (signal(load_exec(input.val())), ())
}

#[derive(Clone, Debug, Default, Synchronous)]
pub struct LoadUnit;

impl SynchronousIO for LoadUnit {
    type I = LoadInput;
    type O = ExecResult;
    type Kernel = load_sync;
}

impl SynchronousDQ for LoadUnit {
    type D = ();
    type Q = ();
}

#[kernel]
pub fn load_sync(_cr: ClockReset, input: LoadInput, _q: ()) -> (ExecResult, ()) {
    (load_exec(input), ())
}
