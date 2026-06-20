use rhdl::prelude::*;

use crate::MemReq;

use super::{AluIn, AluOp, AluSync, ExecResult};

#[derive(Copy, Clone, Debug, Default, PartialEq, Digital)]
pub struct OpInput {
    pub funct3: b3,
    pub funct7: b7,
    pub pc_plus_4: b32,
    pub rs1: b32,
    pub rs2: b32,
}

#[derive(Copy, Clone, Debug, Default, PartialEq, Digital)]
pub struct AluSelect {
    pub valid: bool,
    pub alu: AluIn,
}

#[kernel]
pub fn op_select(input: OpInput) -> AluSelect {
    let mut valid = true;
    let mut op = AluOp::Add;
    if input.funct3 == b3(0b000) {
        if input.funct7 == b7(0) {
            op = AluOp::Add;
        } else if input.funct7 == b7(0b0100000) {
            op = AluOp::Sub;
        } else {
            valid = false;
        }
    } else if input.funct3 == b3(0b001) {
        if input.funct7 == b7(0) {
            op = AluOp::Sll;
        } else {
            valid = false;
        }
    } else if input.funct3 == b3(0b010) {
        if input.funct7 == b7(0) {
            op = AluOp::Slt;
        } else {
            valid = false;
        }
    } else if input.funct3 == b3(0b011) {
        if input.funct7 == b7(0) {
            op = AluOp::Sltu;
        } else {
            valid = false;
        }
    } else if input.funct3 == b3(0b100) {
        if input.funct7 == b7(0) {
            op = AluOp::Xor;
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
    } else if input.funct3 == b3(0b110) {
        if input.funct7 == b7(0) {
            op = AluOp::Or;
        } else {
            valid = false;
        }
    } else if input.funct3 == b3(0b111) {
        if input.funct7 == b7(0) {
            op = AluOp::And;
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
            rhs: input.rs2,
        },
    }
}

#[kernel]
pub fn op_finish(input: OpInput, selected: AluSelect, alu_out: b32) -> ExecResult {
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
pub struct OpUnit {
    alu: AluSync,
}

impl SynchronousIO for OpUnit {
    type I = OpInput;
    type O = ExecResult;
    type Kernel = op_sync;
}

#[kernel]
pub fn op_sync(_cr: ClockReset, input: OpInput, q: Q) -> (ExecResult, D) {
    let selected = op_select(input);
    let mut d = D::dont_care();
    d.alu = selected.alu;
    (op_finish(input, selected, q.alu), d)
}
