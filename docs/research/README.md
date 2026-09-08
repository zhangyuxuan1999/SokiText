# SokiText 调研报告（2026-09-07）

> 调研方法：11 个并行 agent，8 个方向的文献/一手资料调研 + 2 个在本机做**真实编译与测量**的实证 agent
> + 1 个对抗性审稿 agent（专门找前面十个的错）。原始输出见 [`raw/research-2026-09-07.json`](raw/research-2026-09-07.json)（419 KB）。
>
> 报告里凡是标 **[实测]** 的，都是在这台开发机上跑出来的命令输出，不是网上抄的。
> 有几条广为流传的说法被实测推翻了，见 [02-measurements.md](02-measurements.md)。

## 目录

| 文件 | 内容 |
|---|---|
| [01-stack-decision.md](01-stack-decision.md) | 技术选型：语言、形态、数据结构、每个组件选哪个库 |
| [02-measurements.md](02-measurements.md) | 本机实测数据 + 被推翻的常见说法 |
| [03-cross-platform-hazards.md](03-cross-platform-hazards.md) | 会**静默毁掉用户文件**的跨平台陷阱清单 |
| [04-verification-strategy.md](04-verification-strategy.md) | 没有 mac/Windows 怎么保证不发出坏版本：测试金字塔 + CI/发布流水线 |
| [05-prior-art.md](05-prior-art.md) | 前人经验：xi-editor 复盘、Helix、Kakoune、Neovim、Zed、VS Code |
| [06-open-questions.md](06-open-questions.md) | **需要拍板的问题**（Q2/Q3/Q4 已定，剩 Q1 编辑模型阻塞开工） |
| [07-language-tradeoffs.md](07-language-tradeoffs.md) | 六个语言的完整利弊对比（应要求单独写的） |

---

## 一页纸结论

### 这个项目的真正约束不是技术，是"你看不见 macOS 和 Windows"

所有决策都从这一条推出来：**在 CI 里验证不了的设计 = 会带着 bug 发版的设计**。
这条约束否决了 GUI（本机连跑都跑不起来），否决了多进程架构（跨进程异步 bug 是最难在只有 CI 日志时调的），
并且直接决定了代码结构（纯函数核心 + 极薄 IO 外壳）。

### 选型

| 维度 | 结论 | 一句话理由 |
|---|---|---|
| **语言** | **Rust** | [实测] 带 tree-sitter C 语法的 339 crate 依赖集，能从这台 Linux 机器交叉编译出**真的 macOS arm64 / x86_64 / universal2 二进制**；Go 的 cgo 交叉到 macOS 需要真 SDK，做不到 |
| **形态** | **终端 TUI，单进程** | GUI 在这台机器上装不起来也跑不起来（你自己都用不了自己的编辑器）；多进程是 xi-editor 亲口承认的失败 |
| **架构** | 纯函数核心 + 100~150 行 IO 外壳 | 让 90% 的行为变成三个平台上完全一样的 `cargo test`，这是无硬件开发的唯一出路 |
| **文本结构** | Rope（ropey 1.6.1 或 crop 0.4.3），包在自己的 `Buffer` 类型后面 | [实测] String 在 100MB 文件上比 rope 慢 **1143 倍**（2.4ms/次插入）；包一层让这个选择可逆 |
| **编辑模型** | Helix 式 `Selection`/`ChangeSet` 代数 | 撤销、多光标、未来的协同编辑变成同一套机制，而不是三套 |
| **终端库** | crossterm 0.29 起步，藏在自己的 `Backend` trait 后面 | 它是 Windows 上唯一稳的（Helix 至今在 Windows 上仍单独依赖 crossterm）；termina 更好但连 Helix 都没敢在 Windows 上全切 |
| **渲染** | ratatui 0.30.2 | 它的 `TestBackend` 是你在无头机器上最快的验证回路，本身就是选它的理由 |
| **主测试手段** | ratatui TestBackend + insta 快照（约 90%） | 三个 OS 上字节级一致，毫秒级，无需终端 |
| **发布** | GitHub Actions 8 target 矩阵 + **smoke job** | smoke job 把每个产物下载到它声称支持的 OS 上真跑一次 —— 这是你没有 Mac 的唯一替代品 |

### 分阶段路线（防止项目死掉）

前人项目最常见的死法**不是选错数据结构，是在能编辑文件之前就耗光了热情**（Ox 停在 2025-03，Zee 停在 2025-02，
Helix 为了插件系统 14 个月没发版）。所以顺序是：

- **Phase 0（几天）**：先搭测试骨架，再写编辑器。`soki-core` 纯函数 + 可注入事件流的 `event_loop_until_idle`，
  第一个 commit 就把三平台 CI 矩阵立起来。**测试骨架如果不是第一个做的，就永远不会做。**
- **Phase 1**：无聊但正确的核心。rope + 光标 + 撤销 + `LineEnding` 一等公民类型。不上 tokio，不上 tree-sitter。
- **Phase 2**：**发布一个人能用的版本**。打开/编辑/保存/撤销/搜索/退出 + 三平台预编译二进制。
  参照物：Kibi 用 2652 行做完了整个跨平台编辑器，kilo 1308 行。
- **Phase 3**：编辑模型上花创新预算（多选区），tree-sitter 高亮。
- **Phase 4**：扩展性放最后，且先用便宜的形式（命令注册表 + TOML 声明式键位映射，从 Phase 1 就开始，几乎不花钱）。

**永久禁止清单**（来自前人尸检）：不做独立前端进程、不做 CRDT、不自己写 GUI 框架、不自己写 rope、
不自己写终端库、不靠 shell-out 实现功能。

### 最大的三个风险

1. **范围失控**。这份调研本身就是最大的风险来源——把八个方向的建议全做了，是一年之后才能编辑文件。v0.1 应该更接近 Kibi 而不是 Helix。
2. **macOS 只验证了"能编译"，从没验证过"能跑"**。这次调研产出的所有 macOS 二进制，一个都没被执行过。
3. **保存/编码路径会静默毁文件**。[实测] 三个"看起来对"的默认写法都会毁数据，见 [03-cross-platform-hazards.md](03-cross-platform-hazards.md)。

### 已拍板（2026-09-08）

| 问题 | 结论 |
|---|---|
| **名字** | **SokiText**，二进制名 `soki`。`sokitext` / `soki` 在 crates.io 均可用（已核实）。原 `jio` 的商标与占名问题随改名消除 |
| **语言** | **Rust**。项目采用 vibe coding，开发者的语言熟练度不再是变量；反而因为"没人逐行 review"，编译期安全的价值被放大 |
| **License** | 暂缓。后果：Homebrew / Scoop / crates.io / 发行版打包在加 License 之前都做不了，只能提供 GitHub Release 裸二进制 |

### 还需要拍板的

1. 🔴 **Q1 按键怎么工作**（唯一还阻塞第一行代码的）：像记事本那样直接打字，还是像 vim 那样分模式，
   还是像 Helix 那样"先选后动"？—— [用大白话解释在这里](06-open-questions.md#-q1-按键怎么工作唯一还阻塞第一行代码的问题)
2. 🟠 **Q5 要不要 tree-sitter**：这是唯一能推翻"用 Rust"的问题
3. 🟠 **Q6 v0.1 到底做什么**：没有这个清单就没法判断任何子系统该不该进 v1

详见 [06-open-questions.md](06-open-questions.md)。
