# SokiText 开发会话日志 / Session Log

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
编辑模型（模态/选区优先/非模态）、名字（`SokiText` 在 crates.io 被占 + 商标风险）、License、是否有 Rust 经验。

### 2026-09-08 — Session 1（续）：改名 + 三个决策落地

**改名**：`jio` → **SokiText**。所有者已把 GitHub 仓库改名为 `zhangyuxuan1999/SokiText`
（repo id 1360584115 未变，仍是 public）。本地 remote 已更新。

命名约定：

| 用途 | 名字 |
|---|---|
| 项目 / 应用名 | SokiText |
| 可执行文件 | `soki` |
| crate | `soki-core` / `soki-render` / `soki-platform` / `soki`(bin) |
| 配置目录 | `~/.config/soki/` |
| Homebrew tap / Scoop bucket | `homebrew-sokitext` / `scoop-sokitext` |

名称可用性已核实（crates.io sparse index，2026-09-08）：`sokitext` `soki` `soki-text` 均为 404（可用）。

> ⚠️ `docs/research/raw/research-2026-09-07.json` **故意保持原样不改名** —— 它是调研当时的存档，
> 里面出现的 `jio` 是历史记录，不是笔误。

**已定的决策**：

| # | 决策 | 备注 |
|---|---|---|
| 名字 | SokiText / `soki` | 原 `jio` 的 crates.io 占名 + Reliance 商标风险随之消除 |
| 语言 | **Rust**（ADR-0001 转为 accepted） | 所有者说明是 vibe coding，自身语言熟练度不再是变量 |
| License | **暂缓** | 后果已写进 Q3：Homebrew/Scoop/crates.io/发行版打包在加 License 前都做不了 |

**vibe coding 对论证的影响**（写进了 ADR-0001 和 07-language-tradeoffs.md）：
"开发者会不会 Rust"这个因素消失，被"**没有人会逐行 review 代码，所以编译器和测试套件必须承担全部抓 bug 职责**"替代。
这条对 Rust 有利——编辑器最可怕的 bug 是静默改坏用户文件，那类 bug 在 Go 里能编译通过、测试通过，
只在真实用户的 GBK 文件/只读文件/软链接上出事。

**新增文档**：`docs/research/07-language-tradeoffs.md`（应要求写的六语言完整利弊对比）。

**Q1 已用大白话重写** —— 之前的问法太术语化，所有者反馈看不懂。

#### 下一步

只剩 **Q1（按键模型）** 阻塞第一行代码，其次是 Q5（要不要 tree-sitter）和 Q6（v0.1 功能清单）。
