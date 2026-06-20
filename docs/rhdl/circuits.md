# Circuit 与同步电路

RHDL 有两条主要电路抽象：

- `Circuit`：异步/组合电路，输入输出通常是 `Signal<T, Domain>`。
- `Synchronous`：同步电路，输入输出是 `Digital`，clock/reset 由 `ClockReset` 统一传入。

## `Circuit`

`Circuit` 由三个 trait 共同描述：

```rust
pub trait CircuitDQ {
    type D: Timed;
    type Q: Timed;
}

pub trait CircuitIO: CircuitDQ {
    type I: Timed;
    type O: Timed;
    type Kernel;
}

pub trait Circuit: CircuitIO {
    type S: Clone + PartialEq;
    fn init(&self) -> Self::S;
    fn sim(&self, input: Self::I, state: &mut Self::S) -> Self::O;
    fn descriptor(&self, scoped_name: ScopedName) -> Result<Descriptor<AsyncKind>, RHDLError>;
}
```

实际使用时通常写：

```rust
#[derive(Circuit, Clone)]
pub struct XorGate;
```

然后手动实现 `CircuitDQ` 和 `CircuitIO`。如果有子电路，也可以用 `#[derive(CircuitDQ)]` 辅助生成 `D/Q` 类型。

## `D` 与 `Q`

RHDL 的组合方式是结构化的。父电路的 kernel 不直接调用子电路，而是：

- 从 `Q` 读取子电路当前输出。
- 返回 `D`，作为子电路下一次模拟/连接的输入。

如果父电路有这些字段：

```rust
pub struct HalfAdder {
    xor: XorGate,
    and: AndGate,
}
```

那么 `D` 和 `Q` 应该结构对应：

```rust
#[derive(Timed, Digital, PartialEq, Copy, Clone)]
pub struct HalfAdderD {
    pub xor: <XorGate as CircuitIO>::I,
    pub and: <AndGate as CircuitIO>::I,
}

#[derive(Timed, Digital, PartialEq, Copy, Clone)]
pub struct HalfAdderQ {
    pub xor: <XorGate as CircuitIO>::O,
    pub and: <AndGate as CircuitIO>::O,
}
```

字段名必须和子电路字段名对应。RHDL 的 derive 宏就是基于这个约定连线。

## 子电路组合示意

```rust
#[derive(Circuit, CircuitDQ, Clone)]
pub struct HalfAdder {
    xor: XorGate,
    and: AndGate,
}

impl CircuitIO for HalfAdder {
    type I = Signal<(bool, bool), Red>;
    type O = Signal<HalfAdderOut, Red>;
    type Kernel = half_adder;
}

#[derive(Digital, Timed, PartialEq, Copy, Clone, Default)]
pub struct HalfAdderOut {
    pub sum: bool,
    pub carry: bool,
}

#[kernel]
pub fn half_adder(i: Signal<(bool, bool), Red>, q: Q) -> (Signal<HalfAdderOut, Red>, D) {
    let out = HalfAdderOut {
        sum: q.xor.val(),
        carry: q.and.val(),
    };
    let d = D {
        xor: i,
        and: i,
    };
    (signal(out), d)
}
```

上例中 `Q`/`D` 的具体名字由 `CircuitDQ` derive 生成，源码中也可手写。复杂设计建议使用具名 struct 输出，避免 tuple 字段含义不清。

## `Synchronous`

同步电路的 trait 形状类似，但 `ClockReset` 是固定输入：

```rust
pub trait SynchronousDQ {
    type D: Digital;
    type Q: Digital;
}

pub trait SynchronousIO: SynchronousDQ {
    type I: Digital;
    type O: Digital;
    type Kernel;
}

pub trait Synchronous: SynchronousIO {
    type S: PartialEq + Clone;
    fn init(&self) -> Self::S;
    fn sim(&self, clock_reset: ClockReset, input: Self::I, state: &mut Self::S) -> Self::O;
}
```

`rhdl-fpga` 中的 `DFF<T>` 是最基础的同步状态元件：

```rust
let ff: rhdl_fpga::core::dff::DFF<b32> = rhdl_fpga::core::dff::DFF::new(bits(0));
```

它是正沿触发、active-high reset。当前项目尚未添加 `rhdl-fpga` 依赖；如果要实现寄存器堆、PC 寄存器、流水寄存器等状态元件，可以考虑引入或仿照它实现。

## `Func` 与 `AsyncFunc`

如果只是把一个纯函数包装成电路，可以用 wrapper 减少样板代码：

- `rhdl::core::circuit::function::asynchronous::AsyncFunc`
- `rhdl::core::circuit::function::synchronous::Func`

示例：

```rust
use rhdl::core::circuit::function::asynchronous::AsyncFunc;
use rhdl::prelude::*;

#[kernel]
fn adder(i: Signal<(b4, b4), Red>) -> Signal<b4, Red> {
    let (a, b) = i.val();
    signal(a + b)
}

let circuit = AsyncFunc::new::<adder>()?;
```

当前版本的 `rhdl::prelude::*` 没有直接 re-export `AsyncFunc`，需要从 `rhdl::core::circuit::function::asynchronous` 引入。
