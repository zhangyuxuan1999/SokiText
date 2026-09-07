# 05 · 前人经验

> 调研方式：完整读了 xi-editor 的官方复盘原文（作者博客被出口封锁，改从他 blog 仓库抓的 raw markdown），
> 并且在本机 clone + 实际统计了 helix / zed / kakoune / micro / kibi / kilo 的代码量，用数字而不是印象说话。

## 最重要的一句话

**没有一个项目是因为选错了文本数据结构而死的。死掉或停滞的，全都是因为把"创新点数"花在了架构上，
而那个架构的回报永远差一次重构。**

## xi-editor（2016~2020，Google 的 Raph Levien）——最值钱的一份反面教材

作者自己列出他花掉的五个"创新点数"：Rust 核心、rope、**多进程架构**（前端和插件各自一个进程）、
**彻底拥抱 async**、**用 CRDT 做并发修改**。结论是这套系统"相当复杂，需要的工作量远超必要"。

三句原话，直接决定 jio 的架构：

> "我现在坚定地认为，前端和核心的**进程分离不是个好主意**。"

> "**async 是复杂度的乘数。**"

> "太多时候事情卡在某个大的架构重做上（'我们得先重做插件 API，你才能实现那个功能'）……
> 所以我认为**单体架构，也许挺讽刺的，对社区更好**。"

**坏掉的功能恰恰是最普通的那些**，不是什么高级特性：

- **带折行的实时窗口 resize** —— 编辑和重排之间的竞态与撕裂
- **滚动**（取前端缓存之外的文本）—— "花了几个月才做对"；作者补了一句：
  "如果文本就是一个 UI 可以直接查询的**进程内数据结构**，这本来会相当直接。"

这两个，正是 jio 第一天就要有的功能。

CRDT 的结论："我得出结论，**CRDT 没有撑起它（相当可观的）重量**"，
他看到"一个更简单的、基本同步的模型有远为光明的未来"。

还有一条关于野心蔓延的：

> "如果我一开始就说这项工作包含从零写一个 GUI 工具包，人们会很正当地嘲笑这个范围太狂妄。但事情就是这么发展的。"

那个工具包（Druid）现在已经"**项目已终止**"，继任者 Xilem 在 2026 年仍自称"实验性"，且要求 Rust ≥1.96
（本机是 1.94.1）。

**xi 最受欢迎的功能请求 —— vi 键位 —— 从来没有被合并。**

## Helix —— 最接近 jio 的正面参照，但也是停滞的活例子

- 13 个 crate、107,479 行 Rust、337 个锁定依赖，外加 1,193 个 tree-sitter query 文件和 5,657 行 `languages.toml`。
  **46.1k star、7,689 commit、大量贡献者——不是一个人。**
- **要抄的东西（最有价值的单个构件）**：它的集成测试**完全无头**地跑真编辑器 ——
  构造 `Application`，把 `parse_macro("ihello world<esc>")` 产生的合成 `Event::Key` 推进一个 tokio channel，
  调 `app.event_loop_until_idle(&mut rx_stream)`，然后断言 `doc.text()` + `doc.selection(view.id)`。
  **没有 PTY，没有 TTY。** 而且它的 harness 有个 `LineFeedHandling::Native` 模式，
  会在 Windows 上把所有期望字符串重写成 CRLF。这正是"零 macOS/Windows 硬件也能验证 macOS/Windows"的模式。
- **要抄的第二个东西**：`Selection`/`Range`/`Transaction`(ChangeSet) 代数。
- **一个必须知道的事实**：**Helix 至今在 Windows 上仍然单独依赖 crossterm 0.28**
  （`[target.'cfg(windows)'.dependencies]`），termina 只用在其它平台。连它自己的测试 helper 都是
  `#[cfg(windows)] use crossterm::event::...` / `#[cfg(not(windows))] use termina::event::...`。
  **它切 termina 的 PR 合并一年后，Windows 那条路径还没删掉。**
  所以"termina 解决了 Windows 终端问题"这个说法，旗舰用户自己都没验证过。
  全代码库有 73 处平台条件编译（只有 6 处是 macOS 专属），碰 Windows 的文件是：
  clipboard、path、uri、faccess、env、语法加载、line_ending、application、补全路径处理。
- **CI 矩阵**：`[ubuntu-latest, macos-latest, windows-latest, ubuntu-24.04-arm, windows-11-arm]`，
  五个上面都跑 `cargo test --workspace` 和 `cargo integration-test`，MSRV 单独一个 job 钉在 1.90。
- **⚠️ 停滞的部分**：插件系统 PR #8675（Steel）**从 2023-10-31 开到 2026 年 8 月还是 draft**，
  491+ commit、700+ 评论。维护者放弃了 WASM（"三次尝试都没走通"）、拒绝了 Rhai（"太新且 bus factor 是 1"）、
  部分基于 Neovim 的经验拒绝了 Lua。**与此同时，Helix 最后一个 tag 是 2025-07-18 的 25.07.1 —— 14 个月没发版**，
  尽管 master 一直有提交。
  **教训：一个健康的、46k star、多人维护的项目，花了近三年也没能落地插件系统，并为此停止了发版。**

## Zed —— 野心的定价表

在本机实测统计：**1,611,986 行 Rust、244 个 crate、1,815 个锁定包**。

- GPUI（自研 GPU UI 框架）家族：88,619 + linux 15,391 + windows 12,524 + macos 10,510 + apple 2,200 + wgpu 4,944
  ≈ **14.3 万行，比整个 Helix（10.7 万行）还多**。
- 而它真正的文本引擎只有：rope 4,405 + text 6,592（CRDT）+ sum_tree 3,327 ≈ **1.43 万行**。
  `editor` crate 反而是 171,571 行。
- 时间线：2019 年开始 → 2023 仅 macOS 公开发布 → 2024 Linux → **2025 年 10 月才有 Windows** → 2026-04-29 才 1.0。
  **有钱、有团队，跨平台仍然花了这么久，且成熟度至今 macOS > Linux > Windows。**

**结论：文本引擎很便宜，UI 框架极贵。别自己写 GUI 工具包。**

## Neovim —— 进程边界放对地方的正面例子

进程边界在**表现层**，不在模型层：UI 协议是一个笨拙的字符网格
（`grid_line`/`grid_resize`/`grid_scroll`，约 40 个事件，`ext_*` 是可选加入的，
协议明确写着"客户端必须准备好忽略这类扩展，以保证前向兼容"），
而**所有编辑逻辑都在一个进程里**，跑在 libuv 事件循环上。
连内置 TUI 也是到 0.9 才移出主进程的，而且官方明确警告这"可能导致行为的细微变化和 bug"。

## Kakoune —— 数据结构无关紧要的证据

38,888 行 C++，15 年历史，广受尊重，仍在发版（2026.05.21）。
它的缓冲区是 `Vector<StringDataPtr>` —— **一个行数组，不是 rope**。
它用 client/server 分离（每个 UI 客户端是独立进程，对一个持有缓冲区的 server）。
它明确拒绝 Windows：**"由于 Kakoune 严重依赖类 Unix 环境，不计划做原生 Windows 版本。"**

→ 值得抄的是它的**选区优先（selection-first）编辑模型**；不值得抄的是它靠 shell-out 实现功能的哲学，
那正是它对 Windows 不友好的原因。

## VS Code —— 两个架构上的胜仗

1. 用 **piece tree** 取代行数组缓冲区，加载后内存接近原文件大小
   （在 14MB/55.2 万行的词典和 54MB/300 万行的 heap snapshot 上做的基准）。
2. **扩展跑在独立的 extension host 进程**里，带约 3 秒的响应看门狗，
   所以"行为不端的扩展不能影响 VS Code"，UI 在扩展宿主挂掉后还能活。

→ 注意：VS Code 的进程边界是围绕**不可信的扩展**画的，和 xi 围绕**模型**画的边界完全不是一回事。

## 极小核心 —— 跨平台编辑器可以很小

| 项目 | 规模 | 备注 |
|---|---|---|
| antirez 的 **kilo** | **1,308 行 C** | 不依赖任何库，连 curses 都不用 |
| **Kibi** | **2,652 行 Rust** | 支持 Linux / macOS / Windows 10 1703+ / WASI。**整个 Windows 平台层只有 75 行**（开 `ENABLE_VIRTUAL_TERMINAL_INPUT` 和 `ENABLE_VIRTUAL_TERMINAL_PROCESSING`，查一下控制台屏幕缓冲区拿尺寸） |

**jio 的 v0.1 应该更接近 Kibi，而不是接近这份调研里任何一条建议的并集。**

## 单人编辑器的真实死法：注意力衰减，不是架构

| 项目 | 最后活动 |
|---|---|
| **Ox**（Rust，Lua 可配置，功能挺全） | 最后提交 2025-03-13 |
| **Zee**（ropey + tree-sitter） | 最后提交 2025-02-06 |
| **Amp** | 还活着但很慢（2026-06-10） |
| **Xray**（GitHub 官方的 Rust+CRDT+WebGL 编辑器） | 2019-07-23 归档，"GitHub 决定不再推进本项目的任何方面" |

**失败模式是一个 6 个月的空档，不是一个错误的数据结构。**
凡是拉长"到第一个可安装可用的二进制"这段时间的东西，都直接放大这个风险 —— 而这份调研里的大部分建议都会拉长它。

## Micro —— 易上手路线

约 25,254 行 Go，朴素的 LineArray 缓冲区，gopher-lua 插件 + 内置插件管理器，单静态二进制，
合理的非模态默认键位，最新版 2.0.15（2025-12-31）。
代价：它现在归社区组织维护，并且**自己 fork 并维护了终端库**（`github.com/micro-editor/tcell/v2`）和 json5 ——
"不要自己写终端库"的另一面是"你可能被迫维护别人的"。

## 扩展性是唯一能活过一切的护城河（但要放到最后做）

Emacs 31.1 于 2026-08-24 发布（项目约 50 岁）；Vim 9.2 于 2026-02-14 发布，
到 2026-08-22 已打到 9.2.0995 —— **Vim 在 Bram Moolenaar 去世后仍在继续，正因为它的价值活在脚本和社区里，不在它的 C 代码里。**
Neovim 的整个论点就是：Vim 约 30 万行 C89 的单一维护者瓶颈才是真正的缺陷。

**但这不意味着现在就做插件系统。** 便宜的形式是：
**从 Phase 1 起，把每一个用户可见的动作都路由过一个内部命令注册表 + TOML 声明式键位映射。**
这几乎不花钱，而且这才是 Neovim/Emacs 真正的教训。脚本运行时（如果做的话）等一年后 jio 还活着再说。

## 永久禁止清单

- ❌ 独立前端进程（作者亲口否定）
- ❌ CRDT（同一位作者亲口否定）
- ❌ 自己写 GUI 工具包（Druid 已死；GPUI 家族约 14.3 万行）
- ❌ 自己写 rope（Zed 整个文本引擎才 1.43 万行，而 ropey 是免费的）
- ❌ 自己写终端库（micro 最后被迫 fork 并维护 tcell）
- ❌ 靠 shell-out 实现功能（这正是 Kakoune 对 Windows 不友好的原因）
