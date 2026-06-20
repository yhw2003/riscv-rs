use rhdl::prelude::*;

use crate::units::alu::alu;
use crate::units::decoder::split_inst;
use crate::units::{
    self, BranchInput, BranchUnit, Decoder, ExecCandidates, ExecResult, FenceInput, FenceUnit,
    ImmediateGenerator, JumpInput, JumpUnit, LoadInput, LoadUnit, OpImmInput, OpImmUnit, OpInput,
    OpUnit, RegRead, RegReadInput, StoreInput, StoreUnit, UpperInput, UpperUnit,
};
use crate::{
    CommitTrace, CoreState, InstFields, OPCODE_AUIPC, OPCODE_BRANCH, OPCODE_JAL, OPCODE_JALR,
    OPCODE_LOAD, OPCODE_LUI, OPCODE_MISC_MEM, OPCODE_OP, OPCODE_OP_IMM, OPCODE_STORE,
    OPCODE_SYSTEM, RegBus, RegReadReq, RegWriteReq, StepInput, StepOutput,
};

#[derive(Clone, Debug, Default, Synchronous, SynchronousDQ)]
#[rhdl(dq_no_prefix)]
pub struct Rv32iStep {
    decoder: Decoder,
    immediates: ImmediateGenerator,
    reg_read: RegRead,
    upper: UpperUnit,
    jump: JumpUnit,
    branch: BranchUnit,
    load: LoadUnit,
    store: StoreUnit,
    op_imm: OpImmUnit,
    op: OpUnit,
    fence: FenceUnit,
}

impl SynchronousIO for Rv32iStep {
    type I = StepInput;
    type O = StepOutput;
    type Kernel = rv32i_step_module;
}

#[kernel]
pub fn rv32i_step_module(_cr: ClockReset, input: StepInput, q: Q) -> (StepOutput, D) {
    let fields = q.decoder;
    let imms = q.immediates;
    let regs = q.reg_read;
    let pc = input.state.pc;
    let pc_plus_4 = pc + b32(4);

    let mut d = D::dont_care();
    d.decoder = input.inst;
    d.immediates = input.inst;
    d.reg_read = RegReadInput {
        fields,
        rdata: input.reg_rdata,
    };
    d.upper = UpperInput {
        opcode: fields.opcode,
        pc,
        pc_plus_4,
        imm_u: imms.u,
    };
    d.jump = JumpInput {
        opcode: fields.opcode,
        funct3: fields.funct3,
        pc,
        pc_plus_4,
        rs1: regs.rs1,
        imm_i: imms.i,
        imm_j: imms.j,
    };
    d.branch = BranchInput {
        funct3: fields.funct3,
        pc,
        pc_plus_4,
        rs1: regs.rs1,
        rs2: regs.rs2,
        imm_b: imms.b,
    };
    d.load = LoadInput {
        funct3: fields.funct3,
        pc_plus_4,
        rs1: regs.rs1,
        imm_i: imms.i,
        dmem_rdata: input.dmem_rdata,
    };
    d.store = StoreInput {
        funct3: fields.funct3,
        pc_plus_4,
        rs1: regs.rs1,
        rs2: regs.rs2,
        imm_s: imms.s,
    };
    d.op_imm = OpImmInput {
        funct3: fields.funct3,
        funct7: fields.funct7,
        pc_plus_4,
        rs1: regs.rs1,
        imm_i: imms.i,
    };
    d.op = OpInput {
        funct3: fields.funct3,
        funct7: fields.funct7,
        pc_plus_4,
        rs1: regs.rs1,
        rs2: regs.rs2,
    };
    d.fence = FenceInput {
        funct3: fields.funct3,
        pc_plus_4,
    };

    let result = select_exec_result(
        fields,
        pc_plus_4,
        ExecCandidates {
            upper: q.upper,
            jump: q.jump,
            branch: q.branch,
            load: q.load,
            store: q.store,
            op_imm: q.op_imm,
            op: q.op,
            fence: q.fence,
        },
    );
    (finish_step(input, fields, result), d)
}

#[kernel]
pub fn select_exec_result(
    fields: InstFields,
    pc_plus_4: b32,
    candidates: ExecCandidates,
) -> ExecResult {
    let mut out = ExecResult::default();
    out.next_pc = pc_plus_4;
    if fields.opcode == OPCODE_LUI || fields.opcode == OPCODE_AUIPC {
        out = candidates.upper;
    } else if fields.opcode == OPCODE_JAL || fields.opcode == OPCODE_JALR {
        out = candidates.jump;
    } else if fields.opcode == OPCODE_BRANCH {
        out = candidates.branch;
    } else if fields.opcode == OPCODE_LOAD {
        out = candidates.load;
    } else if fields.opcode == OPCODE_STORE {
        out = candidates.store;
    } else if fields.opcode == OPCODE_OP_IMM {
        out = candidates.op_imm;
    } else if fields.opcode == OPCODE_OP {
        out = candidates.op;
    } else if fields.opcode == OPCODE_MISC_MEM {
        out = candidates.fence;
    } else if fields.opcode == OPCODE_SYSTEM {
        out.illegal = true;
    } else {
        out.illegal = true;
    }
    out
}

#[kernel]
pub fn finish_step(input: StepInput, fields: InstFields, result: ExecResult) -> StepOutput {
    let pc = input.state.pc;
    let rd_write = result.rd_write && fields.rd != b5(0) && !result.illegal && !input.state.trap;

    let trapped = input.state.trap || result.illegal;
    let state_next = CoreState {
        pc: if trapped { pc } else { result.next_pc },
        trap: trapped,
    };
    let trace = CommitTrace {
        valid: !input.state.trap,
        pc,
        inst: input.inst,
        rd: fields.rd,
        rd_write,
        rd_wdata: result.rd_wdata,
        next_pc: state_next.pc,
        trap: trapped,
    };

    StepOutput {
        state: state_next,
        reg_bus: RegBus {
            read: RegReadReq {
                rs1_addr: fields.rs1,
                rs2_addr: fields.rs2,
            },
            write: RegWriteReq {
                valid: rd_write,
                rd: fields.rd,
                data: result.rd_wdata,
            },
        },
        imem_addr: pc,
        dmem_req: result.dmem_req,
        trace,
    }
}

pub fn rv32i_step(input: StepInput) -> StepOutput {
    let fields = split_inst(input.inst);
    let imms = units::immediate::immediates(input.inst);
    let regs = units::reg_read::read_regs(RegReadInput {
        fields,
        rdata: input.reg_rdata,
    });
    let pc = input.state.pc;
    let pc_plus_4 = pc + b32(4);
    let candidates = ExecCandidates {
        upper: units::upper::upper_exec(UpperInput {
            opcode: fields.opcode,
            pc,
            pc_plus_4,
            imm_u: imms.u,
        }),
        jump: units::jump::jump_exec(JumpInput {
            opcode: fields.opcode,
            funct3: fields.funct3,
            pc,
            pc_plus_4,
            rs1: regs.rs1,
            imm_i: imms.i,
            imm_j: imms.j,
        }),
        branch: units::branch::branch_exec(BranchInput {
            funct3: fields.funct3,
            pc,
            pc_plus_4,
            rs1: regs.rs1,
            rs2: regs.rs2,
            imm_b: imms.b,
        }),
        load: units::load::load_exec(LoadInput {
            funct3: fields.funct3,
            pc_plus_4,
            rs1: regs.rs1,
            imm_i: imms.i,
            dmem_rdata: input.dmem_rdata,
        }),
        store: units::store::store_exec(StoreInput {
            funct3: fields.funct3,
            pc_plus_4,
            rs1: regs.rs1,
            rs2: regs.rs2,
            imm_s: imms.s,
        }),
        op_imm: units::op_imm::op_imm_finish(
            OpImmInput {
                funct3: fields.funct3,
                funct7: fields.funct7,
                pc_plus_4,
                rs1: regs.rs1,
                imm_i: imms.i,
            },
            units::op_imm::op_imm_select(OpImmInput {
                funct3: fields.funct3,
                funct7: fields.funct7,
                pc_plus_4,
                rs1: regs.rs1,
                imm_i: imms.i,
            }),
            alu(units::op_imm::op_imm_select(OpImmInput {
                funct3: fields.funct3,
                funct7: fields.funct7,
                pc_plus_4,
                rs1: regs.rs1,
                imm_i: imms.i,
            })
            .alu),
        ),
        op: units::op_reg::op_finish(
            OpInput {
                funct3: fields.funct3,
                funct7: fields.funct7,
                pc_plus_4,
                rs1: regs.rs1,
                rs2: regs.rs2,
            },
            units::op_reg::op_select(OpInput {
                funct3: fields.funct3,
                funct7: fields.funct7,
                pc_plus_4,
                rs1: regs.rs1,
                rs2: regs.rs2,
            }),
            alu(units::op_reg::op_select(OpInput {
                funct3: fields.funct3,
                funct7: fields.funct7,
                pc_plus_4,
                rs1: regs.rs1,
                rs2: regs.rs2,
            })
            .alu),
        ),
        fence: units::fence::fence_exec(FenceInput {
            funct3: fields.funct3,
            pc_plus_4,
        }),
    };
    finish_step(
        input,
        fields,
        select_exec_result(fields, pc_plus_4, candidates),
    )
}
