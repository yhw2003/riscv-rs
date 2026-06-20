use rhdl::prelude::*;

use crate::{InstFields, split_inst};

#[derive(Clone, Debug, Default, Circuit)]
pub struct DecoderCircuit;

impl CircuitDQ for DecoderCircuit {
    type D = ();
    type Q = ();
}

impl CircuitIO for DecoderCircuit {
    type I = Signal<b32, Red>;
    type O = Signal<InstFields, Red>;
    type Kernel = decoder_circuit;
}

#[kernel]
pub fn decoder_circuit(input: Signal<b32, Red>, _q: ()) -> (Signal<InstFields, Red>, ()) {
    (signal(split_inst(input.val())), ())
}

#[derive(Clone, Debug, Default, Synchronous)]
pub struct Decoder;

impl SynchronousIO for Decoder {
    type I = b32;
    type O = InstFields;
    type Kernel = decoder_sync;
}

impl SynchronousDQ for Decoder {
    type D = ();
    type Q = ();
}

#[kernel]
pub fn decoder_sync(_cr: ClockReset, input: b32, _q: ()) -> (InstFields, ()) {
    (split_inst(input), ())
}
