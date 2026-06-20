use rhdl::prelude::*;

use crate::{AluIn, alu};

#[derive(Clone, Debug, Default, Circuit)]
pub struct AluCircuit;

impl CircuitDQ for AluCircuit {
    type D = ();
    type Q = ();
}

impl CircuitIO for AluCircuit {
    type I = Signal<AluIn, Red>;
    type O = Signal<b32, Red>;
    type Kernel = alu_circuit;
}

#[kernel]
pub fn alu_circuit(input: Signal<AluIn, Red>, _q: ()) -> (Signal<b32, Red>, ()) {
    (signal(alu(input.val())), ())
}

#[derive(Clone, Debug, Default, Synchronous)]
pub struct AluSync;

impl SynchronousIO for AluSync {
    type I = AluIn;
    type O = b32;
    type Kernel = alu_sync;
}

impl SynchronousDQ for AluSync {
    type D = ();
    type Q = ();
}

#[kernel]
pub fn alu_sync(_cr: ClockReset, input: AluIn, _q: ()) -> (b32, ()) {
    (alu(input), ())
}
