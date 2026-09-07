# 01 · 技术选型

## 1. 语言：Rust

### 决定性证据（不是偏好，是实测）

带 tree-sitter 的**真实依赖集**（339 个 crate：tokio、tree-sitter 0.27.0 + 三个 C 语法、ratatui、ropey、arboard、nucleo）
在这台无头 Linux 机器上交叉编译结果：

| 目标 | 结果 | 产物验证 |
|---|---|---|
| `x86_64-unknown-linux-musl` | ✅ | `ELF 64-bit, static-pie linked` |
| `aarch64-unknown-linux-gnu/musl` | ✅（需 cargo-zigbuild） | `ELF 64-bit ARM aarch64` |
| `x86_64-pc-windows-gnu` | ✅ | `PE32+ executable (console) x86-64` |
| `aarch64-apple-darwin` | ✅（需 cargo-zigbuild） | `Mach-O 64-bit arm64`，platform=1、sdk=26.4、minos=13.0 |
| `x86_64-apple-darwin` | ✅ | `Mach-O 64-bit x86_64` |
| macOS universal2 | ✅ | `Mach-O universal binary with 2 architectures` |
| `x86_64-pc-windows-msvc` | ❌ | 缺 `link.exe`；cargo-xwin 因代理封锁 `aka.ms` 也不行 → **MSVC 只能在 CI 上** |

tree-sitter 的 C 运行时确实产出了**真正的 Mach-O arm64 目标文件**和 Windows COFF 目标文件，
darwin target 下编译出 173 个 rlib。也就是说交叉编译里最难的一档（带 build.rs 代码生成的 C 库）是通的。

**Go 在同一件事上失败**：

```
$ GOOS=darwin GOARCH=arm64 go env CGO_ENABLED
0                              # 交叉编译时静默关掉 cgo

$ CC='zig cc -target aarch64-macos' GOOS=darwin CGO_ENABLED=1 go build
error: unable to find dynamic system library 'resolv' using strategy 'paths_first'
```

Go 运行时要求 `-lresolv` 和 `-framework CoreFoundation`，必须有真的 macOS SDK。
而两个还在维护的 Go tree-sitter binding **都是 cgo**。所以选 Go 就意味着：要么放弃 tree-sitter，
要么 macOS 变成一个你在本地永远无法复现的构建目标——在"哪儿都没有 Mac"的前提下，这是复合风险而不是小麻烦。

> 注意：Go+cgo 交叉到 **Windows** 是能成的（zig cc 可以）。Go 的 cgo 问题是 macOS 专属，不要以讹传讹。

### 次要但真实的优势

- **Unicode 版本领先两个大版本**。unicode-segmentation 1.13.3 / unicode-width 0.2.2 都是 Unicode 17.0.0，
  带 Indic conjunct break（GB9c）；Go 的 rivo/uniseg v0.4.7 冻结在 15.0.0。
  [实测] 对 `हिन्दी` 分割：Rust 得 2 个字形簇，Go 得 3 个。**同一个光标键在两种语言里落点不同**。
  Go 标准库压根没有字形簇分割（`go doc unicode | grep -ci grapheme` = 0）。
- **有生产级 rope**。ropey 1.6.1 在 125MB / 177 万行的文件上：构建 137ms，随机单字符插入 1.91µs。
- **二进制反而更小**。[实测] 同一个 ~160 行 TUI：Rust 调优后 550,816 B，Go 最佳 2,486,456 B —— **Rust 小 4.5 倍**。
  连 Rust 未调优的默认 release（931,832 B）都比 Go 最佳小 2.7 倍。这和通常的印象相反。

### 必须诚实说明的：三个常见的"选 Rust 的理由"是假的

1. **"Go 的 GC 停顿会毁掉编辑器"——不成立。** [实测] 176 万个存活指针对象、GOGC=50、300 万次撤销记录分配下，
   Go 的 STW 停顿中位数 0.094ms / 最大 0.182ms，远在一帧预算之内。要拒绝 Go 请用 cgo 和 Unicode 的理由，别用 GC。
2. **"Rust 不能交叉编译，那是 Go 的杀手锏"——对 Windows 而言是错的。** [实测] `cargo build --target x86_64-pc-windows-gnu`
   14.27 秒产出 454,656 字节的 exe，wine 能跑。Rust 真正的缺口是 msvc + apple-darwin + aarch64-linux，不是"Windows"。
3. **"macOS 交叉编译必须有 SDK / osxcross"——2026 年对 Rust 项目已经不成立。** `pip install ziglang` + cargo-zigbuild
   就产出了带正确 LC_BUILD_VERSION 的真 Mach-O，连 arboard 的 objc2 桥接和 notify 的 fsevent-sys 都编过了。

### Rust 的代价（诚实版）

| 项 | Rust | Go | 差距 |
|---|---|---|---|
| 冷编译（debug） | 15.65s | 7.77s | Go 快 2.0× |
| 增量编译（debug） | 0.39s | 0.07s | Go 快 5.6× |
| 增量编译（release+LTO） | 5.2s | — | **别把 LTO 放进 `[profile.release]`**，每次编辑都要付 13 倍代价 |
| `target/` 体积 | 单 target 148MB，8 个 target 共 1.2GB | 整个项目含 7 个交叉产物 31MB | Go 小 40× —— 这是 **CI 缓存**问题，不是好奇心问题 |

**最大的排期风险不在上面这张表里**：借用检查器 vs 编辑器天然的环形数据模型（buffer ↔ view ↔ cursor ↔ undo）。
Helix 的解法是一个巨大的 `Editor` struct + 整数索引，Zed 不得不发明 GPUI 的 `Entity<T>`。
**如果你还没写过 Rust，这一条比其它所有因素加起来都更能决定项目死活**——见 [06-open-questions.md](06-open-questions.md)。

### 什么会让我改主意

jio 决定**不上 tree-sitter**（只用正则或手写高亮）。那样 Go 的 cgo 问题消失，它 5.6 倍的迭代速度对单人项目就是压倒性的。
所以"要不要 tree-sitter"是一个**架构决策，不是功能决策**——它同时决定语言和交叉编译能力。

---

## 2. 形态：终端 TUI，单进程

### 为什么不是 GUI

不是审美问题，是约束问题：[实测] eframe/egui 是 38 倍的二进制体积、345 个 crate、5 分钟编译，
而且**在这台机器上 apt 装 X 库和 Mesa 之前根本跑不起来**。也就是说你写完了自己都用不了。
GUI 还是唯一一种在这个环境里完全无法验证的形态。

Tauri 更糟：Linux 端 webview 是系统 webkit2gtk 依赖，无头环境完全跑不了，而且 Tauri 官方文档承认
`tauri-driver` 没有 macOS 支持。Electron/Node SEA 直接被体积否决——hello-world 就 124.8MB。

### 为什么不是"核心 + 独立前端进程"（xi-editor 模型）

xi-editor 作者 Raph Levien 自己的复盘（两个 agent 分别抓取了原文）说：前后端分进程"**不是个好主意**"，
异步是"**复杂度乘数**"，而且**坏掉的恰恰是滚动和折行时的 resize**——正是 jio 第一天就要有的功能。

对照组：Neovim 和 VS Code 的进程边界都在**边缘**（前者是一个笨拙的字符网格协议，后者是隔离不可信扩展），
**不在模型层**。这是可以抄的；xi 的做法不可以。

### 但要保留 xi 的"内部形状"，只是不要 IPC

从第一个 commit 起就分成两个 crate：

- `jio-core`：rope、字形簇光标模型、撤销、选区、语法、LSP。**不许依赖 crossterm/ratatui/termina**，
  并且用 CI 强制：`cargo tree -p jio-core | grep -q crossterm && exit 1`
- `jio-tui`：ratatui + 终端后端

中间放**你自己的** `Backend` / `Capabilities` trait。这样以后换终端库是改一个模块，不是改整个项目。

### 终端现实（这些会直接影响键位设计）

- **GNOME Terminal（Ubuntu/Fedora 默认）发不出 `Ctrl+Shift+*` 和 `Ctrl+数字`**，而且 `Tab == Ctrl+I`、`Enter == Ctrl+M`。
  Kitty 键盘协议能解决，但支持它的终端是少数。**默认键位必须按最差的终端设计**，
  kitty 协议只用来解锁额外绑定，永远不承载默认绑定。模态编辑器之所以存在，很大程度上就是因为这个约束。
- **[实测] crossterm 的 kitty 键盘增强命令在 Windows 上不是 no-op，是硬错误。**
  `execute!(out, EnterAlternateScreen, PushKeyboardEnhancementFlags(...))` —— 这是大多数教程的写法 ——
  会让 jio **在 Windows 上根本启动不了**，报 `Keyboard progressive enhancement not implemented for the legacy Windows API`。
  而且 `cargo check --target x86_64-pc-windows-msvc` **完全检查不出来**。这个 bug 是靠 wine 跑出来的。
- **宽度是渲染提示，不是真理。** [实测] 一个 ZWJ 家庭 emoji：unicode-width 算 2 格、tmux 3.4 渲染 5 格、vt100 认为 6 格。
  12 个测试字符串里有 7 个三方不一致。**没有任何 CI 层能抓住这一类问题。**
  结构性缓解：内部按字形簇建模，每画一行都重发一次绝对定位 `CSI row;col H`，
  这样宽度分歧只毁掉一行的样子，而不会让整个屏幕错位。再加一个 `jio --width-report` 让用户能贴报告。

---

## 3. 文本核心

### Rope，且包在自己的类型后面

[实测] 100MB 文件、10000 次随机单字符插入：

| 结构 | 耗时 | 单次 | 峰值内存 |
|---|---|---|---|
| ropey 1.6.1 | 20.98 ms | 2.097 µs | 125 MiB（用 `from_reader`） |
| crop 0.4.3 | 28.82 ms | 2.882 µs | 208 MiB（harness 用了 `read_to_string`，非 crop 的锅） |
| jumprope 1.1.2 | 34.31 ms | 3.431 µs | 294 MiB |
| **std String** | **23.985 s** | **2.3985 ms** | 102 MiB |

String 慢 **1143 倍**——每秒只能承受约 400 次击键。这个差距比通常"rope 对大文件更好"的含糊说法大得多。

**ropey vs crop 没有明确赢家，所以这个决定必须做成可逆的：**

- crop 的架构优势是真的：**字节索引**（和 tree-sitter 的 `InputEdit`、LSP 的 utf-8 positionEncoding、ripgrep 结果、
  Rust 的 `str` 全都一致）。ropey 1.x 强制 char 索引，**字节/字符搞混会编译通过、在 ASCII 上正常、只在 CJK/emoji 上出错**
  ——正是那种没有硬件就发现不了的 bug 类别。另外 ropey 默认把 U+2028/VT/NEL/FF 也算换行，会让诊断行号静默错位
  （要用 `default-features = false, features = ["simd", "cr_lines"]`）。
- ropey 的优势也是真的：Helix 在三个平台上出货用的就是它，文档最好，随机插入更快，`from_reader` 省 42% 内存。
  crop 只有约 320 star，基本是单人维护。
- 局部编辑（真实打字模式）crop 更快（140ns vs 168ns），均匀随机插入 ropey 更快。**两组数据不矛盾，是不同工况。**

**结论：先用 ropey 1.6.1（无聊即安全），但所有 rope 调用点只能出现在一个模块里，对外只暴露字节偏移。**
换成 crop 应该是约 200 行的改动。在没有 mac/Windows 的情况下，这层包装不是可选项，它就是"让决策可逆"本身。

### 光标 / 选区 / 编辑：抄 Helix 的代数

```rust
Range { anchor: usize, head: usize }                        // 半开区间，与 anchor/head 顺序无关
Selection { ranges: SmallVec<[Range; 1]>, primary_index: usize }
ChangeSet { Retain(n) | Delete(n) | Insert(text) }          // 所有修改都走它
```

`apply` 改 rope，`invert` 得到撤销，`compose` 合并，`map_pos(pos, Assoc)` 把光标、标记、诊断、折叠状态
**用同一段代码**变换过去。这是整份报告里杠杆最高的一个设计决定：**它让撤销、多光标、以及未来可能的协同编辑变成一套机制而不是三套。**
多光标于是免费，vim 式单光标只是 `ranges.len() == 1`。

不要用 VS Code 那种"光标是一个独立列表，每次编辑单独打补丁"的做法——排序 bug 一定会写错。

撤销用**反转 ChangeSet 日志 + 打字突发合并**，不要每次击键存一个 rope 快照（[实测] 每个快照 2.5~3.1 KB，
2 万次击键就是 50~60 MiB）。

### 编码与换行：只在 I/O 边界处理

- rope 里**只存 UTF-8**，只存 LF。
- 打开时探测：① BOM → ② NUL 密度启发式（BOM-less UTF-16 会被当成合法 UTF-8 静默变成满是 NUL 的乱码！）
  → ③ 严格 UTF-8 校验 → ④ chardetng → ⑤ 配置的兜底编码。
- 在 `Document` 上记录 `(encoding, had_bom, line_ending_style, was_mixed)`，保存时原样还原。
- 无法无损解码的字节转义到 U+10FE00+byte（可精确往返），**绝不要用 U+FFFD**——那是不可逆的。

这样 CRLF 就变成一个**两点问题（加载/保存）**，CI 可以做字节级往返测试，而不是一个每次编辑都要维护的不变量。
这也是在没有 Windows 机器的情况下，唯一能保证"Linux 上编辑的 Windows 文件能原样还原"的办法。

---

## 4. 组件选型清单

| 组件 | 选择 | 理由 / 陷阱 |
|---|---|---|
| 终端后端 | **crossterm 0.29.0** 起步，藏在 `Backend` trait 后 | Windows 上唯一稳的。termina 0.4.0 更好（走 ConPTY/VT），但**Helix 至今在 `cfg(windows)` 下仍单独依赖 crossterm 0.28**——连旗舰用户都没敢在 Windows 上全切，你不该走在它前面 |
| 渲染 | **ratatui 0.30.2** | 别手写。它已经有你会写的那套双缓冲 damage tracking，而 `TestBackend` 是无头机器上最快的验证回路。代价：没有 undercurl（这是 Helix 当年 fork tui-rs 的原因） |
| ⚠️ ratatui 0.30 结构变了 | facade over `ratatui-core` 0.1.2 / `ratatui-widgets` 0.3.2 / `ratatui-crossterm` 0.1.2 | 任何 0.2x 时代的教程/记忆描述的都是另一个 crate 布局 |
| Rope | **ropey 1.6.1** `default-features = false, features = ["simd", "cr_lines"]` | 千万别用默认 features（会把 U+2028 当换行） |
| 字形簇 | unicode-segmentation 1.13.3 | Unicode 17.0.0 |
| 显示宽度 | unicode-width 0.2.2，**按字形簇算，绝不按 char 算** | 家庭 emoji：按簇=2，按 char 求和=8 |
| 语法高亮 | tree-sitter 0.27.0，**自己 vendor 语法的 C 源码**，用一个 build.rs 编 | **绝不要依赖 `tree-sitter-*` wrapper crate**——[实测] 它们会拉进冲突的 runtime 版本（tree-sitter-toml 0.20.0 拉 tree-sitter 0.20.10）。每个主流语法约 1MB |
| 高亮范围 | `QueryCursor::set_byte_range` 只高亮视口 | |
| 语法高亮备选 | syntect 5.3.0 **必须** `default-features = false, features = ["default-fancy"]` | [实测] 默认 features 拉进 C 库 Oniguruma（onig_sys），**这一个 build script 会搞垮整个交叉编译**，而 crate 名字和快速上手文档完全不提这事 |
| 搜索 | regex 1.13.1 + grep-searcher/grep-regex/ignore；fancy-regex 0.19.1 可选 | |
| 配置 | toml 读 + toml_edit 写（保留注释） | |
| 剪贴板 | **OSC 52 优先** + 内部寄存器永远可用；arboard 只作可选降级 | [实测] arboard 在无头机器上直接失败（X11 连接错误、1 秒卡顿），且**它会破坏 Linux→macOS 交叉编译**（`unable to find dynamic system library 'objc'`）。OSC 52 反而在 PTY 测试里完全可观测——把最难测的子系统变成 40 行的确定性测试。它也是 SSH 场景的正确答案 |
| 异步 | v1 不要 tokio | 阻塞读循环就够，还能消掉一整类测不了的竞态。等接 LSP 子进程时再引入，且只限于进程层 |
| 日志 | tracing + tracing-appender 非阻塞写文件 | TUI 下绝不能往 stdout 打日志 |
| 错误 | 内部 thiserror 2，边界 anyhow 1 | |
| 快照测试 | insta 1.48.0 | |
| PTY 端到端 | portable-pty 0.9.0 + vt100 0.16.2 | ⚠️ 见 [04-verification-strategy.md](04-verification-strategy.md) 里的 Windows 警告 |
| LSP | 自己写 JSON-RPC transport；类型库**先不定** | 生态正在三分裂：lsp-types 0.97.0 已冻结、tower-lsp 0.20.0 还钉在 0.94.1、async-lsp 钉 0.95.1、tower-lsp-server 0.23.0 干脆自己搞了一套。gen-lsp-types 100 天内发了 12 个 pre-1.0 版本——对单人项目是另一个问题，不是更小的问题 |
| Git | gix | [实测] 148 个 crate、236MB target 目录、68 秒编译。**Phase 3 之后再说** |
| ⚠️ helix-core | **不能用** | [实测] crates.io 上的 `helix-core` 是作者自己占位的 0.0.0，`src/lib.rs` 只有一行 `//! Placeholder`。想用 Helix 的代码只能走 git 依赖，那意味着 **MPL-2.0**。抄它的**设计**，别抄它的 crate |
