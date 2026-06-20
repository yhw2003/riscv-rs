use rhdl::prelude::*;

pub mod gpio;
pub mod soc;
pub mod step;
pub mod units;

pub use gpio::{GPIO_BASE, Gpio, GpioInput, GpioOutput, apply_wstrb, gpio_decode, gpio_kernel};
pub use soc::{Rv32iSoc, SocInput, SocOutput, soc_kernel};
pub use step::{Rv32iStep, finish_step, rv32i_step, rv32i_step_module, select_exec_result};
pub use units::alu::alu;
pub use units::*;

pub type RegFile = [b32; 32];

const OPCODE_LUI: b7 = b7(0b0110111);
const OPCODE_AUIPC: b7 = b7(0b0010111);
const OPCODE_JAL: b7 = b7(0b1101111);
const OPCODE_JALR: b7 = b7(0b1100111);
const OPCODE_BRANCH: b7 = b7(0b1100011);
const OPCODE_LOAD: b7 = b7(0b0000011);
const OPCODE_STORE: b7 = b7(0b0100011);
const OPCODE_OP_IMM: b7 = b7(0b0010011);
const OPCODE_OP: b7 = b7(0b0110011);
const OPCODE_MISC_MEM: b7 = b7(0b0001111);
const OPCODE_SYSTEM: b7 = b7(0b1110011);

#[derive(Copy, Clone, Debug, Default, PartialEq, Digital)]
pub struct CoreState {
    pub pc: b32,
    pub regs: RegFile,
    pub trap: bool,
}

#[derive(Copy, Clone, Debug, Default, PartialEq, Digital)]
pub struct StepInput {
    pub state: CoreState,
    pub inst: b32,
    pub dmem_rdata: b32,
}

#[derive(Copy, Clone, Debug, Default, PartialEq, Digital)]
pub struct InstFields {
    pub opcode: b7,
    pub rd: b5,
    pub funct3: b3,
    pub rs1: b5,
    pub rs2: b5,
    pub funct7: b7,
}

#[derive(Copy, Clone, Debug, Default, PartialEq, Digital)]
pub struct MemReq {
    pub valid: bool,
    pub is_write: bool,
    pub addr: b32,
    pub wdata: b32,
    pub wstrb: b4,
}

#[derive(Copy, Clone, Debug, Default, PartialEq, Digital)]
pub struct CommitTrace {
    pub valid: bool,
    pub pc: b32,
    pub inst: b32,
    pub rd: b5,
    pub rd_write: bool,
    pub rd_wdata: b32,
    pub next_pc: b32,
    pub trap: bool,
}

#[derive(Copy, Clone, Debug, Default, PartialEq, Digital)]
pub struct StepOutput {
    pub state: CoreState,
    pub imem_addr: b32,
    pub dmem_req: MemReq,
    pub trace: CommitTrace,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone, Debug, Default, PartialEq)]
    struct RefState {
        pc: u32,
        regs: [u32; 32],
        trap: bool,
    }

    fn b(value: u32) -> b32 {
        b32(value as u128)
    }

    fn raw(value: b32) -> u32 {
        value.raw() as u32
    }

    fn sign_extend(value: u32, bits: u32) -> u32 {
        let shift = 32 - bits;
        (((value << shift) as i32) >> shift) as u32
    }

    fn encode_r(funct7: u32, rs2: usize, rs1: usize, funct3: u32, rd: usize, opcode: u32) -> u32 {
        (funct7 << 25)
            | ((rs2 as u32) << 20)
            | ((rs1 as u32) << 15)
            | (funct3 << 12)
            | ((rd as u32) << 7)
            | opcode
    }

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

    fn encode_b(imm: i32, rs2: usize, rs1: usize, funct3: u32) -> u32 {
        let imm = (imm as u32) & 0x1fff;
        (((imm >> 12) & 0x1) << 31)
            | (((imm >> 5) & 0x3f) << 25)
            | ((rs2 as u32) << 20)
            | ((rs1 as u32) << 15)
            | (funct3 << 12)
            | (((imm >> 1) & 0xf) << 8)
            | (((imm >> 11) & 0x1) << 7)
            | 0b1100011
    }

    fn encode_u(imm: u32, rd: usize, opcode: u32) -> u32 {
        (imm & 0xffff_f000) | ((rd as u32) << 7) | opcode
    }

    fn encode_j(imm: i32, rd: usize) -> u32 {
        let imm = (imm as u32) & 0x1f_ffff;
        (((imm >> 20) & 0x1) << 31)
            | (((imm >> 1) & 0x3ff) << 21)
            | (((imm >> 11) & 0x1) << 20)
            | (((imm >> 12) & 0xff) << 12)
            | ((rd as u32) << 7)
            | 0b1101111
    }

    fn addi(rd: usize, rs1: usize, imm: i32) -> u32 {
        encode_i(imm, rs1, 0b000, rd, 0b0010011)
    }

    fn slti(rd: usize, rs1: usize, imm: i32) -> u32 {
        encode_i(imm, rs1, 0b010, rd, 0b0010011)
    }

    fn sltiu(rd: usize, rs1: usize, imm: i32) -> u32 {
        encode_i(imm, rs1, 0b011, rd, 0b0010011)
    }

    fn xori(rd: usize, rs1: usize, imm: i32) -> u32 {
        encode_i(imm, rs1, 0b100, rd, 0b0010011)
    }

    fn ori(rd: usize, rs1: usize, imm: i32) -> u32 {
        encode_i(imm, rs1, 0b110, rd, 0b0010011)
    }

    fn andi(rd: usize, rs1: usize, imm: i32) -> u32 {
        encode_i(imm, rs1, 0b111, rd, 0b0010011)
    }

    fn slli(rd: usize, rs1: usize, shamt: u32) -> u32 {
        encode_i(shamt as i32, rs1, 0b001, rd, 0b0010011)
    }

    fn srli(rd: usize, rs1: usize, shamt: u32) -> u32 {
        encode_i(shamt as i32, rs1, 0b101, rd, 0b0010011)
    }

    fn srai(rd: usize, rs1: usize, shamt: u32) -> u32 {
        encode_i((0b0100000 << 5) | shamt as i32, rs1, 0b101, rd, 0b0010011)
    }

    fn add(rd: usize, rs1: usize, rs2: usize) -> u32 {
        encode_r(0, rs2, rs1, 0b000, rd, 0b0110011)
    }

    fn sub(rd: usize, rs1: usize, rs2: usize) -> u32 {
        encode_r(0b0100000, rs2, rs1, 0b000, rd, 0b0110011)
    }

    fn sll(rd: usize, rs1: usize, rs2: usize) -> u32 {
        encode_r(0, rs2, rs1, 0b001, rd, 0b0110011)
    }

    fn slt(rd: usize, rs1: usize, rs2: usize) -> u32 {
        encode_r(0, rs2, rs1, 0b010, rd, 0b0110011)
    }

    fn sltu(rd: usize, rs1: usize, rs2: usize) -> u32 {
        encode_r(0, rs2, rs1, 0b011, rd, 0b0110011)
    }

    fn xor(rd: usize, rs1: usize, rs2: usize) -> u32 {
        encode_r(0, rs2, rs1, 0b100, rd, 0b0110011)
    }

    fn srl(rd: usize, rs1: usize, rs2: usize) -> u32 {
        encode_r(0, rs2, rs1, 0b101, rd, 0b0110011)
    }

    fn sra(rd: usize, rs1: usize, rs2: usize) -> u32 {
        encode_r(0b0100000, rs2, rs1, 0b101, rd, 0b0110011)
    }

    fn or(rd: usize, rs1: usize, rs2: usize) -> u32 {
        encode_r(0, rs2, rs1, 0b110, rd, 0b0110011)
    }

    fn and(rd: usize, rs1: usize, rs2: usize) -> u32 {
        encode_r(0, rs2, rs1, 0b111, rd, 0b0110011)
    }

    fn lui(rd: usize, imm: u32) -> u32 {
        encode_u(imm, rd, 0b0110111)
    }

    fn auipc(rd: usize, imm: u32) -> u32 {
        encode_u(imm, rd, 0b0010111)
    }

    fn jal(rd: usize, imm: i32) -> u32 {
        encode_j(imm, rd)
    }

    fn jalr(rd: usize, rs1: usize, imm: i32) -> u32 {
        encode_i(imm, rs1, 0b000, rd, 0b1100111)
    }

    fn beq(rs1: usize, rs2: usize, imm: i32) -> u32 {
        encode_b(imm, rs2, rs1, 0b000)
    }

    fn bne(rs1: usize, rs2: usize, imm: i32) -> u32 {
        encode_b(imm, rs2, rs1, 0b001)
    }

    fn blt(rs1: usize, rs2: usize, imm: i32) -> u32 {
        encode_b(imm, rs2, rs1, 0b100)
    }

    fn bge(rs1: usize, rs2: usize, imm: i32) -> u32 {
        encode_b(imm, rs2, rs1, 0b101)
    }

    fn bltu(rs1: usize, rs2: usize, imm: i32) -> u32 {
        encode_b(imm, rs2, rs1, 0b110)
    }

    fn bgeu(rs1: usize, rs2: usize, imm: i32) -> u32 {
        encode_b(imm, rs2, rs1, 0b111)
    }

    fn lb(rd: usize, rs1: usize, imm: i32) -> u32 {
        encode_i(imm, rs1, 0b000, rd, 0b0000011)
    }

    fn lh(rd: usize, rs1: usize, imm: i32) -> u32 {
        encode_i(imm, rs1, 0b001, rd, 0b0000011)
    }

    fn lw(rd: usize, rs1: usize, imm: i32) -> u32 {
        encode_i(imm, rs1, 0b010, rd, 0b0000011)
    }

    fn lbu(rd: usize, rs1: usize, imm: i32) -> u32 {
        encode_i(imm, rs1, 0b100, rd, 0b0000011)
    }

    fn lhu(rd: usize, rs1: usize, imm: i32) -> u32 {
        encode_i(imm, rs1, 0b101, rd, 0b0000011)
    }

    fn sb(rs2: usize, rs1: usize, imm: i32) -> u32 {
        encode_s(imm, rs2, rs1, 0b000)
    }

    fn sh(rs2: usize, rs1: usize, imm: i32) -> u32 {
        encode_s(imm, rs2, rs1, 0b001)
    }

    fn sw(rs2: usize, rs1: usize, imm: i32) -> u32 {
        encode_s(imm, rs2, rs1, 0b010)
    }

    fn fence() -> u32 {
        0b0001111
    }

    fn illegal() -> u32 {
        0
    }

    fn to_core_state(state: &RefState) -> CoreState {
        let mut regs = [b32(0); 32];
        for (dst, src) in regs.iter_mut().zip(state.regs.iter()) {
            *dst = b(*src);
        }
        CoreState {
            pc: b(state.pc),
            regs,
            trap: state.trap,
        }
    }

    fn apply_store(mem: &mut [u8], req: MemReq) {
        if !req.valid || !req.is_write {
            return;
        }
        let addr = raw(req.addr) as usize;
        let data = raw(req.wdata);
        let wstrb = req.wstrb.raw() as u8;
        for lane in 0..4 {
            if ((wstrb >> lane) & 1) != 0 {
                mem[addr + lane] = ((data >> (lane * 8)) & 0xff) as u8;
            }
        }
    }

    fn load_word(mem: &[u8], addr: u32) -> u32 {
        let addr = (addr & !3) as usize;
        (mem[addr] as u32)
            | ((mem[addr + 1] as u32) << 8)
            | ((mem[addr + 2] as u32) << 16)
            | ((mem[addr + 3] as u32) << 24)
    }

    fn step_dut(state: &RefState, inst: u32, mem: &mut [u8]) -> StepOutput {
        let probe = rv32i_step(StepInput {
            state: to_core_state(state),
            inst: b(inst),
            dmem_rdata: b32(0),
        });
        let rdata = if probe.dmem_req.valid && !probe.dmem_req.is_write {
            load_word(mem, raw(probe.dmem_req.addr))
        } else {
            0
        };
        let out = rv32i_step(StepInput {
            state: to_core_state(state),
            inst: b(inst),
            dmem_rdata: b(rdata),
        });
        apply_store(mem, out.dmem_req);
        out
    }

    fn ref_load(mem: &[u8], addr: u32, funct3: u32) -> Result<u32, ()> {
        let addr = addr as usize;
        match funct3 {
            0b000 => Ok((mem[addr] as i8 as i32) as u32),
            0b001 => {
                if addr & 1 != 0 {
                    Err(())
                } else {
                    let value = (mem[addr] as u16) | ((mem[addr + 1] as u16) << 8);
                    Ok((value as i16 as i32) as u32)
                }
            }
            0b010 => {
                if addr & 3 != 0 {
                    Err(())
                } else {
                    Ok(load_word(mem, addr as u32))
                }
            }
            0b100 => Ok(mem[addr] as u32),
            0b101 => {
                if addr & 1 != 0 {
                    Err(())
                } else {
                    Ok((mem[addr] as u32) | ((mem[addr + 1] as u32) << 8))
                }
            }
            _ => Err(()),
        }
    }

    fn ref_store(mem: &mut [u8], addr: u32, value: u32, funct3: u32) -> Result<(), ()> {
        let addr = addr as usize;
        match funct3 {
            0b000 => {
                mem[addr] = (value & 0xff) as u8;
                Ok(())
            }
            0b001 => {
                if addr & 1 != 0 {
                    Err(())
                } else {
                    mem[addr] = (value & 0xff) as u8;
                    mem[addr + 1] = ((value >> 8) & 0xff) as u8;
                    Ok(())
                }
            }
            0b010 => {
                if addr & 3 != 0 {
                    Err(())
                } else {
                    for lane in 0..4 {
                        mem[addr + lane] = ((value >> (lane * 8)) & 0xff) as u8;
                    }
                    Ok(())
                }
            }
            _ => Err(()),
        }
    }

    fn step_ref(state: &mut RefState, inst: u32, mem: &mut [u8]) {
        if state.trap {
            return;
        }
        let opcode = inst & 0x7f;
        let rd = ((inst >> 7) & 0x1f) as usize;
        let funct3 = (inst >> 12) & 0x7;
        let rs1 = ((inst >> 15) & 0x1f) as usize;
        let rs2 = ((inst >> 20) & 0x1f) as usize;
        let funct7 = (inst >> 25) & 0x7f;
        let pc = state.pc;
        let mut next_pc = pc.wrapping_add(4);
        let mut rd_write = None;
        let mut trap = false;

        match opcode {
            0b0110111 => rd_write = Some(inst & 0xffff_f000),
            0b0010111 => rd_write = Some(pc.wrapping_add(inst & 0xffff_f000)),
            0b1101111 => {
                rd_write = Some(pc.wrapping_add(4));
                let imm = (((inst >> 31) & 1) << 20)
                    | (((inst >> 12) & 0xff) << 12)
                    | (((inst >> 20) & 1) << 11)
                    | (((inst >> 21) & 0x3ff) << 1);
                next_pc = pc.wrapping_add(sign_extend(imm, 21));
            }
            0b1100111 => {
                if funct3 == 0 {
                    rd_write = Some(pc.wrapping_add(4));
                    next_pc = state.regs[rs1].wrapping_add(sign_extend(inst >> 20, 12)) & !1;
                } else {
                    trap = true;
                }
            }
            0b1100011 => {
                let imm = (((inst >> 31) & 1) << 12)
                    | (((inst >> 7) & 1) << 11)
                    | (((inst >> 25) & 0x3f) << 5)
                    | (((inst >> 8) & 0xf) << 1);
                let lhs = state.regs[rs1];
                let rhs = state.regs[rs2];
                let taken = match funct3 {
                    0b000 => lhs == rhs,
                    0b001 => lhs != rhs,
                    0b100 => (lhs as i32) < (rhs as i32),
                    0b101 => (lhs as i32) >= (rhs as i32),
                    0b110 => lhs < rhs,
                    0b111 => lhs >= rhs,
                    _ => {
                        trap = true;
                        false
                    }
                };
                if taken {
                    next_pc = pc.wrapping_add(sign_extend(imm, 13));
                }
            }
            0b0000011 => {
                let addr = state.regs[rs1].wrapping_add(sign_extend(inst >> 20, 12));
                match ref_load(mem, addr, funct3) {
                    Ok(value) => rd_write = Some(value),
                    Err(()) => trap = true,
                }
            }
            0b0100011 => {
                let imm = ((inst >> 7) & 0x1f) | (((inst >> 25) & 0x7f) << 5);
                let addr = state.regs[rs1].wrapping_add(sign_extend(imm, 12));
                if ref_store(mem, addr, state.regs[rs2], funct3).is_err() {
                    trap = true;
                }
            }
            0b0010011 => {
                let imm = sign_extend(inst >> 20, 12);
                let lhs = state.regs[rs1];
                let shamt = (inst >> 20) & 0x1f;
                let value = match funct3 {
                    0b000 => lhs.wrapping_add(imm),
                    0b010 => u32::from((lhs as i32) < (imm as i32)),
                    0b011 => u32::from(lhs < imm),
                    0b100 => lhs ^ imm,
                    0b110 => lhs | imm,
                    0b111 => lhs & imm,
                    0b001 if funct7 == 0 => lhs.wrapping_shl(shamt),
                    0b101 if funct7 == 0 => lhs.wrapping_shr(shamt),
                    0b101 if funct7 == 0b0100000 => ((lhs as i32) >> shamt) as u32,
                    _ => {
                        trap = true;
                        0
                    }
                };
                if !trap {
                    rd_write = Some(value);
                }
            }
            0b0110011 => {
                let lhs = state.regs[rs1];
                let rhs = state.regs[rs2];
                let shamt = rhs & 0x1f;
                let value = match (funct3, funct7) {
                    (0b000, 0) => lhs.wrapping_add(rhs),
                    (0b000, 0b0100000) => lhs.wrapping_sub(rhs),
                    (0b001, 0) => lhs.wrapping_shl(shamt),
                    (0b010, 0) => u32::from((lhs as i32) < (rhs as i32)),
                    (0b011, 0) => u32::from(lhs < rhs),
                    (0b100, 0) => lhs ^ rhs,
                    (0b101, 0) => lhs.wrapping_shr(shamt),
                    (0b101, 0b0100000) => ((lhs as i32) >> shamt) as u32,
                    (0b110, 0) => lhs | rhs,
                    (0b111, 0) => lhs & rhs,
                    _ => {
                        trap = true;
                        0
                    }
                };
                if !trap {
                    rd_write = Some(value);
                }
            }
            0b0001111 => {
                if funct3 != 0 {
                    trap = true;
                }
            }
            0b1110011 => trap = true,
            _ => trap = true,
        }

        if trap {
            state.trap = true;
        } else {
            if let Some(value) = rd_write {
                if rd != 0 {
                    state.regs[rd] = value;
                }
            }
            state.regs[0] = 0;
            state.pc = next_pc;
        }
    }

    fn run_program(program: &[u32], mem: &mut [u8], steps: usize) -> (RefState, RefState) {
        let mut dut_state = RefState::default();
        let mut ref_state = RefState::default();
        let mut dut_mem = mem.to_vec();
        let mut ref_mem = mem.to_vec();

        for _ in 0..steps {
            let pc_index = (dut_state.pc / 4) as usize;
            let inst = program[pc_index];
            let out = step_dut(&dut_state, inst, &mut dut_mem);
            step_ref(&mut ref_state, inst, &mut ref_mem);

            dut_state.pc = raw(out.state.pc);
            dut_state.trap = out.state.trap;
            for idx in 0..32 {
                dut_state.regs[idx] = raw(out.state.regs[idx]);
            }

            assert_eq!(dut_state, ref_state);
            assert_eq!(dut_mem, ref_mem);

            if dut_state.trap {
                break;
            }
        }

        (dut_state, ref_state)
    }

    #[test]
    fn alu_and_immediates_work() {
        assert_eq!(
            raw(alu(AluIn {
                op: AluOp::Add,
                lhs: b(1),
                rhs: b(2),
            })),
            3
        );
        assert_eq!(
            raw(alu(AluIn {
                op: AluOp::Sub,
                lhs: b(1),
                rhs: b(2),
            })),
            0xffff_ffff
        );
        assert_eq!(raw(imm_i(b(addi(1, 2, -8)))), 0xffff_fff8);
        assert_eq!(raw(imm_s(b(sw(3, 4, -12)))), 0xffff_fff4);
        assert_eq!(raw(imm_b(b(beq(1, 2, -16)))), 0xffff_fff0);
        assert_eq!(raw(imm_j(b(jal(1, -2048)))), 0xffff_f800);
    }

    #[test]
    fn program_matches_reference_model() {
        let program = [
            lui(1, 0x0001_0000),
            addi(1, 1, 0x20),
            addi(2, 0, -5),
            addi(3, 0, 17),
            add(4, 2, 3),
            sub(5, 3, 2),
            slli(6, 3, 2),
            srli(7, 6, 1),
            srai(8, 2, 1),
            slt(9, 2, 3),
            sltu(10, 2, 3),
            xor(11, 2, 3),
            or(12, 2, 3),
            and(13, 2, 3),
            slti(14, 2, 0),
            sltiu(15, 2, 0),
            xori(16, 3, 0x55),
            ori(17, 3, 0x80),
            andi(18, 17, 0xff),
            sll(19, 3, 14),
            srl(20, 19, 14),
            sra(21, 2, 14),
            sw(4, 1, 0),
            sb(2, 1, 4),
            sh(3, 1, 6),
            lw(22, 1, 0),
            lb(23, 1, 4),
            lbu(24, 1, 4),
            lh(25, 1, 6),
            lhu(26, 1, 6),
            beq(4, 22, 8),
            addi(27, 0, 111),
            bne(23, 24, 8),
            addi(28, 0, 222),
            blt(2, 3, 8),
            addi(29, 0, 123),
            bge(3, 2, 8),
            addi(30, 0, 124),
            bltu(3, 2, 8),
            addi(31, 0, 125),
            bgeu(2, 3, 8),
            addi(5, 0, 126),
            auipc(6, 0x0000_1000),
            jal(7, 8),
            addi(8, 0, 127),
            addi(9, 0, 9),
            fence(),
            illegal(),
        ];
        let mut mem = vec![0_u8; 0x20_000];
        let (dut, reference) = run_program(&program, &mut mem, program.len());
        assert_eq!(dut, reference);
        assert!(dut.trap);
        assert_eq!(dut.regs[0], 0);
        assert_eq!(dut.regs[27], 0);
        assert_eq!(dut.regs[28], 0);
        assert_eq!(dut.regs[29], 0);
        assert_eq!(dut.regs[30], 0);
        assert_eq!(dut.regs[31], 0);
        assert_eq!(dut.regs[5], 22);
        assert_eq!(dut.regs[9], 9);
    }

    #[test]
    fn jalr_and_misaligned_accesses_trap() {
        let mut mem = vec![0_u8; 256];
        let program = [addi(1, 0, 12), jalr(2, 1, 0), illegal(), addi(3, 0, 33)];
        let (dut, reference) = run_program(&program, &mut mem, 3);
        assert_eq!(dut, reference);
        assert_eq!(dut.pc, 16);
        assert_eq!(dut.regs[2], 8);
        assert_eq!(dut.regs[3], 33);

        let program = [addi(1, 0, 3), lw(2, 1, 0)];
        let (dut, reference) = run_program(&program, &mut mem, 2);
        assert_eq!(dut, reference);
        assert!(dut.trap);
        assert_eq!(dut.pc, 4);
    }

    #[test]
    fn step_kernel_compiles_in_rhdl() {
        let core = Rv32iStep::default();
        let descriptor = core.descriptor("rv32i_step".into()).unwrap();
        let hdl = descriptor.hdl().unwrap();
        let modules = hdl.modules.to_string();
        assert!(modules.contains("rv32i_step_decoder"));
        assert!(modules.contains("rv32i_step_op_imm_alu"));
    }
}
