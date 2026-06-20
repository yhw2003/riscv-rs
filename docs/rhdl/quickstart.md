# Quickstart

下面的例子都基于本项目已配置的依赖：

```rust
use rhdl::prelude::*;
```

## 1. 固定位宽数据

硬件中经常需要 4 位、5 位、32 位这类非 Rust 原生整数。RHDL 用 `Bits<N>` 表示无符号固定位宽，用 `SignedBits<N>` 表示有符号固定位宽，并提供 `b1..b128`、`s1..s128` 别名。

```rust
use rhdl::prelude::*;

let a: b4 = bits(0b1010);
let b: b4 = 3.into();
let sum: b4 = a + b;       // 固定位宽二补码回绕加法
let anded: b4 = a & b;

assert_eq!(sum.raw(), 0b1101);
assert_eq!(anded.raw(), 0b0010);
```

建议在硬件逻辑里优先使用 `b8`、`b16`、`b32` 等类型，而不是 `u8`、`u16`、`u32`。这样位宽、截断和回绕语义都更接近硬件。

## 2. 最小 XOR gate

`#[kernel]` 标注的函数必须是合法 Rust 函数，同时只能使用 RHDL 支持的可综合 Rust 子集。

```rust
use rhdl::prelude::*;

#[derive(Circuit, Clone)]
pub struct XorGate;

impl CircuitDQ for XorGate {
    type D = ();
    type Q = ();
}

impl CircuitIO for XorGate {
    type I = Signal<(bool, bool), Red>;
    type O = Signal<bool, Red>;
    type Kernel = xor_gate;
}

#[kernel]
pub fn xor_gate(i: Signal<(bool, bool), Red>, _q: ()) -> (Signal<bool, Red>, ()) {
    let (a, b) = i.val();
    (signal(a ^ b), ())
}
```

要点：

- `Signal<T, Red>` 表示 `T` 类型的数据属于 `Red` 这个时钟/时序域。
- `.val()` 取出信号承载的值。
- `signal(value)` 把值重新包装成 `Signal`，具体 domain 由返回类型推断。
- 这个 gate 没有子电路，所以 `D` 和 `Q` 都是 `()`。
- `CircuitIO::Kernel` 的签名固定是 `fn(I, Q) -> (O, D)`。

## 3. 直接测试 kernel

kernel 仍然是 Rust 函数，所以最早期的单元测试可以不经过电路模拟器。

```rust
#[test]
fn xor_kernel_works() {
    let cases = [
        ((false, false), false),
        ((false, true), true),
        ((true, false), true),
        ((true, true), false),
    ];

    for (input, expected) in cases {
        let (out, _) = xor_gate(signal(input), ());
        assert_eq!(out.val(), expected);
    }
}
```

## 4. 运行 Circuit 仿真

派生 `Circuit` 后，可以初始化状态并调用 `sim`。

```rust
#[test]
fn xor_circuit_sim_works() {
    let gate = XorGate;
    let mut state = gate.init();

    let y = gate.sim(signal((true, false)), &mut state);
    assert_eq!(y.val(), true);
}
```

这类手写 loop 适合小电路。更大的同步电路通常会用 iterator、`run`、`with_reset`、`clock_pos_edge` 等组合式仿真工具，见 [仿真、测试与波形](simulation.md)。

## 5. 一个 ALU kernel

RHDL 支持 payload enum、struct、tuple、array 等可综合数据。下面是一个 4 位 ALU 的组合逻辑。

```rust
use rhdl::prelude::*;

#[derive(Digital, PartialEq, Default, Copy, Clone)]
pub enum OpCode {
    #[default]
    Add,
    And,
    Or,
    Xor,
}

#[derive(Circuit, Clone)]
pub struct Alu;

impl CircuitDQ for Alu {
    type D = ();
    type Q = ();
}

impl CircuitIO for Alu {
    type I = Signal<(OpCode, b4, b4), Green>;
    type O = Signal<b4, Green>;
    type Kernel = alu;
}

#[kernel]
pub fn alu(i: Signal<(OpCode, b4, b4), Green>, _q: ()) -> (Signal<b4, Green>, ()) {
    let (opcode, a, b) = i.val();
    let y = match opcode {
        OpCode::Add => a + b,
        OpCode::And => a & b,
        OpCode::Or => a | b,
        OpCode::Xor => a ^ b,
    };
    (signal(y), ())
}
```

这和 RISC-V ALU 的建模方式很接近：先把操作码做成 `Digital` enum，再在 kernel 里 `match` 产生不同硬件路径。

## 6. 生成 Verilog

最简单的方式是从电路 descriptor 取 HDL：

```rust
let gate = XorGate;
let descriptor = gate.descriptor("xor_gate".into())?;
let hdl = descriptor.hdl()?;
println!("{}", hdl.modules);
```

如果需要把内部 packed 输入输出映射成更友好的顶层端口，可以使用 `Fixture` 和 `bind!`，见 [Verilog 导出与 Fixture](verilog.md)。
