# 架构决策记录（ADR）

一个决策一个文件。格式：背景 → 决策 → 后果 → 什么会推翻它。

状态：`proposed`（待拍板） / `accepted`（已定） / `superseded by NNNN`（被取代）

| # | 决策 | 状态 |
|---|---|---|
| [0001](0001-language-rust.md) | 用 Rust | proposed（依赖 [Q4](../research/06-open-questions.md#-q4-你写过-rust-吗) ） |
| [0002](0002-single-process-tui.md) | 单进程终端 TUI | accepted |
| [0003](0003-pure-core-thin-shell.md) | 纯函数核心 + 极薄 IO 外壳 | accepted |
| [0004](0004-rope-behind-newtype.md) | Rope 藏在自己的 Buffer 类型后面 | accepted |
| [0005](0005-in-place-save.md) | 默认就地写入保存 | proposed |
