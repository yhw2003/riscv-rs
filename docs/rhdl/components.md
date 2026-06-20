# 基础组件与功能清单

本页按 crate 和能力层整理 RHDL 提供的基础组件。当前项目直接依赖的是 `rhdl`，同仓库扩展 crate 需要额外添加依赖。

## `rhdl` 顶层 crate

`rhdl` 是当前项目已经可用的入口，主要 re-export：

| 模块/类型 | 作用 |
| --- | --- |
| `rhdl::prelude::*` | 常用类型、宏、仿真扩展、trace、fixture、Verilog 工具的集合导入。 |
| `rhdl::bits` | `rhdl-bits` re-export，包含 `Bits`、`SignedBits`、位宽别名等。 |
| `rhdl::core` | `rhdl-core` re-export，包含 `Circuit`、`Synchronous`、compiler、sim、trace 等。 |
| `rhdl::vlog` | Verilog AST/formatter 相关能力。 |
| `rhdl::rtt` | trace type 相关能力。 |

## 位宽与算术

来自 `rhdl-bits`，已进入 prelude：

- `Bits<N>`、`SignedBits<N>`。
- `b1..b128`、`s1..s128`。
- `bits`、`signed` 构造函数。
- 普通算术/逻辑运算：`+`、`-`、`*`、`&`、`|`、`^`、`!`、`<<`、`>>`。
- 扩展运算 trait：`XAdd`、`XSub`、`XMul`、`XNeg`、`XSgn`。
- 方法：`resize`、`raw`、`dyn_bits`、`any`、`all`、`xor`、`as_signed`。

`Bits<N>` 当前上限是 128 位。更宽的数据可以用 struct/array 表达，但默认不提供超宽算术。

## 可综合数据类型

来自 `rhdl-core` 和 `rhdl-macro`：

- `#[derive(Digital)]`：把 struct/enum/tuple-like 数据转为 bit pattern。
- `#[derive(Timed)]`：给包含 timed 信息的类型派生时序能力。
- `Kind`、`TypedBits`、`TraceType`：运行期类型描述、bit 编码和 trace 描述。
- 支持带 payload 的 enum，适合指令、状态机、packet、总线响应建模。

## 信号与时钟域

- `Signal<T, C>`：带 domain 的数字信号。
- `signal(value)`：构造 `Signal`。
- `Domain` 和内置域：`Red`、`Orange`、`Yellow`、`Green`、`Blue`、`Indigo`、`Violet`。
- `Clock`、`Reset`、`ResetN`。
- `ClockReset`、`clock_reset`。

## Kernel 与电路结构

- `#[kernel]`：标注可综合函数。
- `Circuit`、`CircuitIO`、`CircuitDQ`：异步/组合电路抽象。
- `Synchronous`、`SynchronousIO`、`SynchronousDQ`：同步电路抽象。
- `#[derive(Circuit)]`、`#[derive(CircuitDQ)]`。
- `#[derive(Synchronous)]`、`#[derive(SynchronousDQ)]`。
- `Adapter`：同步/异步或 domain 适配场景中的辅助组件。
- `AsyncFunc`：把 `fn(I) -> O` 包装为异步 `Circuit`，路径是 `rhdl::core::circuit::function::asynchronous::AsyncFunc`。
- `Func`：把 `fn(ClockReset, I) -> O` 包装为同步 `Synchronous`，prelude 中 re-export 了同步版本 `Func`。

## 仿真与测试

prelude 中可用：

- `RunExt`、`RunSynchronousExt`、`RunSynchronousFeedbackExt`。
- `TestBench`、`SynchronousTestBench`、`TestBenchOptions`。
- iterator 扩展：`with_reset`、`without_reset`、`clock_pos_edge`、`uniform`、`merge_map`。
- probe 扩展：`ProbeExt`、`SynchronousProbeExt`、`AroundEventExt`。
- `TimedSample`、`timed_sample`。

输出容器：

- `VcdFile`、`VcdOptions`。
- `SvgFile`、`SvgOptions`。
- `Session`、`TracedSample`。

## Trace 与诊断

- `trace("name", &value)`：在 kernel 或 sim 中记录信号。
- `trace_push_path` / `trace_pop_path`：组织层级路径。
- `TraceKey`、`ScopedName`：trace/descriptor 层级命名。
- `RHDLError`：统一错误类型。

## Verilog 与 Fixture

- `compile_design`、`compile_design_stage1`。
- `CompilationMode::{Asynchronous, Synchronous}`。
- `HDLDescriptor`、`Descriptor`。
- `Fixture`、`Driver`、`MountPoint`、`ExportError`。
- `bind!`、`path!`、`export`。
- `Pretty` formatter、`parse_quote_miette`。
- `circuit_black_box`、`synchronous_black_box`、`constant`。

## `rhdl-std`

同仓库扩展 crate，当前项目未依赖。它提供一组可综合 bits helper：

- `slice::<N, M>(x, start)`：从 `Bits<N>` 切出 `Bits<M>`。
- `get_bit`、`set_bit`。
- `any`、`all`、`xor`。
- `as_signed`、`as_unsigned`、`sign_bit`。
- `UnsignedMethods`、`SignedMethods` 扩展 trait。

如果项目后续需要频繁做位切片、立即数字段抽取、指令字段抽取，可以考虑把 `rhdl-std` 加入 workspace dependencies。

## `rhdl-fpga`

同仓库扩展 crate，当前项目未依赖。它是更接近 IP 库的一层，模块包括：

| 模块 | 能力 |
| --- | --- |
| `core` | `DFF`、`Counter`、`Delay`、constant、RAM、slice、option 等基础 core。 |
| `fifo` | 同步/异步 FIFO、读写逻辑、测试 filler/drainer。 |
| `stream` | ready/valid stream 组件，支持 backpressure。 |
| `pipe` | 无 backpressure 的 pipeline 组件，类似 iterator 的 map/filter/filter_map/chunked。 |
| `axi4lite` | AXI4-Lite 类型、channel、endpoint、controller、register、stream bridge。 |
| `cdc` | clock domain crossing 相关 core。 |
| `reset` | reset conditioner、reset polarity 转换。 |
| `gray` | Gray code encode/decode。 |
| `rng` | xorshift 伪随机数 core。 |
| `dsp` | DSP 相关 core，目前可见 `lerp`。 |
| `tristate` | tristate IO 支持。 |

这些组件对 CPU 项目很有用：PC/寄存器可用 DFF/RAM，访存队列可用 FIFO，外设总线可参考 AXI4-Lite，跨域信号可用 CDC/reset 组件。

## `rhdl-bsp` 与 `rhdl-toolchains`

同仓库还包含：

- `rhdl-bsp`：板级支持和厂商/板卡相关 glue。
- `rhdl-toolchains`：Yosys、nextpnr-ice40、IceStorm、Vivado、openFPGALoader 等工具链封装。

当前项目还没进入板级 bitstream 生成阶段，暂时不需要引入。
