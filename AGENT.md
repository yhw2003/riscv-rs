# 项目介绍
这是一个通过用rust来实现在fpga上可运行的riscv指令集的简易实现。

# 核心依赖
- rhdl：这是一个让rust可以通过高级综合输出verilog代码的rust框架，类似于chisel

# 项目架构
- digital-base：基础逻辑元件的实现，例如加法器等基础组建
- riscv-core： riscv指令集相关的逻辑实现
- docs：文档