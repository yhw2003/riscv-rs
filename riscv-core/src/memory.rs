use rhdl::prelude::*;
use rhdl_fpga::core::dff::DFF;
use rhdl_fpga::core::ram::synchronous::{In as BramIn, SyncBRAM, Write as BramWrite};

use crate::units::decoder::split_inst;
use crate::units::immediate::imm_i;
use crate::units::load::{LoadKind, load_value};
use crate::{
    CommitTrace, CoreState, Gpio, GpioInput, MemReq, RegBus, RegFileInput, RegFileUnit, RegReadReq,
    RegWriteReq, Rv32iStep, StepInput, StepOutput,
};

pub const BRAM_ADDR_BITS: usize = 14;
pub const BRAM_WORDS: usize = 1 << BRAM_ADDR_BITS;
pub const BRAM_BYTES: usize = BRAM_WORDS * 4;

pub type BramAddr = Bits<BRAM_ADDR_BITS>;

#[derive(Copy, Clone, Debug, Default, PartialEq, Digital)]
pub struct BramMemoryInput {
    pub imem_addr: b32,
    pub dmem_req: MemReq,
}

#[derive(Copy, Clone, Debug, Default, PartialEq, Digital)]
pub struct BramMemoryOutput {
    pub inst: b32,
    pub dmem_rdata: b32,
}

#[derive(Clone, Debug, Synchronous, SynchronousDQ)]
pub struct BramMemory {
    imem0: SyncBRAM<b8, BRAM_ADDR_BITS>,
    imem1: SyncBRAM<b8, BRAM_ADDR_BITS>,
    imem2: SyncBRAM<b8, BRAM_ADDR_BITS>,
    imem3: SyncBRAM<b8, BRAM_ADDR_BITS>,
    dmem0: SyncBRAM<b8, BRAM_ADDR_BITS>,
    dmem1: SyncBRAM<b8, BRAM_ADDR_BITS>,
    dmem2: SyncBRAM<b8, BRAM_ADDR_BITS>,
    dmem3: SyncBRAM<b8, BRAM_ADDR_BITS>,
}

impl Default for BramMemory {
    fn default() -> Self {
        Self::new([])
    }
}

impl BramMemory {
    pub fn new(initial_words: impl IntoIterator<Item = (BramAddr, b32)>) -> Self {
        let initial_words: Vec<_> = initial_words.into_iter().collect();
        Self {
            imem0: lane_bram(&initial_words, 0),
            imem1: lane_bram(&initial_words, 1),
            imem2: lane_bram(&initial_words, 2),
            imem3: lane_bram(&initial_words, 3),
            dmem0: lane_bram(&initial_words, 0),
            dmem1: lane_bram(&initial_words, 1),
            dmem2: lane_bram(&initial_words, 2),
            dmem3: lane_bram(&initial_words, 3),
        }
    }
}

fn lane_bram(initial_words: &[(BramAddr, b32)], lane: u32) -> SyncBRAM<b8, BRAM_ADDR_BITS> {
    let mut contents = vec![b8(0); BRAM_WORDS];
    for (addr, word) in initial_words {
        let index = addr.raw() as usize;
        if index < BRAM_WORDS {
            contents[index] = word_byte(*word, lane);
        }
    }
    SyncBRAM::new(
        contents
            .into_iter()
            .enumerate()
            .map(|(addr, byte)| (bits(addr as u128), byte)),
    )
}

fn word_byte(word: b32, lane: u32) -> b8 {
    b8((word.raw() >> (lane * 8)) & 0xff)
}

impl SynchronousIO for BramMemory {
    type I = BramMemoryInput;
    type O = BramMemoryOutput;
    type Kernel = bram_memory_kernel;
}

#[kernel]
pub fn bram_word_addr(addr: b32) -> BramAddr {
    (addr >> 2).resize()
}

#[kernel]
pub fn pack_word(byte0: b8, byte1: b8, byte2: b8, byte3: b8) -> b32 {
    byte0.resize::<32>()
        | (byte1.resize::<32>() << 8)
        | (byte2.resize::<32>() << 16)
        | (byte3.resize::<32>() << 24)
}

#[kernel]
pub fn bram_write(req: MemReq, value: b8, mask: b4) -> BramWrite<b8, BRAM_ADDR_BITS> {
    BramWrite::<b8, BRAM_ADDR_BITS> {
        addr: bram_word_addr(req.addr),
        value,
        enable: req.valid && req.is_write && (req.wstrb & mask) != b4(0),
    }
}

#[kernel]
pub fn bram_memory_kernel(
    _cr: ClockReset,
    input: BramMemoryInput,
    q: BramMemoryQ,
) -> (BramMemoryOutput, BramMemoryD) {
    let imem_addr = bram_word_addr(input.imem_addr);
    let dmem_addr = bram_word_addr(input.dmem_req.addr);
    let write0 = bram_write(input.dmem_req, input.dmem_req.wdata.resize(), b4(0b0001));
    let write1 = bram_write(
        input.dmem_req,
        (input.dmem_req.wdata >> 8).resize(),
        b4(0b0010),
    );
    let write2 = bram_write(
        input.dmem_req,
        (input.dmem_req.wdata >> 16).resize(),
        b4(0b0100),
    );
    let write3 = bram_write(
        input.dmem_req,
        (input.dmem_req.wdata >> 24).resize(),
        b4(0b1000),
    );

    let d = BramMemoryD {
        imem0: BramIn::<b8, BRAM_ADDR_BITS> {
            read_addr: imem_addr,
            write: write0,
        },
        imem1: BramIn::<b8, BRAM_ADDR_BITS> {
            read_addr: imem_addr,
            write: write1,
        },
        imem2: BramIn::<b8, BRAM_ADDR_BITS> {
            read_addr: imem_addr,
            write: write2,
        },
        imem3: BramIn::<b8, BRAM_ADDR_BITS> {
            read_addr: imem_addr,
            write: write3,
        },
        dmem0: BramIn::<b8, BRAM_ADDR_BITS> {
            read_addr: dmem_addr,
            write: write0,
        },
        dmem1: BramIn::<b8, BRAM_ADDR_BITS> {
            read_addr: dmem_addr,
            write: write1,
        },
        dmem2: BramIn::<b8, BRAM_ADDR_BITS> {
            read_addr: dmem_addr,
            write: write2,
        },
        dmem3: BramIn::<b8, BRAM_ADDR_BITS> {
            read_addr: dmem_addr,
            write: write3,
        },
    };

    (
        BramMemoryOutput {
            inst: pack_word(q.imem0, q.imem1, q.imem2, q.imem3),
            dmem_rdata: pack_word(q.dmem0, q.dmem1, q.dmem2, q.dmem3),
        },
        d,
    )
}

#[derive(Copy, Clone, Debug, Default, PartialEq, Digital)]
pub struct BramSocOutput {
    pub imem_addr: b32,
    pub dmem_req: MemReq,
    pub gpio_pins: b32,
    pub trace: CommitTrace,
    pub trap: bool,
}

#[derive(Copy, Clone, Debug, Default, PartialEq, Digital)]
pub struct PendingLoad {
    pub valid: bool,
    pub pc: b32,
    pub inst: b32,
    pub rd: b5,
    pub addr: b32,
    pub kind: LoadKind,
    pub next_pc: b32,
}

#[derive(Clone, Debug, Synchronous, SynchronousDQ)]
pub struct Rv32iBramSoc {
    state: DFF<CoreState>,
    reg_file: RegFileUnit,
    step: Rv32iStep,
    gpio: Gpio,
    memory: BramMemory,
    pending_load: DFF<PendingLoad>,
}

impl Default for Rv32iBramSoc {
    fn default() -> Self {
        Self {
            state: DFF::new(CoreState::default()),
            reg_file: RegFileUnit::default(),
            step: Rv32iStep::default(),
            gpio: Gpio::default(),
            memory: BramMemory::default(),
            pending_load: DFF::new(PendingLoad::default()),
        }
    }
}

impl Rv32iBramSoc {
    pub fn new(initial_words: impl IntoIterator<Item = (BramAddr, b32)>) -> Self {
        Self {
            state: DFF::new(CoreState::default()),
            reg_file: RegFileUnit::default(),
            step: Rv32iStep::default(),
            gpio: Gpio::default(),
            memory: BramMemory::new(initial_words),
            pending_load: DFF::new(PendingLoad::default()),
        }
    }
}

impl SynchronousIO for Rv32iBramSoc {
    type I = ();
    type O = BramSocOutput;
    type Kernel = bram_soc_kernel;
}

#[kernel]
pub fn load_kind_from_inst(inst: b32) -> LoadKind {
    let fields = split_inst(inst);
    if fields.funct3 == b3(0b000) {
        LoadKind::Byte
    } else if fields.funct3 == b3(0b001) {
        LoadKind::Half
    } else if fields.funct3 == b3(0b010) {
        LoadKind::Word
    } else if fields.funct3 == b3(0b100) {
        LoadKind::ByteUnsigned
    } else if fields.funct3 == b3(0b101) {
        LoadKind::HalfUnsigned
    } else {
        LoadKind::Byte
    }
}

#[kernel]
pub fn bram_soc_kernel(
    _cr: ClockReset,
    _input: (),
    q: Rv32iBramSocQ,
) -> (BramSocOutput, Rv32iBramSocD) {
    let mem_out: BramMemoryOutput = q.memory;
    let pending = q.pending_load;
    let step_input = StepInput {
        state: q.state,
        reg_rdata: q.reg_file.rdata,
        inst: if pending.valid {
            b32(0x0000_0013)
        } else {
            mem_out.inst
        },
        dmem_rdata: mem_out.dmem_rdata,
    };
    let step_out: StepOutput = q.step;
    let step_dmem_req = if pending.valid {
        MemReq::default()
    } else {
        step_out.dmem_req
    };
    let gpio_out = q.gpio;

    let load_request = step_dmem_req.valid && !step_dmem_req.is_write;
    let mut state_next = step_out.state;
    let mut reg_bus = step_out.reg_bus;
    let mut pending_next = PendingLoad::default();
    let mut memory_dmem_req = gpio_out.dmem_req;
    let mut memory_imem_addr = step_out.trace.next_pc;
    let mut trace = step_out.trace;

    if pending.valid {
        let rd_write = pending.rd != b5(0) && !q.state.trap;
        let rd_wdata = load_value(pending.kind, pending.addr, mem_out.dmem_rdata);
        state_next = CoreState {
            pc: pending.next_pc,
            trap: q.state.trap,
        };
        reg_bus = RegBus {
            read: RegReadReq::default(),
            write: RegWriteReq {
                valid: rd_write,
                rd: pending.rd,
                data: rd_wdata,
            },
        };
        memory_dmem_req = MemReq::default();
        memory_imem_addr = pending.next_pc;
        trace = CommitTrace {
            valid: !q.state.trap,
            pc: pending.pc,
            inst: pending.inst,
            rd: pending.rd,
            rd_write,
            rd_wdata,
            next_pc: pending.next_pc,
            trap: q.state.trap,
        };
    } else if load_request {
        let raw_addr = q.reg_file.rdata.rs1 + imm_i(step_out.trace.inst);
        state_next = q.state;
        reg_bus.write.valid = false;
        pending_next = PendingLoad {
            valid: true,
            pc: step_out.trace.pc,
            inst: step_out.trace.inst,
            rd: step_out.trace.rd,
            addr: raw_addr,
            kind: load_kind_from_inst(step_out.trace.inst),
            next_pc: step_out.trace.next_pc,
        };
        memory_imem_addr = step_out.trace.next_pc;
        trace.valid = false;
    }

    (
        BramSocOutput {
            imem_addr: step_out.imem_addr,
            dmem_req: memory_dmem_req,
            gpio_pins: gpio_out.pins,
            trace,
            trap: state_next.trap,
        },
        Rv32iBramSocD {
            state: state_next,
            reg_file: RegFileInput { bus: reg_bus },
            step: step_input,
            gpio: GpioInput {
                dmem_req: step_dmem_req,
            },
            memory: BramMemoryInput {
                imem_addr: memory_imem_addr,
                dmem_req: memory_dmem_req,
            },
            pending_load: pending_next,
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn encode_i(imm: i32, rs1: usize, funct3: u32, rd: usize, opcode: u32) -> u32 {
        (((imm as u32) & 0xfff) << 20)
            | ((rs1 as u32) << 15)
            | (funct3 << 12)
            | ((rd as u32) << 7)
            | opcode
    }

    fn encode_s(imm: i32, rs2: usize, rs1: usize, funct3: u32) -> u32 {
        let imm = (imm as u32) & 0xfff;
        ((imm >> 5) << 25)
            | ((rs2 as u32) << 20)
            | ((rs1 as u32) << 15)
            | (funct3 << 12)
            | ((imm & 0x1f) << 7)
            | 0b0100011
    }

    fn encode_u(imm: u32, rd: usize, opcode: u32) -> u32 {
        (imm & 0xffff_f000) | ((rd as u32) << 7) | opcode
    }

    fn lw(rd: usize, rs1: usize, imm: i32) -> u32 {
        encode_i(imm, rs1, 0b010, rd, 0b0000011)
    }

    fn sw(rs2: usize, rs1: usize, imm: i32) -> u32 {
        encode_s(imm, rs2, rs1, 0b010)
    }

    fn lui(rd: usize, imm: u32) -> u32 {
        encode_u(imm, rd, 0b0110111)
    }

    fn jal_zero() -> u32 {
        0x0000_006f
    }

    #[test]
    fn bram_soc_feeds_instruction_and_data_reads() {
        let uut = Rv32iBramSoc::new([
            (bits(0), b32(lw(1, 0, 32) as u128)),
            (bits(1), b32(lui(2, 0x1000_0000) as u128)),
            (bits(2), b32(sw(1, 2, 0) as u128)),
            (bits(3), b32(jal_zero() as u128)),
            (bits(8), b32(0x0000_005a)),
        ]);
        let stream = std::iter::repeat(())
            .take(40)
            .with_reset(1)
            .clock_pos_edge(100);
        let outputs: Vec<_> = uut
            .run(stream)
            .glitch_check(|x| (x.input.0.clock, x.output))
            .synchronous_sample()
            .skip(2)
            .map(|x| x.output)
            .collect();
        let gpio_write = outputs
            .iter()
            .position(|output| output.gpio_pins == b32(0x0000_005a));
        let gpio_write = gpio_write.expect("program should write loaded BRAM data to GPIO");
        assert!(outputs[..=gpio_write].iter().all(|output| !output.trap));
    }
}
