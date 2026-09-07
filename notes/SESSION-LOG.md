# jio 开发会话日志 / Session Log

> 这个文件是跨会话的"记忆"。每个 Claude Code 会话开始前先读它，结束前更新它。
> 开发容器是临时的（会被回收），所以任何有价值的中间产物都必须 commit 进这个仓库。

## 目录约定

| 路径 | 用途 |
|---|---|
| `docs/research/` | 调研报告（人读的结论） |
| `docs/research/raw/` | 调研过程的原始产物（agent 输出的 JSON、命令输出、实测数据） |
| `docs/decisions/` | ADR（Architecture Decision Record），一个决策一个文件 |
| `notes/` | 开发过程笔记、TODO、会话日志 |
| `notes/scratch/` | 临时脚本、基准测试脚本等（可以脏，但要能复现） |

## 时间线

### 2026-09-07 — Session 1：立项 + 调研

- 仓库初始状态：只有一行 `Readme.md`（"a text editor."）。
- 开发分支：`claude/cross-platform-text-editor-h0vslo`
- 关键约束：**开发者手头没有电脑**，全部开发在无 GUI 的 Linux 云容器里进行；
  macOS / Windows 只能通过 GitHub Actions runner 验证，没有任何手动测试的可能。
  => 这条约束会反过来决定架构：**不能在 CI 里验证的设计 = 会带着 bug 发版的设计**。
- 开发机环境：Ubuntu 24.04 x86_64 / 4 vCPU / 15GB RAM；
  rustc+cargo 1.94.1、go、node 22、python 3.11、gcc 13、clang 18、cmake 3.28；无 zig、无 dotnet。
- 网络：crates.io sparse index / npm / pypi / goproxy 可达；
  直接 curl github.com 与 crates.io HTTP API 被代理拦截（400/403），网页调研需走 WebSearch/WebFetch。

（后续进展见下方追加）

#### 调研执行情况

- 方法：11 个并行 agent —— 8 个方向调研 + 2 个在本机做真实编译/测量的实证 agent + 1 个对抗性审稿 agent。
  耗时约 2.7 小时，895 次工具调用，163 万 token。
- 产物：
  - `docs/research/` 六篇报告（索引见 `docs/research/README.md`）
  - `docs/research/raw/research-2026-09-07.json` 419KB 原始结构化输出（含全部 findings / 证据 / URL，
    报告里没展开的细节都在里面）
  - `docs/decisions/` 五个 ADR
- 已核实：仓库是 **public**（GitHub API `"private": false`）
  → macOS/Windows runner 免费且无限量。**这是整个验证策略的前提，仓库不能转私有。**

#### 本次得到的、最容易被遗忘的实测事实

1. `execute!(out, EnterAlternateScreen, PushKeyboardEnhancementFlags(..))` —— 大多数教程的写法 ——
   会让程序**在 Windows 上根本启动不了**（crossterm 0.29 在 Windows 上返回硬错误，不是 no-op），
   而 `cargo check --target x86_64-pc-windows-msvc` 检查不出来。
2. `tempfile::NamedTempFile` + persist 会把用户文件权限降成 0600、丢 xattr、断硬链接。
3. `encoding_rs::UTF_16LE.encode(s)` 返回的是 **UTF-8 字节**，`had_errors == false`。
4. ratatui 把双宽字符的第二格写成字面空格 → 手写逐格快照会把错误烤进 `.snap` 文件。
5. PTY 测试用固定 sleep 必然 flaky（负载下 10ms sleep 失败率 56%）；谓词轮询 0/180 失败。
6. 本机可以用 `pip install ziglang` + cargo-zigbuild 构建**真正的 macOS 二进制**，
   但**从未执行过任何一个** —— macOS 是"构建已验证、运行未验证"。

#### 下一步（等所有者拍板）

阻塞第一行代码的四个问题见 `docs/research/06-open-questions.md`：
编辑模型（模态/选区优先/非模态）、名字（`jio` 在 crates.io 被占 + 商标风险）、License、是否有 Rust 经验。
