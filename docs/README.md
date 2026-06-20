# 文档索引

这个目录收纳本项目的设计文档和依赖说明。

## SoC

- [当前 SoC 实现架构](soc-architecture.md)：梳理 `Rv32iSoc` 顶层、`Rv32iStep` 执行路径、GPIO MMIO、访存模型和验证入口。

## RHDL

当前整理重点是 `rhdl`。本项目在根 `Cargo.toml` 中锁定的依赖来源是：

```toml
rhdl = { git = "https://github.com/samitbasu/rhdl.git", rev = "c99d5cc53269a247bbc675d0fbd766991d409f56" }
```

建议按下面顺序阅读：

1. [RHDL 总览](rhdl/README.md)
2. [Quickstart](rhdl/quickstart.md)
3. [核心概念](rhdl/concepts.md)
4. [Kernel 编写指南](rhdl/kernels.md)
5. [Circuit 与同步电路](rhdl/circuits.md)
6. [仿真、测试与波形](rhdl/simulation.md)
7. [Verilog 导出与 Fixture](rhdl/verilog.md)
8. [基础组件与功能清单](rhdl/components.md)
9. [RISC-V 项目用例](rhdl/use-cases.md)
10. [常见限制与踩坑](rhdl/faq.md)
