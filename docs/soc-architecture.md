# 当前 SoC 实现架构

本文梳理 `riscv-core` 中当前 SoC 的实现方式。代码入口主要在：

- `riscv-core/src/soc.rs`：SoC 顶层 `Rv32iSoc`。
- `riscv-core/src/step.rs`：单条 RV32I 指令的执行与提交。
- `riscv-core/src/units/`：译码、立即数、寄存器读取和各类执行单元。
- `riscv-core/src/gpio.rs`：内存映射 GPIO 外设。
- `riscv-core/src/lib.rs`：共享数据结构、opcode 常量和 Rust 参考测试。

## 总体定位

当前 SoC 是一个用 RHDL 描述的简易 RV32I 单核系统。它包含 CPU 状态寄存器、单步执行逻辑和一个 GPIO 外设，但不内置指令存储器或数据存储器。

外部环境需要根据 `imem_addr` 提供当前指令 `inst`，并根据 `dmem_req` 处理数据访存请求，再把读数据放到 `dmem_rdata`。因此它更像一个可综合 CPU+MMIO 外设顶层，而不是完整片上系统平台。

```text
        +-------------------------------+
        |           Rv32iSoc            |
        |                               |
inst -->|                               |--> imem_addr
rdata ->|                               |--> dmem_req
        |  +----------+   +----------+  |--> gpio_pins
        |  | CoreState|-->| Rv32iStep|--|--> trace
        |  |   DFF    |   +----------+  |--> trap
        |  +----------+         |       |
        |                       v       |
        |                  +----------+ |
        |                  |   Gpio   | |
        |                  +----------+ |
        +-------------------------------+
```

## 顶层接口

`SocInput` 是 SoC 的外部输入：

| 字段 | 含义 |
| --- | --- |
| `inst: b32` | 指令存储器在 `imem_addr` 对应地址返回的 32 位指令。 |
| `dmem_rdata: b32` | 数据存储器对当前 load 请求返回的 32 位读数据。 |

`SocOutput` 是 SoC 的外部输出：

| 字段 | 含义 |
| --- | --- |
| `imem_addr: b32` | 当前取指地址，来自 CPU 当前 `pc`。 |
| `dmem_req: MemReq` | 经过 GPIO 地址过滤后的数据存储器请求。 |
| `gpio_pins: b32` | GPIO 输出寄存器当前值。 |
| `trace: CommitTrace` | 当前指令提交轨迹，用于测试和调试。 |
| `trap: bool` | CPU 是否进入 trap 状态。 |

`MemReq` 是当前唯一的数据访存请求格式：

| 字段 | 含义 |
| --- | --- |
| `valid` | 请求有效。 |
| `is_write` | `true` 表示 store，`false` 表示 load。 |
| `addr` | 4 字节对齐后的数据地址。 |
| `wdata` | 写数据，已按 byte lane 移位。 |
| `wstrb` | 4 bit 字节写使能。 |

当前接口没有 ready/valid 握手、错误响应或 load response valid。load 数据被假定能在同一组合收敛周期内通过 `dmem_rdata` 返回。

## 顶层组成

`Rv32iSoc` 有 3 个子模块：

| 子模块 | 类型 | 作用 |
| --- | --- | --- |
| `state` | `DFF<CoreState>` | 保存 CPU 架构状态，包括 `pc`、32 个通用寄存器和 sticky `trap`。 |
| `step` | `Rv32iStep` | 根据当前状态、指令和数据读值，计算下一状态、访存请求和提交 trace。 |
| `gpio` | `Gpio` | 捕获写到 `GPIO_BASE` 的 store，并把其他访存请求透传给外部数据存储器。 |

`CoreState::default()` 使 PC 为 `0`、寄存器全 `0`、`trap` 为 `false`。GPIO 复位值也是 `0`。

RHDL 的组合方式需要特别注意：父模块通过 `D` 给子模块输入或寄存器下一态，通过 `Q` 读取子模块输出或寄存器当前态。对 `DFF` 来说，`q.state` 是当前 CPU 状态，`d.state` 是下一个时钟沿写入的状态；对组合型子模块来说，`q.step`、`q.gpio` 表示对应输入在组合收敛后的输出。

## 每拍数据流

顶层 `soc_kernel` 的主要流程是：

1. 从 `q.state` 取出当前 `CoreState`。
2. 把当前状态、外部 `inst` 和 `dmem_rdata` 送入 `Rv32iStep`。
3. 读取 `q.step` 得到 `StepOutput`，其中包含下一 CPU 状态、取指地址、原始数据访存请求和提交 trace。
4. 把 `step_out.dmem_req` 送入 `Gpio`。
5. 读取 `q.gpio` 得到 GPIO 处理后的输出：GPIO 命中时更新 `gpio_pins` 并吞掉该访存请求，未命中时透传给外部数据存储器。
6. 把 `step_out.state` 写到 `d.state`，等待下一个时钟沿提交。

简化后的路径如下：

```text
q.state + inst + dmem_rdata
    -> Rv32iStep
    -> step_out.state      -> d.state
    -> step_out.dmem_req   -> Gpio -> SocOutput.dmem_req
    -> step_out.imem_addr  -> SocOutput.imem_addr
    -> step_out.trace      -> SocOutput.trace
```

## `Rv32iStep` 执行结构

`Rv32iStep` 负责“一条指令”的逻辑执行。它不是把所有指令写在一个巨大 `match` 里，而是先并行生成多个执行候选，再按 opcode 选择最终结果：

```text
inst
  -> Decoder            -> InstFields
  -> ImmediateGenerator -> I/S/B/U/J immediates

CoreState.regs + InstFields
  -> RegRead -> rs1/rs2 values

InstFields + immediates + rs values + pc
  -> UpperUnit
  -> JumpUnit
  -> BranchUnit
  -> LoadUnit
  -> StoreUnit
  -> OpImmUnit
  -> OpUnit
  -> FenceUnit
  -> select_exec_result
  -> finish_step
```

### 前端辅助单元

| 单元 | 文件 | 输出 |
| --- | --- | --- |
| `Decoder` | `units/decoder.rs` | `opcode`、`rd`、`funct3`、`rs1`、`rs2`、`funct7`。 |
| `ImmediateGenerator` | `units/immediate.rs` | I/S/B/U/J 五类立即数，已做符号扩展或高位对齐。 |
| `RegRead` | `units/reg_read.rs` | 从 32 个通用寄存器中读取 `rs1` 和 `rs2`。 |

### 执行候选

所有执行单元统一输出 `ExecResult`：

| 字段 | 含义 |
| --- | --- |
| `next_pc` | 指令执行后的下一 PC。 |
| `rd_write` | 是否写回 `rd`。 |
| `rd_wdata` | 写回数据。 |
| `illegal` | 当前候选是否认为指令非法。 |
| `dmem_req` | 数据访存请求。 |

各执行单元覆盖的指令如下：

| 单元 | 支持指令/功能 |
| --- | --- |
| `UpperUnit` | `LUI`、`AUIPC`。 |
| `JumpUnit` | `JAL`、`JALR`。`JALR` 要求 `funct3 == 0`，目标地址 bit0 清零。 |
| `BranchUnit` | `BEQ`、`BNE`、`BLT`、`BGE`、`BLTU`、`BGEU`。 |
| `LoadUnit` | `LB`、`LH`、`LW`、`LBU`、`LHU`，并检查 half/word 对齐。 |
| `StoreUnit` | `SB`、`SH`、`SW`，生成对齐地址、移位后的 `wdata` 和 `wstrb`，并检查 half/word 对齐。 |
| `OpImmUnit` | `ADDI`、`SLTI`、`SLTIU`、`XORI`、`ORI`、`ANDI`、`SLLI`、`SRLI`、`SRAI`。 |
| `OpUnit` | `ADD`、`SUB`、`SLL`、`SLT`、`SLTU`、`XOR`、`SRL`、`SRA`、`OR`、`AND`。 |
| `FenceUnit` | `FENCE`，当前实现为 no-op，仅接受 `funct3 == 0`。 |

`OpImmUnit` 和 `OpUnit` 内部复用 `AluSync`，ALU 支持 add/sub/shift/compare/logic 等 RV32I 基础运算。

### 结果选择与提交

`select_exec_result` 根据 `opcode` 选择对应候选结果：

| opcode 类别 | 候选 |
| --- | --- |
| `LUI`/`AUIPC` | `upper` |
| `JAL`/`JALR` | `jump` |
| `BRANCH` | `branch` |
| `LOAD` | `load` |
| `STORE` | `store` |
| `OP_IMM` | `op_imm` |
| `OP` | `op` |
| `MISC_MEM` | `fence` |
| `SYSTEM` 或未知 opcode | 直接标记 illegal |

`finish_step` 完成架构状态提交：

- 只有在 `rd_write == true`、`rd != x0`、没有 illegal、当前未 trap 时才写回寄存器。
- 每次都会强制 `x0 = 0`。
- `trap` 是 sticky 的：一旦进入 trap，后续保持 trap。
- 进入 trap 时 PC 保持当前值；正常执行时 PC 更新为 `result.next_pc`。
- `CommitTrace` 记录当前 PC、指令、写回信息、下一 PC 和 trap 状态。

## GPIO 和内存映射

GPIO 基地址定义在 `gpio.rs`：

```text
GPIO_BASE = 0x1000_0000
```

`Gpio` 只处理写请求：

- 当 `dmem_req.valid == true`、`is_write == true` 且 `addr == GPIO_BASE` 时命中。
- 命中后用 `wstrb` 按字节更新 `pins`。
- 命中后将 `dmem_req.valid` 清为 `false`，避免外部数据存储器再看到这次 MMIO 写。
- 未命中时原样透传 `dmem_req`。

当前实现没有 GPIO 读回路径。对 `GPIO_BASE` 的 load 会作为普通外部数据存储器访问透传出去。

## 访存模型

当前 load/store 单元采用“单周期外部存储器”模型：

- load 单元先根据 `rs1 + imm_i` 生成地址，并立即用输入 `dmem_rdata` 做 byte/half/word 抽取和符号扩展。
- store 单元根据 `rs1 + imm_s` 生成对齐后的 `addr`、移位后的 `wdata` 和 `wstrb`。
- half/word 的非对齐 load/store 会设置 illegal，从而进入 trap。
- 地址输出按 4 字节对齐：`addr & 0xffff_fffc`。

这意味着外部仿真或硬件 glue 需要让 `dmem_rdata` 与当前 load 请求在同一组合周期内收敛。`verilator/gpio/sim_main.cpp` 里通过多次 settle 来模拟这个行为。

## Verilog 导出和验证

`riscv-core/src/main.rs` 会实例化 `Rv32iSoc`，生成名为 `rv32i_soc` 的 HDL descriptor，并把 Verilog 写到 `output/o.v`：

```text
cargo run -p riscv-core
```

当前验证分两层：

| 验证 | 位置 | 内容 |
| --- | --- | --- |
| Rust 单元测试 | `riscv-core/src/lib.rs` | 用 `rv32i_step` 纯函数路径与软件参考模型对拍，覆盖 ALU、立即数、访存、分支、跳转和 trap。 |
| RHDL 编译检查 | `riscv-core/src/lib.rs` | 检查 `Rv32iStep` 能生成 HDL，并包含预期子模块。 |
| Verilator 端到端测试 | `riscv-core/tests/gpio_verilator.rs` | 生成 SoC Verilog，编译 `verilator/*/*.c` 固件，仿真 GPIO MMIO 写序列。 |

GPIO Verilator 固件向 `0x1000_0000` 依次写入：

```text
0x00000005
0x0000000a
0x0000003c
0x80000081
```

仿真器检查 `gpio_pins` 是否按这个序列变化。

另有 `verilator/control_flow` 示例，固件包含 `for` 循环、`while` 循环、`if/else` 分支以及加减法计算。它向 GPIO 写入：

```text
0x00000008
0x0000000a
0x00000013
0x0000001d
0x00000022
0x0000001e
0x0000001c
0x0000001b
0x8000001c
```

## 当前边界和后续演进点

当前实现已经覆盖 RV32I 的大部分基础整数指令路径，但仍有一些明确边界：

- 不包含指令存储器、数据存储器、ROM/RAM 初始化或标准总线桥。
- 数据访存没有 ready/valid 握手，也没有多周期 load 支持。
- `SYSTEM` 指令全部 trap，没有 CSR、`ECALL`、`EBREAK`、`MRET` 或中断机制。
- `FENCE` 当前是 no-op。
- trap 只有一个 sticky bool，没有 mcause/mtval/mepc 等异常信息。
- GPIO 只有写输出，没有读回、方向控制或多寄存器地址空间。
- 寄存器堆当前作为 `CoreState` 的 32 个 `b32` 一起保存在 DFF 状态中，还不是独立 RAM/register file 宏。
- `JAL`/branch 目标地址没有额外的指令地址对齐异常检查；`JALR` 已按规范清除 bit0。

如果继续扩展，比较自然的顺序是：

1. 把外部 imem/dmem 封装成明确的 SoC memory 子系统或总线接口。
2. 为 load/store 增加握手或多周期状态机，解除“同组合周期返回数据”的限制。
3. 增加可读写的 MMIO 外设寄存器规范，例如 GPIO data/direction/status。
4. 引入 CSR 和异常信息，把 sticky trap 扩展为可调试的异常路径。
5. 将寄存器堆从 `CoreState` 大数组拆成更接近硬件实现的寄存器堆模块。
