# 常见限制与踩坑

## `rhdl::prelude::*` 不是所有东西

大多数常用类型都在 prelude 里，但不是全部。例如当前版本没有直接 re-export `AsyncFunc`，需要：

```rust
use rhdl::core::circuit::function::asynchronous::AsyncFunc;
```

## `rhdl-std` 和 `rhdl-fpga` 不是当前项目依赖

本仓库目前只直接依赖：

```toml
rhdl = { workspace = true }
```

因此 `rhdl_std::slice`、`rhdl_fpga::core::dff::DFF` 这类组件不能直接用。要使用它们，需要在 workspace dependencies 中额外添加对应 crate，并确认 git/path 来源。

## `Bits` 是硬件整数，不是普通 Rust 整数

```rust
let x: b8 = bits(255);
let y = x + 1;
assert_eq!(y.raw(), 0);
```

溢出回绕是正常硬件语义。如果需要 carry/overflow，需要显式扩展位宽或计算标志位。

## 不同位宽不能随便混算

```rust
let a: b4 = bits(3);
let b: b8 = bits(5);
// let c = a + b; // 不合法
let c: b8 = a.resize() + b;
```

这比 Verilog 的隐式扩展更啰嗦，但能减少位宽 bug。

## `b1` 不等于 `bool`

`b1` 是 1 位整数，`bool` 是布尔。控制流条件要用 `bool`：

```rust
let flag: bool = word.any();
let one_bit: b1 = bits(1);
```

## Signal 的 domain 会参与类型检查

不要直接把不同 domain 的 signal 混在一起：

```rust
type A = Signal<b8, Red>;
type B = Signal<b8, Blue>;
```

跨域需要明确的 CDC 设计，而不是简单 cast。RHDL 会尽量在编译/检查阶段报告这类错误。

## Kernel 里不能用软件式 Rust 随意写

`#[kernel]` 只支持可综合子集。尤其注意：

- 不要借用引用。
- 不要用 `Vec`、slice、动态分配。
- 不要用 closure。
- 不要用 `while`/无限 loop。
- 不要用浮点。
- 不要在函数内部定义 item 或调用宏。

测试代码不受这些限制。

## 先测函数，再测电路

推荐先把译码、ALU、立即数生成等核心逻辑写成普通 Rust 函数测通，再加 `#[kernel]`。这样问题更容易定位：

- Rust 单测失败：业务逻辑错。
- 加 `#[kernel]` 后失败：用了不可综合写法或类型不能映射。
- Verilog testbench 失败：综合/降级过程与 Rust 行为不一致，需要查 descriptor 或中间表示。

## 文档对应固定 rev

本文档基于：

```text
c99d5cc53269a247bbc675d0fbd766991d409f56
```

如果后续升级 `rhdl`，需要重点复核：

- `rhdl::prelude` 的 re-export 列表。
- `Circuit`/`Synchronous` trait 签名。
- `rhdl-fpga` 组件名称和模块路径。
- `kernel` 支持的 Rust 子集。
