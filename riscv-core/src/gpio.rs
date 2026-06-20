use rhdl::prelude::*;
use rhdl_fpga::core::dff::DFF;

use crate::MemReq;

pub const GPIO_BASE: b32 = b32(0x1000_0000);

#[derive(Copy, Clone, Debug, Default, PartialEq, Digital)]
pub struct GpioInput {
    pub dmem_req: MemReq,
}

#[derive(Copy, Clone, Debug, Default, PartialEq, Digital)]
pub struct GpioOutput {
    pub pins: b32,
    pub dmem_req: MemReq,
}

#[derive(Clone, Debug, Synchronous, SynchronousDQ)]
#[rhdl(dq_no_prefix)]
pub struct Gpio {
    pins: DFF<b32>,
}

impl Default for Gpio {
    fn default() -> Self {
        Self {
            pins: DFF::new(b32(0)),
        }
    }
}

impl SynchronousIO for Gpio {
    type I = GpioInput;
    type O = GpioOutput;
    type Kernel = gpio_kernel;
}

#[kernel]
pub fn apply_wstrb(old: b32, wdata: b32, wstrb: b4) -> b32 {
    let lane0 = if (wstrb & b4(0b0001)) != b4(0) {
        wdata & b32(0x0000_00ff)
    } else {
        old & b32(0x0000_00ff)
    };
    let lane1 = if (wstrb & b4(0b0010)) != b4(0) {
        wdata & b32(0x0000_ff00)
    } else {
        old & b32(0x0000_ff00)
    };
    let lane2 = if (wstrb & b4(0b0100)) != b4(0) {
        wdata & b32(0x00ff_0000)
    } else {
        old & b32(0x00ff_0000)
    };
    let lane3 = if (wstrb & b4(0b1000)) != b4(0) {
        wdata & b32(0xff00_0000)
    } else {
        old & b32(0xff00_0000)
    };
    lane0 | lane1 | lane2 | lane3
}

#[kernel]
pub fn gpio_decode(input: GpioInput, current_pins: b32) -> GpioOutput {
    let hit = input.dmem_req.valid && input.dmem_req.is_write && input.dmem_req.addr == GPIO_BASE;
    let mut dmem_req = input.dmem_req;
    if hit {
        dmem_req.valid = false;
    }
    GpioOutput {
        pins: if hit {
            apply_wstrb(current_pins, input.dmem_req.wdata, input.dmem_req.wstrb)
        } else {
            current_pins
        },
        dmem_req,
    }
}

#[kernel]
pub fn gpio_kernel(_cr: ClockReset, input: GpioInput, q: Q) -> (GpioOutput, D) {
    let out = gpio_decode(input, q.pins);
    (out, D { pins: out.pins })
}
