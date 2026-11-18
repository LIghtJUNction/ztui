# ztui

ztui 提供文本终端界面需要的几何、定宽行和 ANSI 帧差分。调用方交给它已经排版好的当前帧与上一帧，它写出需要更新的行和终端模式。SSMCP 使用这些接口生成文本画面；本仓库可以独立构建和使用。

库源码使用 `#![no_std]` 和 `#![forbid(unsafe_code)]`，仅引用 `core`，没有 normal、dev、build 或 optional 第三方依赖。字符串、帧和输出缓冲由调用方拥有；库内部不申请堆内存。测试、示例和基准使用 Rust 标准库 `std`。[源码](src/lib.rs) · [依赖声明](Cargo.toml)

## 使用

需要 Rust **1.100 或更新版本**。Cargo 包名是 **ztui-core**，Rust 库名是 **ztui**。crates.io 上已有的 [ztui](https://crates.io/crates/ztui) 是其他作者的 ZeroTier 工具。

从这个仓库引用时，可在应用的 `Cargo.toml` 中使用包别名：

```toml
[dependencies]
ztui = { package = "ztui-core", git = "https://github.com/LIghtJUNction/ztui" }
```

需要固定源码时添加具体 `rev`，并提交应用自己的 `Cargo.lock`。相邻源码检出也可以使用 `path = "../ztui"`。本次仓库导出没有发布新的 crate 或 tag；版本发布由独立的发布步骤处理。

```rust
use ztui::Row;

let mut output = String::new();
// 调用方已测得两个汉字占 4 个显示列。
Row::with_columns("界面", 4)
    .write_padded(&mut output, 12)
    .unwrap();
```

[固定数组示例](examples/fixed_buffer.rs) 展示不使用堆缓冲的写出方式；缓冲不足会返回错误。输出容器和应用自己的测宽函数仍可能分配内存，因此库内部零分配不等于整个程序零分配。

## 接口与职责

| 接口 | 用途 | 调用方负责的边界 |
|---|---|---|
| `Size`、`Point`、`Rect` | 显示单元坐标、裁剪和分栏 | 像素尺寸；矩形排他边界溢出时构造返回 `None` |
| `Row::measured` / `Row::with_columns` | 接受已测得的显示列数 | Unicode、grapheme 与终端宽度规则 |
| `Row::write_padded` | 写入文本并补空格到目标宽度 | 可信单行内容、样式和背景 |
| `Frame` | 借用当前帧和上一帧 | 行的生命周期、拼接、布局与宽度限制 |
| `write_diff` | 输出变化行、终端模式和最终光标 | 终端 I/O、部分写入恢复与模式恢复 |

帧坐标从零开始，输出的 ANSI CUP 坐标从一开始。差分包含必要的 SGR reset、整行擦除、应用光标、鼠标和 bracketed paste 模式；空画布和完全不变的帧不写字节。相同行比较仍需读取文本，零分配不代表零计算。[输出与布局约定](docs/contracts.md)

## 输入与失败边界

**ztui 信任调用方提供的单行文本与显示列数。** 它不验证实际字形宽度，也不净化 ANSI 控制流。只应传入已经过滤并排版好的文本和可信 SGR 样式；换行、OSC、光标跳转和清屏等控制序列由上层处理。图片解码、Kitty 媒体通道、键盘输入、主题和历史记录也由应用负责。

**只有完整写出的帧才能成为下一次差分基线。** 行宽超限的 `TooWide` 错误在写出前返回；输出接口失败时，设备可能已经收到部分字节。调用方应继续写完剩余内容，或丢弃差分基线并使用 `previous = None` 全量重画。不能把部分写出的当前帧当成上一帧。

## 本地验证

在独立仓库目录运行：

```sh
cargo test
cargo run --example fixed_buffer
cargo clippy --all-targets -- -D warnings
cargo bench --bench diff -- 200000
```

[契约测试](tests/contracts.rs) 覆盖 ANSI 顺序、不变帧、单行更新、布局和尺寸变化、模式和光标、输出错误、显式 Unicode 列宽与矩形边界。真实终端的字体、背景及原生图片行为仍需在应用层验收。

导出来源已经纳入 SSMCP 的 R14 workspace 测试、Clippy 和打包验证；[固定版本的记录](https://github.com/LIghtJUNction/ssmcp/blob/a83cb4955ba6865bc7fb10077ea4be150c2fc725/docs/reports/ssmcp-iteration-20261118-validation.json) 明确区分验收范围。这份既有结果不等于对本次独立仓库元数据提交重新运行了上述命令。导出来源和源码等价检查见 [provenance](docs/provenance.md)。

## 性能：有快有慢

2026-11-18 的重复测量比较本库与 SSMCP `97c026c` 的原帧差分算法。环境是 x86_64 Linux、Rust 1.100.0、`opt-level=z`；两种实现都复用调用方的 `String`。共运行 3 次独立基准进程，每种实现、每个场景每次执行两轮各 200,000 次，第二轮交换执行顺序。[基准源码](benches/diff.rs) · [完整重复测量](docs/performance-repeat-20261118.json)

| 场景 | ztui / 原实现，配对比值中位数 |
|---|---:|
| 首帧，64×160 | 1.0910 |
| 不变画面 | 1.0909 |
| 单行变化 | 0.9039 |
| 全部行变化 | 0.8765 |
| 尺寸变化，64×180 | 1.1775 |
| 仅光标变化 | 0.5439 |
| 仅模式变化 | 0.5515 |

比值小于 1 表示该场景的 ztui 路径更快。这里取三个配对运行比值的中位数，**不是**两组绝对耗时各自取中位数再相除。七个固定场景在每次计时前各比较一次完整 ANSI 输出，共 21 次字节等价检查；计时循环并不逐次重新断言等价。

这组结果中，首帧、不变画面和尺寸变化较慢，单行、全行、光标及模式变化较快。首次 50,000 次短测中出现的尺寸变化比值 **3.3873** 也保留在证据里，不能据复测结果把它删除或断言为噪声。另保留[更早的测量记录](docs/benchmark-20261118.json)，便于追溯。

基准只测帧差分与字符串写入，不包含 Unicode 测宽、终端 I/O、GPU、网络或端到端交互延迟；开发机同时有其他负载。这些数据不能证明所有场景或整套应用更快。上面的普通 `cargo bench` 使用当前 Cargo bench profile；比较既有数据时需保持原优化配置并记录环境。

## 来源与许可证

本库从 SSMCP 固定提交 `3570af1507031e83eede4abeb0853e609f039f46` 的 `crates/ztui` 导出，保留目录历史。运行时源码、测试、示例和基准在导出时未改动；仓库元数据和文档调整为独立项目。[来源记录](docs/provenance.md)

[MIT License](LICENSE) · Copyright © 2026 LIghtJUNction
