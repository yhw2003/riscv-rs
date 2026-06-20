use rhdl::prelude::*;
use rhdl_fpga::core::dff::DFF;

#[derive(Copy, Clone, Debug, Default, PartialEq, Digital)]
pub struct Register32Input {
    pub write: bool,
    pub data: b32,
}

#[kernel]
pub fn register32_next(current: b32, input: Register32Input) -> b32 {
    if input.write { input.data } else { current }
}

#[derive(Clone, Debug, Synchronous, SynchronousDQ)]
#[rhdl(dq_no_prefix)]
pub struct Register32 {
    value: DFF<b32>,
}

impl Default for Register32 {
    fn default() -> Self {
        Self {
            value: DFF::new(b32(0)),
        }
    }
}

impl SynchronousIO for Register32 {
    type I = Register32Input;
    type O = b32;
    type Kernel = register32_kernel;
}

#[kernel]
pub fn register32_kernel(_cr: ClockReset, input: Register32Input, q: Q) -> (b32, D) {
    (
        q.value,
        D {
            value: register32_next(q.value, input),
        },
    )
}
