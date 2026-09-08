# SokiText

一个跨平台（Linux / macOS / Windows）终端文本编辑器。

**当前状态：调研完成，等待开工。**

- 📋 [调研报告](docs/research/README.md) —— 技术选型、实测数据、跨平台陷阱、验证策略、前人经验
- 🧭 [架构决策记录](docs/decisions/README.md)
- ❓ [待拍板的问题](docs/research/06-open-questions.md)
- 📓 [开发会话日志](notes/SESSION-LOG.md)

## 已定的事

| | |
|---|---|
| 语言 | **Rust** — [为什么](docs/research/07-language-tradeoffs.md) |
| 形态 | 终端 TUI，**单进程** |
| 二进制名 | `soki` |
| 平台 | Linux / macOS / Windows，全部通过 GitHub Actions 验证 |

## 还没定的事

- **按键模型**：非模态 / vim 式模态 / Helix 式先选后动 —— 这个还阻塞着第一行代码
- **要不要 tree-sitter**（这是唯一能推翻"用 Rust"的问题）
- **v0.1 的功能清单**
- **License**（暂缓；在加之前无法通过 Homebrew / Scoop / crates.io 分发）
