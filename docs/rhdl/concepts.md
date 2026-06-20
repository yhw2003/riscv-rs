# 核心概念

## `Bits<N>` 与 `SignedBits<N>`

`rhdl_bits` 是 `rhdl` 的底层位宽整数库，顶层 `rhdl::prelude::*` 已经 re-export 了常用类型。

| 类型 | 说明 |
| --- | --- |
| `Bits<N>` | N 位无符号硬件整数，当前支持 1 到 128 位。 |
| `SignedBits<N>` | N 位有符号二补码硬件整数。 |
| `b1..b128` | `Bits<N>` 的短别名，例如 `b32`。 |
| `s1..s128` | `SignedBits<N>` 的短别名，例如 `s12`。 |
| `bits::<N>(value)` / `bits(value)` | 构造 `Bits<N>` 的 const helper。 |
| `signed::<N>(value)` / `signed(value)` | 构造 `SignedBits<N>` 的 helper。 |

常用操作：

```rust
let x: b8 = bits(0xFE);
let y: b8 = 2.into();

let wrapped = x + y;        // 0x00，硬件式回绕
let low4: b4 = x.resize();  // 截断为低 4 位
let wide: b16 = x.resize(); // 零扩展
let signed_x: s8 = x.as_signed();

assert!(x.any());
assert!(!b8::ZERO.any());
```

注意：

- `b1` 不等于 `bool`。`b1` 是 1 位整数，`bool` 是布尔值。
- 不同位宽不能直接二元运算，先 `resize` 或用扩展运算显式表达位宽。
- 普通 `+`、`-`、`*` 是固定位宽结果；`XAdd`、`XSub`、`XMul` 等扩展 trait 可表达增长后的位宽结果。

## `Digital`

`Digital` 表示一个类型可以被映射成确定的 bit pattern，因此可参与综合、仿真、trace 和 Verilog 端口生成。

可以派生 `Digital` 的常见类型：

- struct
- enum，包括带 payload 的 enum
- tuple 和 array
- 由其他 `Digital` 字段组成的组合类型

自定义类型通常这么写：

```rust
#[derive(Digital, Timed, PartialEq, Copy, Clone, Default)]
pub struct DecodeOut {
    pub rd: b5,
    pub rs1: b5,
    pub rs2: b5,
    pub imm: b32,
}
```

如果类型会作为异步 `Circuit` 的 `Signal` 内容或 trace 内容使用，通常还会派生 `Timed`、`PartialEq`、`Copy`、`Clone`。

## `Signal<T, Domain>`

`Signal<T, C>` 是带时钟/时序域标记的数据，其中：

- `T: Digital` 是实际承载的数据。
- `C: Domain` 是域标记类型，例如 `Red`、`Green`、`Blue`。

```rust
type AluInput = Signal<(OpCode, b32, b32), Red>;

#[kernel]
fn pass(i: Signal<b32, Red>, _q: ()) -> (Signal<b32, Red>, ()) {
    (signal(i.val()), ())
}
```

域标记的意义不是“颜色本身”，而是让类型系统帮你区分不同 clock domain。RHDL 会检查一些跨域混用错误，例如把 `Signal<T, Red>` 和 `Signal<T, Blue>` 直接做二元操作。

## 时钟域颜色

`rhdl::prelude` 默认提供这些 domain marker：

```rust
Red, Orange, Yellow, Green, Blue, Indigo, Violet
```

它们都是零大小类型，只负责在类型系统中区分 domain。项目中可以约定：

- `Red`：核心主时钟域。
- `Blue`：外设或总线域。
- `Green`：测试/组合示例域。

这只是约定，RHDL 本身不赋予颜色速度含义。

## `ClockReset`

同步电路使用 `ClockReset` 传入统一的 clock/reset：

```rust
#[kernel]
pub fn counter(cr: ClockReset, enable: bool, q: b8) -> (b8, b8) {
    let next = if enable { q + 1 } else { q };
    let next = if cr.reset.any() { bits(0) } else { next };
    (q, next)
}
```

`ClockReset` 在 Verilog 中会成为 clock/reset packed 输入。`rhdl-fpga` 里的 `DFF`、`Counter`、FIFO、stream 等同步 core 都围绕这个模型工作。

## `Timed` 与 `TimedSample`

`Timed` 表示一个类型携带 timing domain 信息。异步 `Circuit` 的 `I`/`O` 要求是 `Timed`；同步 `Synchronous` 的 `I`/`O` 要求是 `Digital`，clock/reset 单独传入。

仿真时常见的时间样本类型是：

```rust
TimedSample<T>
```

iterator 扩展会把普通输入流包装成 timed stream，例如：

```rust
let stream = std::iter::repeat_n(true, 10)
    .with_reset(2)
    .clock_pos_edge(100);
```
