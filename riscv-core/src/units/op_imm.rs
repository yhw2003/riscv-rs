use rhdl::prelude::*;

use crate::{AluIn, AluOp, MemReq};

use super::{AluSync, ExecResult};

#[derive(Copy, Clone, Debug, Default, PartialEq, Digital)]
pub struct OpImmInput {
    pub funct3: b3,
    pub funct7: b7,
    pub pc_plus_4: b32,
    pub rs1: b32,
    pub imm_i: b32,
}

#[derive(Copy, Clone, Debug, Default, PartialEq, Digital)]
pub struct AluSelect {
    pub valid: bool,
    pub alu: AluIn,
}

#[kernel]
pub fn op_imm_select(input: OpImmInput) -> AluSelect {
    let mut valid = true;
    let mut op = AluOp::Add;
    if input.funct3 == b3(0b000) {
        op = AluOp::Add;
    } else if input.funct3 == b3(0b010) {
        op = AluOp::Slt;
    } else if input.funct3 == b3(0b011) {
        op = AluOp::Sltu;
    } else if input.funct3 == b3(0b100) {
        op = AluOp::Xor;
    } else if input.funct3 == b3(0b110) {
        op = AluOp::Or;
    } else if input.funct3 == b3(0b111) {
        op = AluOp::And;
    } else if input.funct3 == b3(0b001) {
        if input.funct7 == b7(0) {
            op = AluOp::Sll;
        } else {
            valid = false;
        }
    } else if input.funct3 == b3(0b101) {
        if input.funct7 == b7(0) {
            op = AluOp::Srl;
        } else if input.funct7 == b7(0b0100000) {
            op = AluOp::Sra;
        } else {
            valid = false;
        }
    } else {
        valid = false;
    }
    AluSelect {
        valid,
        alu: AluIn {
            op,
            lhs: input.rs1,
            rhs: input.imm_i,
        },
    }
}

#[kernel]
pub fn op_imm_finish(input: OpImmInput, selected: AluSelect, alu_out: b32) -> ExecResult {
    ExecResult {
        next_pc: input.pc_plus_4,
        rd_write: selected.valid,
        rd_wdata: alu_out,
        illegal: !selected.valid,
        dmem_req: MemReq::default(),
    }
}

#[derive(Clone, Debug, Default, Synchronous, SynchronousDQ)]
#[rhdl(dq_no_prefix)]
pub struct OpImmUnit {
    alu: AluSync,
}

impl SynchronousIO for OpImmUnit {
    type I = OpImmInput;
    type O = ExecResult;
    type Kernel = op_imm_sync;
}

#[kernel]
pub fn op_imm_sync(_cr: ClockReset, input: OpImmInput, q: Q) -> (ExecResult, D) {
    let selected = op_imm_select(input);
    let mut d = D::dont_care();
    d.alu = selected.alu;
    (op_imm_finish(input, selected, q.alu), d)
}
