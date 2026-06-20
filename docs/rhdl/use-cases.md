# RISC-V 项目用例

本项目目标是用 Rust 实现可在 FPGA 上运行的简易 RISC-V 核心。RHDL 可以按“先组合逻辑、再状态、再顶层集成”的节奏引入。

## 1. 指令字段抽取

RISC-V 指令是 32 位。可以先用 `b32` 表示原始指令，再用移位和 `resize` 抽字段。

```rust
use rhdl::prelude::*;

#[derive(Digital, Timed, PartialEq, Copy, Clone, Default)]
pub struct InstFields {
    pub opcode: b7,
    pub rd: b5,
    pub funct3: b3,
    pub rs1: b5,
    pub rs2: b5,
    pub funct7: b7,
}

#[kernel]
pub fn split_inst(i: Signal<b32, Red>, _q: ()) -> (Signal<InstFields, Red>, ()) {
    let inst = i.val();
    let fields = InstFields {
        opcode: inst.resize(),
        rd: (inst >> 7).resize(),
        funct3: (inst >> 12).resize(),
        rs1: (inst >> 15).resize(),
        rs2: (inst >> 20).resize(),
        funct7: (inst >> 25).resize(),
    };
    (signal(fields), ())
}
```

如果后续引入 `rhdl-std`，也可以用 `slice::<32, 5>(inst, 7)` 这类 helper 表达字段切片。

## 2. 译码输出建模

译码输出建议用 `Digital` enum/struct，而不是散落的 magic number。

```rust
#[derive(Digital, PartialEq, Copy, Clone, Default)]
pub enum AluOp {
    #[default]
    Add,
    Sub,
    And,
    Or,
    Xor,
    Sll,
    Srl,
    Sra,
}

#[derive(Digital, Timed, PartialEq, Copy, Clone, Default)]
pub struct DecodeOut {
    pub alu_op: AluOp,
    pub rd: b5,
    pub rs1: b5,
    pub rs2: b5,
    pub writes_rd: bool,
}
```

这能让后面的 ALU、寄存器写回、分支控制都吃同一组强类型信号。

## 3. ALU

ALU 是最适合先落地的模块，因为它大多是纯组合逻辑，可以直接 kernel 单测。

```rust
#[derive(Digital, Timed, PartialEq, Copy, Clone, Default)]
pub struct AluIn {
    pub op: AluOp,
    pub lhs: b32,
    pub rhs: b32,
}

#[kernel]
pub fn alu(i: Signal<AluIn, Red>, _q: ()) -> (Signal<b32, Red>, ()) {
    let i = i.val();
    let y = match i.op {
        AluOp::Add => i.lhs + i.rhs,
        AluOp::Sub => i.lhs - i.rhs,
        AluOp::And => i.lhs & i.rhs,
        AluOp::Or => i.lhs | i.rhs,
        AluOp::Xor => i.lhs ^ i.rhs,
        AluOp::Sll => i.lhs << i.rhs.resize::<5>(),
        AluOp::Srl => i.lhs >> i.rhs.resize::<5>(),
        AluOp::Sra => (i.lhs.as_signed() >> i.rhs.resize::<5>()).as_unsigned(),
    };
    (signal(y), ())
}
```

先用普通 Rust 单测覆盖 corner cases，例如溢出、移位大于 31、算术右移符号扩展。

## 4. 控制路径与状态机

多周期 CPU 或流水线控制可以用 enum 表达状态：

```rust
#[derive(Digital, PartialEq, Copy, Clone, Default)]
pub enum CoreState {
    #[default]
    Fetch,
    Decode,
    Execute,
    Memory,
    WriteBack,
}
```

状态寄存器属于同步电路。当前项目只依赖 `rhdl`，可以先把状态转移写成纯 kernel 进行验证；需要真正寄存时，可以引入 `rhdl-fpga::core::dff::DFF` 或在本项目实现等价 DFF。

## 5. 寄存器堆和存储器

寄存器堆可以分两步做：

1. 先用普通 Rust 数组模型做参考实现，用测试生成期望值。
2. 再用 RHDL 同步 RAM/DFF 结构实现硬件版本。

`rhdl-fpga::core::ram` 里已有同步/异步 RAM core，可以作为后续参考。当前项目未依赖 `rhdl-fpga` 时，不要在主代码里直接引用。

## 6. 总线和外设接口

如果目标是简单 FPGA demo，可以先定义自己的极简 memory bus：

```rust
#[derive(Digital, PartialEq, Copy, Clone, Default)]
pub enum MemOp {
    #[default]
    None,
    Load,
    Store,
}

#[derive(Digital, Timed, PartialEq, Copy, Clone, Default)]
pub struct MemReq {
    pub op: MemOp,
    pub addr: b32,
    pub wdata: b32,
    pub wstrb: b4,
}
```

如果后续需要接标准外设，可参考 `rhdl-fpga::axi4lite` 的类型、channel、endpoint、register 和 stream bridge。

## 7. 验证路线

推荐按模块分层验证：

| 模块 | 首选验证方式 |
| --- | --- |
| 指令字段抽取 | 穷举/样例 kernel 单测。 |
| 立即数生成 | 对照 RISC-V spec 的样例单测。 |
| ALU | kernel 单测加随机输入。 |
| 译码 | 指令样例表驱动测试。 |
| 控制状态机 | 同步仿真，检查每拍状态。 |
| 寄存器堆/RAM | 与软件参考模型对拍。 |
| CPU 顶层 | VCD/SVG 波形加小程序端到端测试。 |

等 Rust 层行为稳定后，再用 `Fixture` 导出顶层 Verilog，把端口整理成 clock/reset、instruction memory、data memory、debug 端口等明确接口。
