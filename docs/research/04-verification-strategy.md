# 04 · 验证策略：没有 mac 和 Windows，怎么保证不发出坏版本

## 核心思路：把跨平台风险压进一个能一屏读完的壳里

```
bytes/events --[decode]--> Key --[keymap]--> Cmd --[Core::apply]--> (State', Vec<Effect>)
                                    State --[render]--> ratatui Buffer --[backend]--> bytes
```

四个纯函数。**`Core::apply` 返回 I/O 的*描述*，自己不做 I/O**：

```rust
enum Effect { WriteFile { path: PathBuf, bytes: Vec<u8> }, SetClipboard(String), Quit, ... }
```

不纯的外壳——`enable_raw_mode`、`event::read`、`fs::write`、`terminal.draw`——应当只有 **100~150 行**，
它是唯一无法单元测试的代码。调研中有 agent 真的搭了这个骨架（47 行核心 + 57 行外壳 + 12 行渲染），
下面每一层测试都在它上面跑通了。

### Workspace 布局（对应 Helix 的 helix-core / helix-view / helix-term，这正是让 Helix 可测的形状）

| crate | 内容 | 平台依赖 |
|---|---|---|
| `soki-core` | rope / 光标 / 撤销 / 命令 / keymap | **零平台依赖、零 C build script**（CI 强制） |
| `soki-render` | `Core → ratatui::Buffer` | 只依赖 ratatui-core |
| `soki-platform` | 文件/路径/编码/剪贴板，写成对"路径和字节"的准纯函数 | cfg 很多，但每个函数都能单独测 |
| `soki` | 二进制 | 那 100~150 行外壳 |

**最该抄的一个东西**：Helix 的 `Application::event_loop_until_idle(&mut rx_stream)` +
`parse_macro("ihello<esc>")`。三个不同方向的调研独立地把它认定为"整份资料里最值得抄的单个构件"。
它就是那个让 macOS/Windows 正确性变成三个 runner 上普通 `cargo test` 的东西，
也是让 crossterm→termina 这个决定保持可逆的东西。

## 测试金字塔

| 占比 | 层 | 工具 | 在哪跑 |
|---|---|---|---|
| **60%** | 纯核心：表驱动 + 击键 DSL | 普通 `cargo test`，微秒级 | 三个 OS |
| **15%** | 属性 / 差分测试 | proptest：rope vs 朴素 `Vec<char>`；`undo(redo(x)) == x`；光标永远在字形簇边界；save→load 字节级往返 | 三个 OS |
| **12%** | 渲染快照 | ratatui `TestBackend` + insta 1.48.0 | 三个 OS |
| **8%** | 平台层 | `soki-platform` 的文件/路径/换行/编码测试，**普通 `cargo test`，不需要终端** | 三个 OS ← **macOS/Windows 覆盖率主要来自这里** |
| **4%** | PTY 端到端 | portable-pty 0.9.0 + vt100 0.16.2 | Linux 全量 / macOS 只做子串断言 / **Windows 只做冒烟** |
| **1%** | 定时任务 | cargo-fuzz（Linux nightly）、cargo-mutants（每月，只对 soki-core）、benchmark | Linux |

目标：每个 runner 上整套 PR 测试 **< 60 秒**。（调研里 8 个测试含 3 个真 PTY 测试跑了 2.5 秒，这个目标是现实的。）

### 关键洞察：差分测试是性价比最高的一招

把 rope 的行为和一个朴素 `String`/`Vec<char>` 参考实现对拍。便宜、无脑、抓 bug 极准。
同理，用一个独立参考实现校验 SokiText 的行/列换算和 UTF-16 偏移。

### ⚠️ ratatui 快照的两个坑（见 [02-measurements.md](02-measurements.md) 有实测细节）

1. `TestBackend` 的 `Display` **会丢掉所有样式** → 颜色相关的断言要自己写 `styled_view` helper。
2. **绝不要手写逐格 symbol 拼接**——双宽字符会被拼错并烤进快照。
   用 `Buffer::with_lines` 或 `assert_debug_snapshot!`，ratatui 自己的 Debug 是对的。
3. 好消息：ratatui 的 Debug 会主动标注 `// hidden by multi-width symbols: [...]`，
   所以**一个 CJK/ZWJ 夹具文件就能当整个 Unicode 宽度回归测试套件**。

### ⚠️ PTY 测试的铁律

**固定 sleep 是 flaky 的唯一来源。永远用谓词轮询（poll until predicate）。**
[实测] 固定 10ms sleep 在负载下失败率 56%，1ms 空闲时也 100% 失败；谓词轮询在所有测试条件下 **0/180 失败**。

另外两个技巧：
- **每帧用 DEC 2026 同步更新标记包起来**（`ESC[?2026h` / `ESC[?2026l`）。渲染循环里两行代码，
  给测试 harness 精确的帧边界，永久消灭一整类 flake。
- 400x100 的 pyte 屏幕上做 `''.join(screen.display)` 谓词会慢到把测试套件挂死。**只轮询你要断言的那几行。**

## 🔴 Windows 测试层是整个计划里最弱、却承担最多的一环

它本该替代"拥有一台 Windows 机器"，但是：

- **portable-pty 0.9.0 硬编码了 `PSUEDOCONSOLE_INHERIT_CURSOR`**，导致 ConPTY 发出 `ESC[6n` 并阻塞
  （约 3 秒超时，老镜像上可能直接挂死），除非 harness 回应；而 **vt100 0.16.2 没有 DSR 回调**，
  所以这个回应必须在解析器上游做字节嗅探。
- **ConPTY 会重写 SGR（`[49m` → `[m`）、丢弃 DCS、重排 OSC**，所以**屏幕快照不能跨平台共用**。
- **GitHub 的 windows runner 跑的是 conhost，不是 Windows Terminal** —— 你的用户实际用的终端从未被测试过。
- 佐证：termlens（一个专门做 TUI PTY 测试的项目）交付了 Linux + macOS，**把 Windows 留着没做**。
  一个专门干这个的项目都不碰，单人隔着 CI 日志盲写不该假设它能成。

**→ Windows PTY 层第一个月设成 `continue-on-error: true`，只当冒烟用。真正的 Windows 覆盖来自"平台层"（8%）那一档普通 `cargo test`。**

同时记住：[实测] crossterm 在 Windows 上对 `PushKeyboardEnhancementFlags` 返回**硬错误**，
而 `cargo check --target x86_64-pc-windows-msvc` **让它顺利通过**——这一整类 bug 对编译器是不可见的。

## 本地能做的（每次提交前，秒级）

```bash
# pre-commit：抓 cfg 门控的类型错误，这是 native check 抓不到的
cargo check --target x86_64-pc-windows-msvc --lib --bins    # 16s
cargo check --target aarch64-apple-darwin  --lib --bins     #  8s

# make winsmoke：交叉编译到 windows-gnu 并用 wine 跑（11s）
# 只验证启动/argv/输入解码/文件逻辑，绝不断言渲染
```

**为此必须给每个 C 依赖加 feature gate**，否则 tree-sitter 的 build.rs 会把这个金丝雀弄红。

## CI 工作流（目标：PR < 5 分钟）

> ⚠️ **runner label 必须写死，不要用 `macos-latest` / `windows-latest`**——两者都在 2026 年中迁移过，
> 而 `macos-14` 从 2026-11 起不再支持。写 workflow 时请对照 GitHub 官方文档核实一遍当时的 label。
> ⚠️ **MSRV 是 1.88**（ratatui 0.30.1+ 声明 `rust_version = 1.88.0`，是整个依赖树里最高的；
> crop 0.4.3 是 1.85、termina 0.4.0 是 1.71、crossterm 0.29.0 是 1.63）。调研初稿里写的 1.85 是错的。
> ⚠️ **提交 `rust-toolchain.toml`**：开发机是 rustc 1.94.1，而 runner 镜像是 1.98.0 ——
> 不钉住的话会出现"CI 报的 lint 在本地复现不了"。注意它会静默压过 `dtolnay/rust-toolchain@stable`，两者别同时用。

```yaml
name: ci
on: { push: { branches: [main] }, pull_request: {} }
permissions: { contents: read }
concurrency: { group: ci-${{ github.ref }}, cancel-in-progress: true }
env:
  CARGO_TERM_COLOR: always
  CARGO_INCREMENTAL: 0
  RUSTFLAGS: "-D warnings"

jobs:
  lint:
    runs-on: ubuntu-24.04
    steps:
      - uses: actions/checkout@v6
      - uses: dtolnay/rust-toolchain@stable
        with: { components: rustfmt, clippy }
      - uses: Swatinem/rust-cache@v2
      - run: cargo fmt --all --check
      - run: cargo clippy --all-targets --all-features --locked

  # 关键：核心 crate 不许沾终端库
  core-purity:
    runs-on: ubuntu-24.04
    steps:
      - uses: actions/checkout@v6
      - run: |
          if cargo tree -p soki-core | grep -qE 'crossterm|ratatui|termina'; then
            echo "::error::soki-core must not depend on a terminal backend"; exit 1
          fi

  test:
    strategy:
      fail-fast: false
      matrix:
        include:
          - { os: ubuntu-24.04,  name: linux-x64,   pty: full  }
          - { os: macos-26,      name: macos-arm64, pty: substring }
          - { os: windows-2025,  name: windows-x64, pty: smoke }
    runs-on: ${{ matrix.os }}
    timeout-minutes: 20
    steps:
      - uses: actions/checkout@v6
      - uses: dtolnay/rust-toolchain@stable
      - uses: Swatinem/rust-cache@v2
        with:
          key: ${{ matrix.name }}
          save-if: ${{ github.ref == 'refs/heads/main' }}   # 别撑爆 10GB 缓存上限
      - uses: taiki-e/install-action@nextest
      - run: cargo nextest run --locked --all-features --no-fail-fast

  msrv:
    runs-on: ubuntu-24.04
    steps:
      - uses: actions/checkout@v6
      - uses: dtolnay/rust-toolchain@1.88.0
      - run: cargo check --locked --all-features

  deny:
    runs-on: ubuntu-24.04
    steps:
      - uses: actions/checkout@v6
      - uses: EmbarkStudios/cargo-deny-action@v2
        with: { command: check bans licenses sources advisories }
```

**< 5 分钟的依据**：与公共仓库 runner 同规格的 4 vCPU 机器上，97 个 crate 的编辑器依赖集
冷编译 debug 27 秒、clippy 15 秒。缓存热了以后 test job 的时间主要是 checkout + 装工具链 + 恢复缓存，约 90~150 秒。
macos runner 是最慢的一条腿（3 vCPU / 7GB），按 Linux 的 2 倍估。

## 发布流水线

### 目标矩阵（8 个）

| target | 在哪构建 | 怎么构建 |
|---|---|---|
| x86_64-unknown-linux-gnu | ubuntu | `cargo zigbuild --target ...gnu.2.28` ← glibc 下限 2.28 比 ubuntu-22.04 的 2.35 兼容性好得多 |
| aarch64-unknown-linux-gnu | ubuntu | 同上，**不需要 arm runner** |
| x86_64-unknown-linux-musl | ubuntu | 裸 cargo，全静态 |
| aarch64-unknown-linux-musl | ubuntu-24.04-arm | 裸 cargo（免费 arm runner） |
| aarch64-apple-darwin | macos | 裸 cargo（原生，SDK 正确） |
| x86_64-apple-darwin | macos | 裸 cargo 交叉（Xcode 两个 SDK 都有；Intel runner 2027-08 就没了） |
| x86_64-pc-windows-msvc | windows | 裸 cargo ← **MSVC ABI 才是 Windows 用户期待的** |
| aarch64-pc-windows-msvc | windows | 裸 cargo 交叉 |

外加一个 macos 上的 `lipo` 合成 universal2 job（**lipo 之后要重新 `codesign -s -`**，lipo 会让 ad-hoc 签名失效）。

### 🔑 最重要的一个 job：`smoke`

**把每一个产物下载到它声称支持的那个 OS/架构上真跑一次 `--version`，然后才允许发布。**

这是你没有 Mac 和 Windows 机器的**唯一**替代品。没有它，"从 Linux 交叉编译"的意思就是"发了个没测过的东西"。
这一点尤其重要，因为：**这次调研产出的每一个 macOS 二进制都从没被执行过**——
cargo-zigbuild 是拿 zig 的 libSystem stub 链接的，不是 Apple 的链接器配真 SDK，
而且没有任何 macOS 模拟器可用（darling 不存在）。**每一个 macOS 运行时 bug 都会表现为一个红色 CI job，
没有本地复现，没有调试器。**

### 签名与分发：v0.1 不要买证书

不用买 99 美元的 Apple 开发者账号，也不用买 Windows 代码签名证书。按这个顺序做三个渠道：

1. **`curl | sh` 安装脚本** —— 一个脚本同时服务 Linux + macOS。
   **curl 不会设置 `com.apple.quarantine`，所以从构造上就绕开了 Gatekeeper。**投入产出比最高。
2. **Homebrew tap**（`zhangyuxuan1999/homebrew-sokitext`），要做 **formula 不是 cask**。
   formula 免公证，Homebrew 会在安装时 ad-hoc 签名。
   （进 homebrew-core 需要 ≥75 star / ≥30 fork / ≥30 watcher，暂时够不着。）
3. **Scoop bucket**（`zhangyuxuan1999/scoop-sokitext`），一个 JSON manifest 加 `checkver`+`autoupdate`。
   **Scoop 的下载路径不设 Mark-of-the-Web，所以没有 SmartScreen 弹窗** ——
   这是在没有证书的情况下给 Windows 用户干净安装体验的**唯一**办法。

WinGet / Chocolatey / AUR / Nix / deb / rpm / Snap / Flatpak 都是以后的事，或者等社区来做。
`cargo publish` 顺手做（前提是名字问题解决，见 [06-open-questions.md](06-open-questions.md)），但它不是真正的分发渠道。

### 版本与变更日志

`release-plz` 在 main 上维护一个滚动的 release PR（从 conventional commits 生成 changelog、
用 cargo-semver-checks 检测破坏性变更、打 tag、发布到 crates.io），**但它不构建二进制**——这个分工正好。
**从第 1 个 commit 就开始用 conventional commits**，事后补是很痛苦的。

**cargo-dist 暂时不用**：还活着但基本是单人维护，默认值里钉着已废弃的 `macos-14`，
不支持 aarch64-pc-windows-msvc，而且链接 glibc 2.35。先手写。

### 供应链

`cargo-deny`（bans/licenses/sources/advisories）+ dependabot（cargo + github-actions，每周）+
`actions/attest-build-provenance`。

> 注意"Rust release 构建默认就是可复现的"这个结论**是在一个没有 build script、没有 C 依赖的玩具 crate 上测的**，
> 不能外推到有 tree-sitter `cc` build script 的 SokiText。

## 第一天要写的 10 个测试

1. `load(save(x)) == x` 字节级往返 —— 夹具覆盖 CRLF / LF / CR-only / 混合换行 / UTF-16LE+BOM / GBK / Shift-JIS / latin-1 / 非法 UTF-8 / 1MiB 无结尾换行
2. proptest：随机编辑序列下 rope 内容 == 朴素 `String` 参考实现
3. proptest：`undo(redo(x)) == x`
4. proptest：光标位置永远落在字形簇边界上
5. 击键 DSL：`test("hello|", "ihi<esc>", "hhielloi")` 这类表驱动核心测试
6. TestBackend 快照：一个含 CJK + emoji ZWJ + 组合字符的夹具（**这一个就是整个宽度回归套件**）
7. 文件名校验表：Windows 保留名/保留字符/结尾空格句点（跑在 Linux 上）
8. 非 UTF-8 文件名的打开-编辑-保存（Linux 上可测）
9. 终端恢复：pty 里跑 soki，正常退出与 panic 退出后断言 termios 恢复且发出了 `ESC[?1049l`
10. 启动不卡死：把 stdout 接管道 + 无 TTY，断言不会因为能力探测超时而挂起
