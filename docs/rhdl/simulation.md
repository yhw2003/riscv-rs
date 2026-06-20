# 仿真、测试与波形

RHDL 的一个重要优势是：设计本身是 Rust，测试也可以用普通 Rust 写。验证可以分层进行，从最快的 kernel 单测，到 Circuit 仿真，再到 Verilog testbench。

## 1. 直接测试 kernel

适合纯组合逻辑，比如 ALU、译码器、立即数生成器。

```rust
#[test]
fn alu_add_works() {
    let input = AluIn {
        op: AluOp::Add,
        lhs: bits(1),
        rhs: bits(2),
    };

    let (out, _) = alu(signal(input), ());
    assert_eq!(out.val(), bits(3));
}
```

这种测试完全不需要启动硬件仿真器，反馈最快。

## 2. 手动 Circuit 仿真

适合小型组合电路或需要检查 `Circuit` 封装是否正确的场景。

```rust
#[test]
fn xor_gate_sim_works() {
    let gate = XorGate;
    let mut state = gate.init();

    let out = gate.sim(signal((true, false)), &mut state);
    assert_eq!(out.val(), true);
}
```

## 3. Iterator 风格仿真

`rhdl::prelude` re-export 了多种仿真 iterator 扩展：

| 扩展 | 作用 |
| --- | --- |
| `with_reset(n)` | 给输入流添加 reset 周期。 |
| `without_reset()` | 生成无 reset 的同步输入流。 |
| `clock_pos_edge(period)` | 给同步输入流加正沿 clock 时间点。 |
| `uniform(...)` | 生成均匀时间采样。 |
| `merge_map(...)` | 合并/映射多个 timed stream。 |

示例来自 `rhdl-fpga` 的计数器测试风格：

```rust
let inputs = std::iter::repeat_n(true, 100)
    .with_reset(4)
    .clock_pos_edge(100);

let uut: rhdl_fpga::core::counter::Counter<6> = Default::default();
let samples = uut.run(inputs);
```

这段需要额外引入 `rhdl-fpga`。当前项目只依赖 `rhdl` 时，仍可使用 `with_reset`、`clock_pos_edge`、`run` 等仿真工具，但被测同步 core 需要来自本项目自身或额外 crate。

`run` 的输出可以继续用 iterator 消费，也可以收集成 VCD、SVG 或 testbench。

## 4. VCD 波形

```rust
let inputs = std::iter::repeat_n(true, 100)
    .with_reset(4)
    .clock_pos_edge(100);

let uut: rhdl_fpga::core::counter::Counter<6> = Default::default();
let vcd: VcdFile = uut.run(inputs).collect();
vcd.dump_to_file("counter.vcd")?;
```

同样，这里的 `Counter` 来自 `rhdl-fpga`。如果只测试本项目自己的 `Synchronous`，把 `uut` 换成对应模块即可。

VCD 适合交给 Surfer、GTKWave 等波形工具查看。

## 5. SVG trace

`SvgFile` 可以直接生成适合文档或快速检查的小波形图。

```rust
let gate = XorGate;
let inputs = [(false, false), (false, true), (true, false), (true, true)];
let mut state = gate.init();
let session = Session::default();
let mut svg = SvgFile::default();

for (time, input) in inputs.iter().enumerate() {
    let sample = session.traced_at_time((time * 100) as u64, || {
        let _ = gate.sim(signal(*input), &mut state);
    });
    svg.record(&sample)?;
}

let svg_text = svg.to_string(&SvgOptions::default())?;
```

## 6. Verilog testbench

RHDL 的 testbench 机制可以用 Rust 仿真样本生成 Verilog 测试台，再调用 Icarus Verilog 等工具对 RTL/NTL 结果做一致性检查。

源码里的测试常见形态：

```rust
let out_stream = uut.run(stream);
let tb = out_stream.collect::<SynchronousTestBench<_, _>>();

let rtl = tb.rtl(&uut, &Default::default())?;
rtl.run_iverilog()?;

let ntl = tb.ntl(&uut, &Default::default())?;
ntl.run_iverilog()?;
```

这要求本机安装对应 Verilog 工具，并且相关 crate feature/依赖可用。当前项目没有配置这些工具链，文档先记录用法。

## 推荐验证顺序

1. 用普通 Rust 单测覆盖 kernel 的边界条件。
2. 用 `Circuit::sim` 或 `Synchronous::run` 覆盖结构连接。
3. 对关键模块生成 VCD/SVG，人工看一眼时序。
4. 对即将导出的模块生成 Verilog testbench，与 Rust 行为对齐。
5. 最后接入 FPGA/ASIC 工具链做综合和时序。
