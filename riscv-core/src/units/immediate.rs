use rhdl::prelude::*;

#[derive(Copy, Clone, Debug, Default, PartialEq, Digital)]
pub struct ImmediateValues {
    pub i: b32,
    pub s: b32,
    pub b: b32,
    pub u: b32,
    pub j: b32,
}

#[kernel]
pub fn imm_i(inst: b32) -> b32 {
    let imm: b12 = (inst >> 20).resize();
    imm.as_signed().resize::<32>().as_unsigned()
}

#[kernel]
pub fn imm_s(inst: b32) -> b32 {
    let lo: b12 = (inst >> 7).resize::<5>().resize();
    let hi: b12 = ((inst >> 25).resize::<7>().resize()) << 5;
    (hi | lo).as_signed().resize::<32>().as_unsigned()
}

#[kernel]
pub fn imm_b(inst: b32) -> b32 {
    let bit_12: b13 = ((inst >> 31).resize::<13>()) << 12;
    let bit_11: b13 = ((inst >> 7).resize::<13>() & b13(1)) << 11;
    let bits_10_5: b13 = ((inst >> 25).resize::<13>() & b13(0b11_1111)) << 5;
    let bits_4_1: b13 = ((inst >> 8).resize::<13>() & b13(0b1111)) << 1;
    (bit_12 | bit_11 | bits_10_5 | bits_4_1)
        .as_signed()
        .resize::<32>()
        .as_unsigned()
}

#[kernel]
pub fn imm_u(inst: b32) -> b32 {
    inst & b32(0xffff_f000)
}

#[kernel]
pub fn imm_j(inst: b32) -> b32 {
    let bit_20: b21 = ((inst >> 31).resize::<21>()) << 20;
    let bits_19_12: b21 = ((inst >> 12).resize::<21>() & b21(0xff)) << 12;
    let bit_11: b21 = ((inst >> 20).resize::<21>() & b21(1)) << 11;
    let bits_10_1: b21 = ((inst >> 21).resize::<21>() & b21(0x3ff)) << 1;
    (bit_20 | bits_19_12 | bit_11 | bits_10_1)
        .as_signed()
        .resize::<32>()
        .as_unsigned()
}

#[kernel]
pub fn immediates(input: b32) -> ImmediateValues {
    ImmediateValues {
        i: imm_i(input),
        s: imm_s(input),
        b: imm_b(input),
        u: imm_u(input),
        j: imm_j(input),
    }
}

#[derive(Clone, Debug, Default, Circuit)]
pub struct ImmediateCircuit;

impl CircuitDQ for ImmediateCircuit {
    type D = ();
    type Q = ();
}

impl CircuitIO for ImmediateCircuit {
    type I = Signal<b32, Red>;
    type O = Signal<ImmediateValues, Red>;
    type Kernel = immediate_circuit;
}

#[kernel]
pub fn immediate_circuit(input: Signal<b32, Red>, _q: ()) -> (Signal<ImmediateValues, Red>, ()) {
    (signal(immediates(input.val())), ())
}

#[derive(Clone, Debug, Default, Synchronous)]
pub struct ImmediateGenerator;

impl SynchronousIO for ImmediateGenerator {
    type I = b32;
    type O = ImmediateValues;
    type Kernel = immediate_sync;
}

impl SynchronousDQ for ImmediateGenerator {
    type D = ();
    type Q = ();
}

#[kernel]
pub fn immediate_sync(_cr: ClockReset, input: b32, _q: ()) -> (ImmediateValues, ()) {
    (immediates(input), ())
}
