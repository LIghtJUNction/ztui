# ztuio

ztuio 提供文本终端界面的几何、定宽行和 ANSI 帧差分。调用方提供已排版的当前帧与上一帧，库通过 `core::fmt::Write` 输出需要更新的行、终端模式与光标位置。

库使用 `#![no_std]` 和 `#![forbid(unsafe_code)]`，仅引用 `core`，没有第三方依赖。字符串、帧和输出缓冲由调用方持有，库内部不申请堆内存。测试、示例和基准使用 `std`。[源码](src/lib.rs) · [依赖声明](Cargo.toml)

## 使用

需要 Rust **1.100 或更新版本**。Cargo 包名和 Rust 导入名均为 **ztuio**：

```toml
[dependencies]
ztuio = { git = "https://github.com/LIghtJUNction/ztuio" }
```

固定源码时指定 `rev`，并提交应用的 `Cargo.lock`。相邻源码检出可使用 `path = "../ztuio"`。

```rust
use ztuio::Row;

let mut output = String::new();
// 调用方已测得两个汉字占 4 个显示列。
Row::with_columns("界面", 4)
    .write_padded(&mut output, 12)
    .unwrap();
```

[固定数组示例](examples/fixed_buffer.rs) 展示无堆缓冲的输出方式。缓冲不足会返回错误，调用方可以选择固定数组、复用的字符串或自己的写出适配器。

## 接口与职责

| 接口 | 用途 | 调用方负责 |
| --- | --- | --- |
| `Size`、`Point`、`Rect` | 单元坐标、裁剪和分栏 | 像素尺寸与实际窗口布局 |
| `Row::measured` / `Row::with_columns` | 接受已测得的显示列数 | Unicode、grapheme 与终端宽度规则 |
| `Row::write_padded` | 写入文本并补空格到目标宽度 | 可信单行内容、样式和背景 |
| `Frame` | 借用当前帧和上一帧 | 行的生命周期、拼接与布局 |
| `write_diff` | 输出变化行、模式和最终光标 | 设备 I/O、部分写入恢复与退出时的模式恢复 |

帧坐标从零开始，ANSI CUP 坐标从一开始。差分包含 SGR reset、整行擦除、应用光标、鼠标和 bracketed paste 模式；空画布与完全不变的帧写出零字节。[输出约定](docs/contracts.md)

帧写出使用固定栈数组生成 ANSI 坐标，减少格式化与写出调用。相同行先比较借用切片的地址与长度，再在需要时比较内容。这些实现保留相同的 ANSI 字节顺序；[契约测试](tests/contracts.rs)覆盖数字位数边界、完整 `u16` 行范围和不同分配中的相同行。

## 输入与失败处理

传入的行内容与显示列数由调用方校验。库信任已经过滤并排版好的文本和 SGR 样式；图片解码、键盘输入、Unicode 测宽和终端设备访问由应用负责。

完整写出的帧才能作为下一次差分基线。行宽超限会在写出前返回 `TooWide`；输出接口失败时可能已有部分字节写入。调用方应写完剩余内容，或丢弃差分基线，以 `previous = None` 全量重画。

## 验证

```sh
cargo +1.100.0 test --locked
cargo +1.100.0 run --locked --example fixed_buffer
cargo +1.100.0 clippy --locked --all-targets -- -D warnings
cargo +1.100.0 check --locked --lib --target x86_64-unknown-none
```

CI 为 Linux、Windows、macOS 配置相同的契约测试、示例检查与格式检查，并单独检查无操作系统目标。终端字体、背景、输入与图片的交互需要由消费这些接口的应用验证。

## 性能记录

`cargo bench --bench diff -- 200000` 测量首帧、不变画面、单行变化、全行变化、尺寸变化、光标变化和模式变化。它复用调用方输出缓冲，并在计时前比较 ANSI 输出。

[2026-11-18 的历史测量](docs/performance-repeat-20261118.json)记录了原始导出版本的环境、样本与场景差异。[首次测量](docs/benchmark-20261118.json)也保留用于追溯。当前源码合入了后续帧写出优化；评估当前性能时应重新运行基准并记录提交、工具链、目标架构和系统负载。基准范围是帧差分与字符串写入，设备 I/O 和端到端交互由应用单独测量。

## 来源与许可证

本库从 SSMCP 的文本终端核心独立维护，并汇入其后续帧写出优化。固定来源提交与保留的历史记录见 [provenance](docs/provenance.md)。

[MIT License](LICENSE) · Copyright © 2026 LIghtJUNction
