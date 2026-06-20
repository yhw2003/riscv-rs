pub mod alu;
pub mod branch;
pub mod decoder;
pub mod fence;
pub mod immediate;
pub mod jump;
pub mod load;
pub mod op_imm;
pub mod op_reg;
pub mod reg_read;
pub mod store;
pub mod upper;

pub use alu::{AluIn, AluOp, AluSync};
pub use branch::{BranchInput, BranchUnit, branch_taken};
pub use decoder::{Decoder, split_inst};
pub use fence::{FenceInput, FenceUnit};
pub use immediate::{ImmediateGenerator, ImmediateValues, imm_b, imm_i, imm_j, imm_s, imm_u};
pub use jump::{JumpInput, JumpUnit};
pub use load::{LoadInput, LoadKind, LoadUnit, is_load_misaligned, load_value};
pub use op_imm::{OpImmInput, OpImmUnit};
pub use op_reg::{OpInput, OpUnit};
pub use reg_read::{RegRead, RegReadInput, RegReadOutput};
pub use store::{StoreInput, StoreKind, StoreUnit, is_store_misaligned, store_req};
pub use upper::{UpperInput, UpperUnit};

use rhdl::prelude::*;

use crate::MemReq;

#[derive(Copy, Clone, Debug, Default, PartialEq, Digital)]
pub struct ExecResult {
    pub next_pc: b32,
    pub rd_write: bool,
    pub rd_wdata: b32,
    pub illegal: bool,
    pub dmem_req: MemReq,
}

#[derive(Copy, Clone, Debug, Default, PartialEq, Digital)]
pub struct ExecCandidates {
    pub upper: ExecResult,
    pub jump: ExecResult,
    pub branch: ExecResult,
    pub load: ExecResult,
    pub store: ExecResult,
    pub op_imm: ExecResult,
    pub op: ExecResult,
    pub fence: ExecResult,
}
