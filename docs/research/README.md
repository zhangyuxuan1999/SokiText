# jio 调研报告（2026-09-07）

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
| [06-open-questions.md](06-open-questions.md) | **需要你拍板的问题**（编辑模型、名字、License） |

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

- **Phase 0（几天）**：先搭测试骨架，再写编辑器。`jio-core` 纯函数 + 可注入事件流的 `event_loop_until_idle`，
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

### 需要你拍板的（我不替你决定）

1. **编辑模型**：模态（vim/helix）还是非模态（nano/VS Code）？这个决定在 rope 之上游，且和终端约束冲突——
   GNOME Terminal（Ubuntu/Fedora 默认）**根本发不出 Ctrl+Shift 和 Ctrl+数字**，非模态编辑器在多数 Linux 终端上会不够键位用。
2. **名字**：`jio` 在 crates.io **已被占用**（0.0.0，2024-08-15），且 "JIO" 是 Reliance 的驰名商标、有起诉个人开发者的记录。
3. **License**：现在仓库没有 LICENSE 文件 = 保留全部权利 = 没人能贡献、没法被打包分发。

详见 [06-open-questions.md](06-open-questions.md)。
