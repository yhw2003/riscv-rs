use rhdl::prelude::*;

#[derive(Copy, Clone, Debug, Default, PartialEq, Digital)]
pub enum AluOp {
    #[default]
    Add,
    Sub,
    Sll,
    Slt,
    Sltu,
    Xor,
    Srl,
    Sra,
    Or,
    And,
}

#[derive(Copy, Clone, Debug, Default, PartialEq, Digital)]
pub struct AluIn {
    pub op: AluOp,
    pub lhs: b32,
    pub rhs: b32,
}

#[kernel]
pub fn alu(input: AluIn) -> b32 {
    let shamt: b5 = input.rhs.resize();
    match input.op {
        AluOp::Add => input.lhs + input.rhs,
        AluOp::Sub => input.lhs - input.rhs,
        AluOp::Sll => input.lhs << shamt,
        AluOp::Slt => {
            if input.lhs.as_signed() < input.rhs.as_signed() {
                b32(1)
            } else {
                b32(0)
            }
        }
        AluOp::Sltu => {
            if input.lhs < input.rhs {
                b32(1)
            } else {
                b32(0)
            }
        }
        AluOp::Xor => input.lhs ^ input.rhs,
        AluOp::Srl => input.lhs >> shamt,
        AluOp::Sra => (input.lhs.as_signed() >> shamt).as_unsigned(),
        AluOp::Or => input.lhs | input.rhs,
        AluOp::And => input.lhs & input.rhs,
    }
}

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
