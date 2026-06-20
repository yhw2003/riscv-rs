use rhdl::prelude::*;

use crate::InstFields;

#[kernel]
pub fn split_inst(inst: b32) -> InstFields {
    InstFields {
        opcode: inst.resize(),
        rd: (inst >> 7).resize(),
        funct3: (inst >> 12).resize(),
        rs1: (inst >> 15).resize(),
        rs2: (inst >> 20).resize(),
        funct7: (inst >> 25).resize(),
    }
}

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
