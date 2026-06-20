# RHDL 总览

`rhdl` 是一个基于 Rust 的硬件描述与高层综合框架。它的目标不是重新发明一套语法，而是让硬件逻辑尽量写成普通 Rust 函数，再由 `#[kernel]` 宏把可综合的 Rust 子集编译为内部表示、仿真模型和 Verilog。

本项目目前只在 `riscv-core` 中依赖 `rhdl`，`digital-base` 尚未引入它。因此这些文档先作为后续实现 RISC-V 核心和基础逻辑元件的依赖手册。

## 分层地图

- [Quickstart](quickstart.md)：从 XOR/ALU 两个最小例子开始，展示可综合 kernel、Circuit、仿真和 Verilog 导出。
- [核心概念](concepts.md)：解释 `Bits`、`Digital`、`Signal`、时钟域、`ClockReset` 等基础模型。
- [Kernel 编写指南](kernels.md)：说明 `#[kernel]` 支持的 Rust 写法、输入输出签名和常见约束。
- [Circuit 与同步电路](circuits.md)：说明组合电路、同步电路、子电路组合、`D/Q` 反馈类型。
- [仿真、测试与波形](simulation.md)：说明直接调用 kernel、运行 Circuit、基于 iterator 的仿真、VCD/SVG 波形。
- [Verilog 导出与 Fixture](verilog.md)：说明 `descriptor`、`Fixture`、`bind!`、顶层端口绑定。
- [基础组件与功能清单](components.md)：整理 `rhdl` 本体和同仓库扩展 crate 提供的能力。
- [RISC-V 项目用例](use-cases.md)：把 RHDL 能力映射到本项目的 CPU 模块划分。
- [常见限制与踩坑](faq.md)：记录当前版本中容易误解的地方。

## 当前版本边界

本文档基于本仓库锁定的 `rhdl` git rev：

```text
c99d5cc53269a247bbc675d0fbd766991d409f56
```

源码中同一仓库还包含 `rhdl-std`、`rhdl-fpga`、`rhdl-bsp`、`rhdl-toolchains` 等 crate；但当前项目的 `Cargo.toml` 只直接依赖了 `rhdl` 这个顶层 crate。文档会明确区分“当前可直接使用的 API”和“需要额外依赖的扩展组件”。

## RHDL 做了什么

RHDL 把硬件设计拆成几层：

- 有限宽度数据：`Bits<N>`、`SignedBits<N>`、`b8`/`s8` 这类别名。
- 可综合数据类型：实现或派生 `Digital` 的 struct、enum、tuple、array。
- 时序/时钟域类型：`Signal<T, Domain>`、`Timed`、`ClockReset`、`Red`/`Blue` 等域标记。
- 可综合计算：用 `#[kernel]` 标注普通 Rust 函数。
- 电路结构：用 `Circuit`/`Synchronous` trait 和 derive 宏组合子电路。
- 仿真与验证：Rust 里直接调用函数、运行电路、收集测试台、导出 VCD/SVG。
- HDL 输出：从 descriptor 或 fixture 生成 Verilog 模块。

对本项目来说，RHDL 很适合把 RISC-V 的译码、ALU、寄存器堆控制、流水级控制等逻辑写成强类型 Rust，再逐步验证和导出硬件描述。
