# 07 · 语言选型：利弊分析

> 这一篇是应要求单独写的完整比较。所有标 **[实测]** 的数字都是在开发机上跑出来的
> （Ubuntu 24.04 / 4 vCPU / rustc 1.94.1 / 2026-09-07），不是网上抄的。

## 先说一个会改变权重的前提：这个项目是 vibe coding

开发者不打算亲自逐行写代码，所以"你会不会某个语言"这个因素**从等式里消失了**。
但它不是消失，而是**被另一个更重要的因素替代**：

> 既然没有人会逐行读代码抓 bug，那么**编译器和测试套件必须承担全部的抓 bug 职责**。

这一条会同时影响两个方向：

- **对 Rust 有利**：Rust 的类型系统在编译期就消灭了整类 bug（空指针、数据竞争、被忽略的错误返回值）。
  在没人 code review 的情况下，"编译不过"比"运行时才发现"值钱得多。
  编辑器最怕的 bug 是**静默改坏用户文件**——那种 bug 在 Go 里可以编译通过、测试通过、只在真实用户的 GBK 文件上出事。
- **对 Rust 不利**：Rust 的编译慢，AI 写代码的迭代循环也会变慢。
  但实测下来这个差距在这个项目的规模上不致命（见下表），而且可以靠配置规避掉最大的那一块。

另一个次要但真实的因素：**这个领域的 Rust 参考实现最多**。
Helix（10.7 万行）、Zed（161 万行）、Xi、Kibi 全是 Rust，都可以直接读。
Go 那边只有 micro（2.5 万行，行数组缓冲区，而且自己 fork 了 tcell）。
写代码的时候有一份高质量的、解决过同样问题的参考实现，价值很大。

---

## 六个候选的横向对比

| | Rust | Go | C/C++ | Zig | TS + Tauri | TS + Electron |
|---|---|---|---|---|---|---|
| **从本机能否交叉编译出 macOS 二进制**（带 tree-sitter） | ✅ 实测通过 | ❌ 需真 SDK | ✅（用 zig c++） | ✅ | ❌ | ❌ |
| **单文件免依赖二进制** | ✅ | ✅ | ✅ | ✅ | ❌ Linux 端依赖系统 webkit2gtk | ❌ |
| **二进制体积**（同一个 TUI） | **550 KB** | 2.49 MB | ~小 | ~小 | ~10 MB+ | **124.8 MB** |
| **能否在无头容器里跑起来** | ✅ | ✅ | ✅ | ✅ | ❌ | ❌ |
| **现成的 rope** | ✅ ropey/crop | ❌ 得自己写 | ❌ | ❌ | — | — |
| **Unicode 字形簇** | ✅ Unicode 17.0.0 | ⚠️ 第三方，冻结在 15.0.0 | ❌ 得自己接 ICU | ❌ | ✅ | ✅ |
| **tree-sitter** | ✅ 官方 Rust binding | ⚠️ 只有 cgo binding | ✅ 原生 | ⚠️ | ✅ WASM | ✅ |
| **内存安全** | ✅ 编译期 | ✅ 运行时(GC) | ❌ | ⚠️ 部分 | ✅ | ✅ |
| **冷编译 / 增量** | 15.6s / 0.39s | **7.8s / 0.07s** | 慢 | 快 | — | — |
| **生态成熟度（编辑器组件）** | ✅ 最好 | ⚠️ 一般 | ⚠️ 老但零散 | ❌ 几乎没有 | — | — |

---

## 逐个分析

### 🥇 Rust —— 推荐

**优点**

1. **交叉编译是决定性的。** [实测] 带 tree-sitter C 语法的 339 crate 真实依赖集，
   从这台 Linux 机器编译出了**真正的 Mach-O arm64 / x86_64 / universal2**（`platform=1, sdk=26.4, minos=13.0`，
   链接 `/usr/lib/libSystem.B.dylib`），以及 Windows PE。
   tree-sitter 的 C 运行时产出了真正的 Mach-O arm64 目标文件——
   **交叉编译里最难的一档（带 build.rs 代码生成的 C 库）是通的。**
2. **编辑器需要的东西全是现成的**：rope（ropey）、模糊匹配（nucleo）、文件监听（notify）、
   gitignore 遍历（ignore）、diff（imara-diff）、LSP 类型、tree-sitter binding。
   你的时间花在编辑器上，不是花在基础设施上。
3. **Unicode 表最新**（17.0.0，含 Indic conjunct break）。
   [实测] 对 `हिन्दी` 分割：Rust 2 个字形簇、Go 3 个——**光标键在两种语言里落点不同**。
   这个项目明确在意 CJK/emoji/组合字符的正确性。
4. **二进制最小**（550 KB vs Go 的 2.49 MB），启动最快（1.4ms vs 1.9ms）。
5. **在"没人做 code review"的前提下，编译期安全的价值被放大了。**

**缺点（诚实版）**

1. **借用检查器和编辑器的数据模型天生不合。** buffer ↔ view ↔ cursor ↔ undo 天然是环形引用。
   Helix 靠一个巨大的 `Editor` struct + 整数索引绕开，Zed 不得不发明 GPUI 的 `Entity<T>` 系统。
   **这是最大的排期风险**，缓解办法是从第一天就照抄 Helix 的所有权布局，而不是到处 `Rc<RefCell<>>`。
2. **增量编译慢 5.6 倍**（0.39s vs 0.07s）。可控，但要注意一个陷阱：
   [实测] `lto = true` 放进 `[profile.release]` 会让增量 release 变成 **5.2 秒**（13 倍代价）。
   → 必须把 LTO 放进单独的 `[profile.dist]`，只在发版时用。
3. **`target/` 目录巨大**：[实测] 单 target 148 MB，8 个交叉 target 并存时峰值 **1.2 GB**（对一个 156 行的程序）。
   Go 整个项目含 7 个交叉产物只有 31 MB。这是 **GitHub Actions 缓存上限**的现实问题。
4. **MSVC 目标在本机链接不了**（缺 link.exe，cargo-xwin 因代理封锁 `aka.ms` 也不行）→ 只能在 CI 上构建。

### 🥈 Go —— 真正的第二名，且**只在放弃 tree-sitter 时**才有竞争力

**优点（比通常认为的更强）**

1. **迭代速度压倒性**：冷编译快 2 倍，增量快 5.6 倍。对 AI 反复试错的循环，这是真实收益。
2. **[实测] "GC 停顿毁编辑器"是过时的说法**：176 万个存活指针对象、GOGC=50、300 万次分配压力下，
   STW 停顿中位数 **0.094ms**、最大 **0.182ms**——比一帧预算低约 90 倍。**要拒绝 Go 请不要用这个理由。**
3. **纯 Go 交叉编译无懈可击**：[实测] 7/7 目标全部成功（windows/amd64+arm64、darwin/amd64+arm64、
   linux/arm64、freebsd/amd64），每个 8 秒左右。
4. **没有借用检查器**，编辑器的环形数据模型可以自然表达。

**致命缺点**

1. **[实测] cgo 交叉到 macOS 做不到**：
   ```
   GOOS=darwin GOARCH=arm64 go env CGO_ENABLED  →  0        # 静默关掉
   CC='zig cc -target aarch64-macos' ... go build
   →  error: unable to find dynamic system library 'resolv'
   ```
   Go 运行时需要真 macOS SDK 里的 `-lresolv` 和 `-framework CoreFoundation`。
   **而两个还在维护的 Go tree-sitter binding 都是 cgo。**
   （注：cgo 交叉到 **Windows** 是能成的，这个问题是 macOS 专属。）
2. **没有生产级 rope**。micro 用的是朴素行数组。要么自己写，要么接受性能天花板。
3. **Unicode 落后两个大版本**，且标准库压根没有字形簇分割（`go doc unicode | grep -ci grapheme` = 0）。
4. **[实测] 两个反直觉的发现，都对 Go 不利：**
   - **tcell 比 crossterm 更不可移植**：TERM 未设置 → 直接崩；TERM=dumb → 崩；
     TERM 不在本地 terminfo 库里 → 崩（它会 shell out 调 infocmp）。crossterm 在以上全部情况下渲染出正确的屏幕。
   - **Go 的 PTY 测试是个陷阱**：creack/pty v1.1.24 的 Windows 实现是 `return nil, ErrUnsupported`，
     而 `GOOS=windows go vet` 报 OK。**类型检查绿、Windows runner 上运行时红。**
     这对一个"只能靠 CI 验证 Windows"的项目是最危险的一类问题。
5. 二进制大 4.5 倍。

### 🥉 C / C++（用 zig c++ 当工具链）

- **优点**：可移植性完全解决（6 个目标全通）；antirez 的 kilo 证明了 1308 行 C 就能做一个跨平台编辑器。
- **缺点**：rope、Unicode 层、LSP 客户端全部自己写；**而且每一个缓冲区 bug 都是潜在的 CVE**。
  在没有 code review 的前提下，把内存安全交给人（或 AI）是最差的选择。

### Zig

- **[实测] 0.16.0（2026-04-13）的 std.Io 重写几乎删掉了 std.posix，连 Ghostty 都还没迁移完。**
- 没有任何编辑器相关的库生态。
- **正确用法：把 zig 当 SokiText 的*链接器*（通过 cargo-zigbuild），而不是实现语言。** 这一点已经在用了。

### TypeScript + Tauri

- **Linux 端 webview 是系统 webkit2gtk 依赖** → 不是自包含二进制，且**在无头容器里完全跑不起来**。
- Tauri 官方承认 `tauri-driver` 没有 macOS 支持 → 自动化测试路线在最需要的平台上是断的。

### TypeScript + Electron

- **[实测] hello-world 的 Node SEA 就是 124,783,808 字节（124.8 MB）。** 直接出局。

---

## 结论

**用 Rust。** 但要诚实地说明它赢在哪：

> Rust 赢在 **tree-sitter 的交叉编译**、**Unicode 时效性**、**库的可得性**，
> **不是**赢在运行时性能，**也不是**因为 Go 的 GC 有问题。

而 vibe coding 这个前提又额外加了一条：**在没有人逐行 review 的情况下，编译期安全的价值被放大了。**
编辑器最可怕的 bug 不是崩溃，是**静默改坏用户的文件**——那类 bug 在 Go 里能编译通过、测试通过，
只在真实用户的 GBK 文件、只读文件、软链接上出事。

### 什么会推翻这个结论

**只有一件事：决定不用 tree-sitter。**

如果 SokiText 的语法高亮只用正则或手写（这对 v0.1 完全合理），
那么 Go 的 cgo 问题消失，它 5.6 倍的迭代速度就成为压倒性优势。

**所以真正该先回答的不是"用什么语言"，而是"要不要 tree-sitter"。**
我的建议是：**v0.1 不要，Phase 3 再加**——但**语言按"最终会加"来选**，
因为语言换起来是重写，tree-sitter 加起来只是加个模块。

### 配套的具体配置

- 目标三元组：`x86_64-unknown-linux-musl`（gnu 版需要 GLIBC_2.34，会排除 22.04 之前的系统）、
  `x86_64-pc-windows-msvc`（**在 windows runner 上原生构建**，mingw 版会链接老的 msvcrt.dll）、
  macOS 用 `cargo-zigbuild --target universal2-apple-darwin` 本地构建 + 在 `macos-26` 上**测试**
- 第一条环境配置命令：`pip install ziglang && cargo install cargo-zigbuild`
- `[profile.release]` 里**不要放 LTO**，单开 `[profile.dist]`
- runner label 全部写死，**永远不要用 `macos-latest` / `windows-latest`**
