# ztui

ztui 是 Project Z 的文本 TUI 基础库。它只负责显示单元几何、定宽行补齐和 ANSI 帧差分；SSMCP 的文本帧实际调用这套接口。

库源码使用 `#![no_std]` 和 `#![forbid(unsafe_code)]`，只引用 `core`，没有 normal、dev、build 或 optional 依赖。行、上一帧、输出缓冲都由调用方拥有。测试、示例和基准程序使用 Rust 工具链自带的 `std`，没有额外 crate。

## 接口与职责

| 接口 | 用途 | 边界 |
|---|---|---|
| `Size`、`Point`、`Rect` | 以显示单元计算位置、裁剪、分栏 | 不处理像素；矩形边界溢出时构造失败 |
| `Row::measured` / `Row::with_columns` | 接受调用方测得的可见列数 | 不内置 Unicode 表，不自行推断 ANSI 或 grapheme 宽度 |
| `Row::write_padded` | 向调用方输出补齐一行 | 超宽先报错，不切坏 UTF-8；背景由调用方的样式决定 |
| `Frame` | 借用现有帧与行 | 不复制屏幕或创建第二份日志存储 |
| `write_diff` | 输出变化行、模式与最终光标 | 相同行不重画；失败输出不能作为下一帧基线 |

`write_diff` 使用零起点行列坐标，生成终端需要的一起点 CUP。输出包括必要的 SGR reset、整行擦除、应用光标/键盘、鼠标与 bracketed paste 模式。空画布和完全不变的帧不写字节。终端初始化、退出恢复、I/O 与部分写入重试由调用方负责。

## 定宽行

```rust
use ztui::Row;

let mut output = String::new();
// 宽度来自调用方的终端/Unicode实现；这里的两个汉字已确定占4列。
Row::with_columns("界面", 4).write_padded(&mut output, 12).unwrap();
```

真实应用可通过 `Row::measured(text, width_provider)` 传入自己的宽度规则。结合字、emoji 组合、不同终端对宽字符的策略仍属于调用方。对于带 ANSI 的文本，调用方必须排除控制序列的宽度，并只提供单行、无光标跳转的可信 SGR 内容。ztui 不是终端转义序列过滤器。

示例 `examples/fixed_buffer.rs` 展示固定数组输出：缓冲不足时返回错误，库不申请堆内存。

## 与 SSMCP 的集成

SSMCP 的 `terminal::Frame` 实现 `ztui::Frame`，保留现有行、布局键、光标和终端模式。帧差分实际交给 `ztui::write_diff`；普通会话行的 padding 使用 `ztui::Row`。SSMCP 继续负责事件折叠、Markdown、真实终端大小、输入、安全过滤、历史记录与主题。

Kitty 图像协议、图片解码、图层 placement 及清理属于上层独立媒体通道。它们不进入 ztui 的文本行，也不会给本库引入图像依赖。

## 验证与性能

```sh
cargo test -p ztui-core
cargo run -p ztui --example fixed_buffer
cargo bench -p ztui-core --bench diff -- 50000
```

基准覆盖首次画面、不变画面、单行变化、全部变化、尺寸变化、光标变化和模式变化。它先与 SSMCP `97c026c` 的旧算法逐字节比较，再计时；两边都复用调用方的 `String` 缓冲，每组先热身，第二轮交换执行顺序。输出为逐行 JSON，便于保留机器信息和结果。

这只测帧差分与字符串写入，不包含 Unicode 测量、事件解析、终端绘制、网络或 GPU。库自身不分配内存，不代表应用或终端没有成本。没有基准结果时不声称更快；不同机器的计时应重复比较。

详细输出约定见 [docs/contracts.md](docs/contracts.md)。需要 Rust 1.100 或更新版本，许可证为 MIT。

The registry package is `ztui-core`; the Rust library name remains `ztui`. The `ztui` registry name belongs to an [existing ZeroTier TUI project](https://crates.io/crates/ztui).
