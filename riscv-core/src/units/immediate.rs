use rhdl::prelude::*;

use crate::{imm_b, imm_i, imm_j, imm_s, imm_u};

#[derive(Copy, Clone, Debug, Default, PartialEq, Digital)]
pub struct ImmediateValues {
    pub i: b32,
    pub s: b32,
    pub b: b32,
    pub u: b32,
    pub j: b32,
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
