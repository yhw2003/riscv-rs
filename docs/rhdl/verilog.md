# Verilog 导出与 Fixture

RHDL 能从 kernel/Circuit 生成 Verilog。对小模块可以直接拿 descriptor；对需要友好顶层端口的模块，使用 `Fixture`。

## 直接导出模块

```rust
let gate = XorGate;
let descriptor = gate.descriptor("xor_gate".into())?;
let hdl = descriptor.hdl()?;

println!("{}", hdl.modules);
```

这会生成一个 packed 输入/输出的 Verilog 模块。例如 `(bool, bool)` 这类 tuple 会被压成 bit vector。

## 使用 `Fixture` 暴露顶层端口

`Fixture` 是顶层包装模块：它实例化 RHDL 电路，并把 packed 输入输出中的某些 bit range 绑定为外部端口。

当前版本中 `Fixture` 在 prelude 中可用，`AsyncFunc` 需要显式从 `rhdl::core` 引入：

```rust
use rhdl::core::circuit::function::asynchronous::AsyncFunc;
use rhdl::prelude::*;

#[kernel]
fn adder(i: Signal<(b4, b4), Red>) -> Signal<b4, Red> {
    let (a, b) = i.val();
    signal(a + b)
}

let adder = AsyncFunc::new::<adder>()?;
let mut fixture = Fixture::new("adder_top", adder);

let input = Signal::<(b4, b4), Red>::dont_care();
let output = Signal::<b4, Red>::dont_care();

bind!(fixture, a -> input.val().0);
bind!(fixture, b -> input.val().1);
bind!(fixture, sum <- output.val());

let module = fixture.module()?;
println!("{}", module.pretty());
```

`bind!` 方向含义：

- `port -> input.path`：外部端口驱动电路输入。
- `port <- output.path`：电路输出驱动外部端口。

这类写法能把内部 packed 端口展开为可读的 `a`、`b`、`sum`，更适合作为 FPGA 顶层、仿真顶层或后续集成边界。

## 黑盒模块

`rhdl::prelude` 暴露了：

- `circuit_black_box`
- `synchronous_black_box`
- `constant`

它们用于把外部 HDL/IP 以黑盒形式接入 RHDL 的电路图。适合封装厂商 RAM、PLL、DDR controller、已有 Verilog 模块等。

## HDL 输出阶段

RHDL 内部大致会经历：

1. Rust AST / proc macro 捕获 kernel。
2. 降到 RHIF。
3. 类型推断、clock domain 检查、常量传播、死代码删除等 pass。
4. 降到 RTL/NTL。
5. 生成 Verilog AST，再 pretty print。

使用者通常不需要手动操作这些阶段；遇到疑难问题时，可以用 `compile_design_stage1`、`CompilationMode`、`descriptor` 等 API 查看中间结果。

## 与本项目的关系

对 `riscv-core`，建议模块边界这样规划：

- 早期：每个小功能只写 kernel 和 Rust 单测。
- 中期：把 ALU、decoder、register file interface 等组合成 `Circuit`/`Synchronous`。
- 后期：用 `Fixture` 把 CPU 顶层端口绑定为 clock/reset、指令总线、数据总线、中断等明确端口。
