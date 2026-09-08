# 02 · 本机实测数据

> 全部在开发机上跑出来：Ubuntu 24.04 / x86_64 / 4 vCPU / 15GB RAM / rustc 1.94.1 / 2026-09-07。
> 这一篇的价值在于：**它推翻了好几条广为流传的说法**。

## 被实测推翻的说法

| 常见说法 | 实测结果 |
|---|---|
| "Go 的 GC 停顿会毁掉编辑器" | ❌ 176 万存活指针对象、GOGC=50 下，STW 中位数 **0.094ms**、最大 **0.182ms**。要拒绝 Go 请用别的理由 |
| "Rust 不能交叉编译，那是 Go 的杀手锏" | ❌ 对 Windows 是错的。`cargo build --target x86_64-pc-windows-gnu` 14.27s 产出可运行 exe |
| "macOS 交叉编译必须有 SDK/osxcross" | ❌ `pip install ziglang` + cargo-zigbuild 产出真 Mach-O（platform=1, sdk=26.4, minos=13.0），连 objc2 桥接都编过了 |
| "Rust 二进制臃肿" | ❌ 同一个 TUI：Rust 550,816 B vs Go 2,486,456 B，**Rust 小 4.5 倍** |
| "tcell 是那个到处都能跑的、开箱即用的可移植选择" | ❌ tcell **在 CI 里比 crossterm 更不可移植**：TERM 未设置 → 直接崩；TERM=dumb → 崩；TERM 不在本地 terminfo 库里 → 崩。crossterm 在以上全部情况下渲染出字节正确的屏幕 |
| "Go 的工具链更好" | ❌ PTY 测试上反过来。creack/pty v1.1.24 的 Windows 实现是个 stub，`return nil, ErrUnsupported`，而 `GOOS=windows go vet` 报 OK ——**类型检查绿、Windows runner 上运行时红**。portable-pty 有真的 ConPTY 实现 |
| "用 tempfile 做原子保存" | ❌ 它会静默把权限降成 0600、丢掉 xattr、断开硬链接。它是为存密钥设计的，不是为替换用户文件 |

## 编译与产物

### 编译时间（4 核）

| | Rust | Go |
|---|---|---|
| 冷编译 debug | 15.65s | 7.77s |
| 冷编译 release（调优 profile） | 18.90s | — |
| 增量 debug（`touch` 后） | 0.39s | 0.07s |
| 增量 release + LTO | **5.2s** | — |
| 无操作 | 0.14s | 0.06s |

> **不要把 `lto = true` 放进 `[profile.release]`**。体积收益（931KB → 550KB）是真的，
> 但你会在每一次编辑上付 13 倍代价。放到单独的 `[profile.dist]` 里，只在发布时用。

核心依赖集（crossterm + ratatui + ropey + unicode-* + anyhow + toml + serde）：
clean release 23.77s，84 个唯一包 / Cargo.lock 191 条，峰值 RSS 321 MiB。

玩具编辑器（168 行 + 67 行测试）：clean release **14.25s**，增量 release 1.07s，增量 debug 0.33s，warm `cargo check` **0.15s**。

### 二进制体积（同一个 ~160 行 TUI）

| 配置 | 大小 |
|---|---|
| Rust debug | 15,392,624 B |
| Rust release 默认 | 931,832 B |
| Rust release + `strip -s` | 739,128 B |
| **Rust `opt-level="z"` + lto + cgu=1 + panic=abort + strip** | **550,816 B** |
| Go 默认 | 3,785,826 B |
| Go `-trimpath -ldflags='-s -w'` | 2,486,456 B |

### 启动时间（hyperfine，500 次）

Rust **1.4 ms** ± 0.1 · Go **1.9 ms** ± 0.1 · `/bin/true` 基线 1.1 ms
→ 净运行时初始化：Rust ~0.3ms，Go ~0.8ms。

### 各 crate 的编译代价（独立 crate、clean release、耗时 / 唯一包数 / target 目录）

```
crop        1.50s/3/5MB      vt100       2.44s/7/7MB      toml_edit  3.54s/9/16MB
notify      4.47s/11/15MB    portable-pty 4.86s/19/38MB   jumprope   5.24s/11/45MB
encoding_rs 6.57s/15/34MB    chardetng   6.60s/17/37MB    nucleo     6.99s/18/35MB
insta      10.28s/13/31MB    tree-sitter 13.39s/19/64MB   lsp-types 13.94s/16/74MB
proptest   15.35s/28/89MB    syntect    19.38s/47/115MB   criterion 28.81s/53/158MB
tower-lsp  29.22s/78/207MB   async-lsp  33.52s/67/193MB   termwiz   36.19s/90/243MB
arboard    37.68s/32/98MB    gix        67.89s/148/236MB
```

**gix 一个就是 148 个 crate / 236MB。** 这不是好奇心，是 GitHub Actions 缓存上限问题：
[实测] Rust 单 target 的 `target/` 是 148MB，同时存在 8 个交叉 target 时峰值 **1.2 GB**（对一个 156 行的程序）。
Go 整个项目含 7 个交叉产物只有 31MB。

## 交叉编译矩阵（从这台 Linux 机器）

| 目标 | 裸 cargo | + cargo-zigbuild | 备注 |
|---|---|---|---|
| x86_64-unknown-linux-gnu | ✅ | ✅ | |
| x86_64-unknown-linux-musl | ✅ 27.3s | ✅ 49.2s | static-pie |
| aarch64-unknown-linux-gnu | ❌ 无交叉链接器 | ✅ 19.0s | |
| aarch64-unknown-linux-musl | ❌ | ✅ 17.6s | |
| x86_64-pc-windows-gnu | ✅ 15.8s | ✅ | mingw 已预装 |
| **x86_64-pc-windows-msvc** | ❌ 缺 link.exe | ❌ | cargo-xwin 也不行：`aka.ms` 被代理 403 → **MSVC 只能在 CI** |
| aarch64-pc-windows-msvc | ❌ | ❌ | 同上 |
| aarch64-apple-darwin | ❌ 无 SDK | ✅ 10.3s | |
| x86_64-apple-darwin | ❌ | ✅ 10.4s | |

**只做类型检查（无需链接器，零额外配置，纯 Rust 依赖）：**

```
cargo check --release --target x86_64-pc-windows-msvc     OK  8.10s
cargo check --release --target aarch64-pc-windows-msvc    OK  3.57s
cargo check --release --target x86_64-apple-darwin        OK  4.09s
cargo check --release --target aarch64-apple-darwin       OK  3.89s
```

**这是这台机器上性价比最高的一个技巧**：它能抓到 native check 抓不到的 `cfg` 门控类型错误
（实测通过注入一个只在 Windows 分支存在的错误验证过：只有交叉 check 发现了它）。
`cargo clippy --target <msvc>` 也能正常跑。

**推论：必须给每一个 C 依赖加 feature gate**，这样 `cargo check --target ... --no-default-features`
这个"金丝雀"才能一直保持绿色。

### 交叉编译的隐藏地雷（实测踩过）

1. **不要手搓 zig cc shim**。`cc` crate 会把 Rust triple 原样传给 zig，zig 报
   `unable to parse target query 'x86_64-pc-windows-gnu': UnknownOperatingSystem`。必须用 `cargo zigbuild`，它自带翻译 wrapper。
2. **musl + zig 会 `duplicate symbol: _start`**（zig 的 crt1.o 和 rustc 的 self-contained crt1.o 撞车）。
   修法：`rustflags = ["-C", "link-self-contained=no", "-C", "target-feature=-crt-static"]`。cargo-zigbuild 的 README 不会先告诉你这个。
3. **`cargo check --target aarch64-apple-darwin` 会被 tree-sitter 的 build.rs 干掉**
   （`cc: error: unrecognized command-line option '-arch'`）。只有 `cargo zigbuild` 能解。

## 本地能做多少 Windows / arm64 验证

这台机器比想象中能干得多——**"只能靠 CI"这个说法要打折**：

| 手段 | 能做什么 | 不能做什么 |
|---|---|---|
| `cargo check --target <msvc/darwin>` 8~16s | 抓 cfg 门控的类型错误 | 抓不到任何运行时行为 |
| **wine 9.0** + windows-gnu 交叉编译（11s） | ✅ 启动、argv、输入解码、文件/路径逻辑。**[实测] 它抓到了两个真 Windows bug**（crossterm 键盘增强硬错误、`KeyEventKind::Release` 导致的 "hi"→"hhii" 重复输入） | ❌ **绝不能用来断言渲染**。驱动真 TUI 时 wine 会吃掉 CSI 序列的 ESC 前缀，屏幕上直接漏出字面量 `5;12H`；tcell 的输出是纯乱码 |
| **qemu-aarch64-static** + musl 静态 | ✅ arm64 Linux 真跑，`cargo test --target aarch64-unknown-linux-musl` 通过 | binfmt_misc 未注册，必须显式调 qemu |
| **tmux 3.4** 无头 | ✅ `tmux new-session -d -x 80 -y 24 <tui>` + `send-keys` + `capture-pane -p` 直接拿到渲染后的屏幕。零依赖、语言无关、纯 shell | 没有内建的 wait-for-predicate，要自己写轮询 |

**真正只能靠 CI 的，只剩三样：真实 ConPTY 控制台输入、macOS 上的一切、MSVC 特定的链接。**

## 环境清单（会影响你能写什么测试）

- **已有**：tmux 3.4、script(1)、infocmp/tput、strace、Xvfb、wine-9.0、qemu-aarch64-static、
  pyte 0.8.2 + pexpect 4.9.0 + ptyprocess 0.7.0（**预装，PTY 原型零安装**）、
  9 个 rustup target、cargo-zigbuild 0.23.4、cargo-xwin、mingw、41 个 terminfo 条目
- **没有**：任何 X 终端模拟器（xterm/alacritty/kitty/foot 全无 → **Xvfb 路线是死的，PTY 是唯一出路**）、
  screen、socat、expect、zig（但 `pip install ziglang` 可以）、可用的 docker daemon
- **没有控制终端**：`tty` → not a tty，`/dev/tty` → ENXIO。但全局 `/dev/ptmx` 是 0666，`pty.fork()` 能用 ——
  这就是 PTY 测试能成立的原因
- **磁盘会成为瓶颈**：调研期间容器根分区一直在 97~99%。带 gix + criterion + tree-sitter 语法的
  SokiText workspace 放不进默认位置，需要把 `CARGO_TARGET_DIR` 指到 `/dev/shm` 或定期清理

## PTY 测试的稳定性（这决定了端到端测试能不能用）

| 等待方式 | 空闲 | 4 核上跑 16 个忙循环 |
|---|---|---|
| **谓词轮询（poll predicate）** | Rust 0 失败/50，中位 7.3ms | Rust 0/40 中位 40ms，Go 0/40 |
| 固定 sleep 50ms | 0/30 | 0/25 |
| 固定 sleep 10ms | 0/30 | **Go 14/25 失败（56% flaky）**，p95 飙到 3060ms |
| 固定 sleep 1ms | **Rust 29/30 失败，Go 30/30 失败** | — |

**结论：固定 sleep 就是 flaky 的唯一来源。谓词轮询在测试过的所有条件下 0/180 失败。**
Rust 路线（portable-pty + vt100）在 4 路并行 PTY + 16 忙循环负载下 60/60 通过。

### 两个会浪费你一天的坑

1. **vt100 的 `Cell::contents()` 对从未写入的格子返回 `""`**（不是空格）。ratatui/crossterm 会跳过空白直接移动光标，
   于是朴素的逐格重建把 `│  1 hello world` 拼成 `│1helloworld`。pyte 没这个问题，vt100 的 `Screen::contents()` 也没问题
   ——**只有逐格 API 有**。
2. **ratatui 把双宽字形放在第 x 格、在 x+1 格写一个字面空格**。所以
   `(0..w).map(|x| buf[(x,y)].symbol()).collect::<String>()` 得到的是 `"日 本 語 ab  "` —— 错的，
   而且会被原样烤进 insta 快照。**必须用 `Buffer::with_lines` 或 `assert_debug_snapshot!`**，
   ratatui 自己的 Debug 输出是对的而且会标注：`"日本語ab  ", // hidden by multi-width symbols: [(1, " "), (3, " "), (5, " ")]`。

## 版本确认（2026-09-07，`cargo add --dry-run` 全部报 "could not be found in registry index"）

`ropey@2` / `ratatui@0.31` / `crossterm@0.30` / `tree-sitter@0.28` / `lsp-types@0.98` / `toml@2` / `helix-core@25` 都不存在。
所以 **ropey 1.6.1 / ratatui 0.30.2 / crossterm 0.29.0 / tree-sitter 0.27.0 / lsp-types 0.97.0 确实就是当前的头版本。**
