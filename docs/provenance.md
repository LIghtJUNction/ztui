# 导出来源与验证边界

## 固定来源

独立仓库通过 `git subtree split --prefix=crates/ztui` 从以下固定版本导出，随后只调整仓库元数据和文档：

| 项目 | 值 |
|---|---|
| 来源仓库 | https://github.com/LIghtJUNction/ssmcp |
| 来源提交 | `3570af1507031e83eede4abeb0853e609f039f46` |
| 来源目录 | `crates/ztui` |
| 来源目录 Git tree | `56c69a513ec8920ac3667f832cdfd50199d705dd` |
| subtree split 提交 | `15e7b5d5174273ab34438af1727db20991500b52` |
| 导出提交 Git tree | `56c69a513ec8920ac3667f832cdfd50199d705dd` |
| `src/` Git tree | `fb49cbbcefdb5b89bfddf28775ef9a1daee1680e` |

两个目录 tree 完全相同，说明导出时 12 个文件的路径、模式和内容一致。独立仓库的后续准备提交只修改 `Cargo.toml` 的 repository 地址与 README，新增本记录、重复测量 JSON 和忽略构建目录的规则；不修改 `src/`、`tests/`、`benches/`、`examples/` 或 MIT 许可证。

subtree 保留了三条涉及该目录的历史提交，按新到旧排列：

- `15e7b5d5174273ab34438af1727db20991500b52` — merge: reconcile reviewed PRs 25, 26 and 30
- `352084bd97ec18e19d083720a331542c38321244` — feat(client): integrate ztui, native images and terminal dialogue reminders [skip ci]
- `40909ec584977cf565beb2e7d275d1ab876d51b3` — feat: add no_std display primitives and gateway guide

这些是重写目录根路径后的 subtree 提交，不应与原 SSMCP 提交 SHA 混用。

## 既有验证

SSMCP R14 在固定提交 `aa8b95d66655adfd4d97ac8191e2b935bcd57c44` 上完成 workspace 测试、严格 Clippy、workspace 打包与优化构建。公开报告的默认 Rust 汇总为 1,142 passed、26 ignored、0 failed，范围包含整个 SSMCP workspace，**不是 ztui 自身的测试数量**。

[固定 R14 验证报告](https://github.com/LIghtJUNction/ssmcp/blob/a83cb4955ba6865bc7fb10077ea4be150c2fc725/docs/reports/ssmcp-iteration-20261118-validation.json)

本次导出检查目录 tree、源码字节、元数据、文档链接及 Git 差异，不把既有 workspace 结果当作独立仓库提交的新一次构建。独立检出后的测试和正式发布是后续分别记录的步骤；没有随仓库创建执行 Cargo、发布 crate、创建 tag 或启动远端 CI。

## 性能证据

重复测量来源提交为 `343163e0b5e8d6d43e6f854d49adbaa913c24c67`。该版本与导出来源的 `crates/ztui/src` tree 都是 `fb49cbbcefdb5b89bfddf28775ef9a1daee1680e`。性能表引用这份既有实验，不把独立仓库的新提交标作测量来源。

[本地证据副本](performance-repeat-20261118.json) 原样来自公开的[固定证据 JSON](https://github.com/LIghtJUNction/ssmcp/blob/a83cb4955ba6865bc7fb10077ea4be150c2fc725/docs/reports/ssmcp-iteration-20261118/ztui-performance.json)。原始报告 SHA-256 为 `dea76fc32d373653d5bc8f77fb1213b3b9a581d48fb5b61cc2570e45bc01cf54`；公开副本已经去掉本机绝对路径，保留原数值。

3 次基准进程分别执行七个场景。每个场景、每种实现执行两轮各 200,000 次，第二轮交换顺序；计时前各做一次字节等价比较。因此，21 是固定场景等价检查次数，每种实现、每个场景累计计时调用 120 万次。统计口径是每次运行的 ztui/原实现配对比值的中位数。

完整记录保留较慢场景、首次尺寸变化 3.3873 比值及环境负载。不包含终端、GPU 和端到端交互，不以库内部零分配推导进程 RSS，也不将不同 run 的绝对耗时中位数相除冒充配对比值。

## 可复查的声明

| 声明 | 直接证据 | 边界 |
|---|---|---|
| 库使用 `no_std`，禁止 unsafe，零第三方依赖 | `src/lib.rs`、`src/*.rs`、`Cargo.toml` | 应用输出和测宽函数不在此承诺内 |
| 导出未修改运行时、测试、示例与基准 | 固定 source/split tree 与文件哈希核对 | 文档和仓库地址有明确调整 |
| 性能有快有慢 | 七场景三次运行的完整 JSON | 固定 fixture，非所有终端或完整应用 |
| 既有 R14 验证通过 | 固定 SSMCP workspace 报告 | 不冒充导出后独立验证 |


## Consolidated frame writer

The frame writer and contract tests include the fixed ANSI control-sequence
formatting and equal-row fast path from SSMCP commit 4e3132d60600dca5053af81cdefa6962c3058ac4.
The public geometry, row and frame APIs remain unchanged; the package and
Rust import name are `ztuio`. The contract suite compares emitted bytes at
all decimal coordinate boundaries, including the full u16 row range.
