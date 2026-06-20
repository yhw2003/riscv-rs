# Kernel 编写指南

Kernel 是 RHDL 设计里最常写的代码。它是一个普通 Rust 函数，加上 `#[kernel]` 后被 RHDL 编译为硬件可综合的内部表示。

## 异步 Circuit kernel

异步 `Circuit` 的 kernel 签名是：

```rust
fn kernel(input: I, q: Q) -> (O, D)
```

其中：

- `I` 是当前电路输入。
- `O` 是当前电路输出。
- `Q` 是子电路上一层反馈回来的输出集合。
- `D` 是本电路要送入子电路的输入集合。

无子电路时，`D = ()`、`Q = ()`。

```rust
#[kernel]
pub fn add4(i: Signal<(b4, b4), Red>, _q: ()) -> (Signal<b4, Red>, ()) {
    let (a, b) = i.val();
    (signal(a + b), ())
}
```

## 同步 Synchronous kernel

同步 `Synchronous` 的 kernel 多一个 `ClockReset` 参数：

```rust
fn kernel(clock_reset: ClockReset, input: I, q: Q) -> (O, D)
```

示例：一个寄存器反馈计数器，输出当前值，下一拍更新。

```rust
#[derive(Clone, Debug, Synchronous, SynchronousDQ)]
#[rhdl(dq_no_prefix)]
pub struct Counter8 {
    count: rhdl_fpga::core::dff::DFF<b8>,
}

impl SynchronousIO for Counter8 {
    type I = bool;
    type O = b8;
    type Kernel = counter8;
}

#[kernel]
pub fn counter8(cr: ClockReset, enable: bool, q: Q) -> (b8, D) {
    let next = if enable { q.count + 1 } else { q.count };
    let next = if cr.reset.any() { bits(0) } else { next };
    (q.count, D { count: next })
}
```

上例依赖 `rhdl-fpga` 的 `DFF`，当前项目还没有引入该 crate。只使用 `rhdl` 本体时，仍然可以定义同步 kernel 和 `Synchronous`，但基础寄存器 core 需要自己实现或额外添加 `rhdl-fpga`。

## 支持的 Rust 写法

RHDL 的目标是“尽量像 Rust”，当前可综合 kernel 常用写法包括：

- `let` 绑定、变量赋值、块表达式。
- `if` / `else` 表达式。
- `match`，适合 enum 操作码、状态机分支。
- tuple、struct、array 的构造和字段访问。
- 定长 array 的索引。
- 有限形式的 `for` 循环，前提是能在编译期展开。
- 调用其他 `#[kernel]` 或可综合函数。
- 早返回 `return`。
- `trace("name", &value)`，作为仿真 trace 副作用。

## 不支持或需要避免的写法

这些 Rust 特性通常不能放进 `#[kernel]` 函数：

- 引用、指针、生命周期相关逻辑。kernel 值按值传递。
- slice，因为本质上依赖指针；请用 `[T; N]` 定长数组。
- closure。
- `async` / `await`。
- `unsafe`。
- `while`、`while let`、无限 `loop`、`break`、`continue`。
- 函数内部定义 item。
- 宏调用。
- union。
- 浮点数。
- `self` 方法调用；用普通函数调用风格替代。

这些限制只约束可综合 kernel。测试代码、构造函数、普通 Rust helper 可以继续使用 iterator、随机数、Vec、闭包等软件侧能力。

## 写 kernel 的习惯

推荐流程：

1. 先写成普通 Rust 函数，不加 `#[kernel]`。
2. 用普通 Rust 单元测试把业务逻辑测通。
3. 再加 `#[kernel]`，处理可综合子集带来的编译错误。
4. 用 `descriptor` 或 testbench 检查生成 Verilog。

这样做的好处是：Rust Analyzer、类型推断和普通测试先帮你解决大多数逻辑问题，最后再处理硬件综合边界。

## 适合 RISC-V 核心的写法

译码和 ALU 很适合用 `Digital` enum/struct：

```rust
#[derive(Digital, PartialEq, Copy, Clone, Default)]
pub enum AluOp {
    #[default]
    Add,
    Sub,
    And,
    Or,
    Xor,
}

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
    };
    (signal(y), ())
}
```
