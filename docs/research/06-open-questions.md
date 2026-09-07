# 06 · 待拍板的问题

> 这些是调研**答不了**的问题——它们要么是产品判断，要么依赖只有你知道的信息。
> 每一条我都给了推荐答案和理由，但决定权在你。
>
> **前四条会阻塞第一行代码**，后面的可以边做边定。

---

## 🔴 Q1. 编辑模型：模态、选区优先、还是非模态？

**这个决定在 rope 之上游**，因为它决定核心数据结构：

- 选区优先（Kakoune/Helix）→ 从第一个 commit 起就需要 `Selection { ranges: SmallVec<[Range; 1]> }` 和多光标
- 非模态（nano/micro/VS Code）→ 单光标，完全不同的命令面

**而且它和一个硬性终端约束冲突**：

> [实测] GNOME Terminal（Ubuntu 和 Fedora 的默认终端，VTE，不支持 kitty 键盘协议）
> **根本发不出 `Ctrl+Shift+*` 和 `Ctrl+数字`**，而且 `Tab == Ctrl+I`、`Enter == Ctrl+M`。

**非模态编辑器在多数 Linux 终端上会不够键位用。**模态编辑器很大程度上就是因为这个约束才存在的。

**我的推荐：选区优先的模态编辑器（Helix 模型）。**理由：

1. 它解决键位不够用的问题（这是终端里的硬约束，不是偏好）
2. `Selection`/`ChangeSet` 代数让撤销、多光标、未来的协同变成**一套**机制
3. 它是**纯 `jio-core` 里的模型决定，完全可无头单元测试** —— 不是架构豪赌，是可以放心花的创新预算
4. `ranges.len() == 1` 就退化成 vim 式单光标，所以这是超集不是分支

**反对意见**：如果你想要的是"能给不会 vim 的人用的编辑器"，那非模态才对，但你要接受在 GNOME Terminal 上
只能靠 `Alt+` 和前缀键（像 nano 那样）撑起整个命令面。

---

## 🔴 Q2. 名字：`jio` 有两个现实障碍

1. **crates.io 上 `jio` 已被占用**（版本 0.0.0，2024-08-15 发布）。
   → `cargo publish` 和 `cargo install jio` 都不可用，除非走 crates.io 的名称争议流程。
2. **"JIO" 是 Reliance Industries 的注册驰名商标**（多个美国注册），
   印度法院授予过禁令，**且该公司有针对个人开发者的 `jio-` 前缀名称维权记录**。

改名的成本随每一个 commit 增长：发版之后再改意味着安装说明失效、Homebrew tap 作废、Scoop bucket 作废。

**我的推荐：现在就换一个名字。**可以保留 `jio` 作为仓库名/内部代号，但发布用的二进制名和 crate 名换掉。
选名字时同时检查：crates.io、Homebrew formula、Scoop/WinGet、以及 `PATH` 上的二进制名冲突。

**如果你坚持用 jio**，那也完全可以——只是要接受不发 crates.io、并承担商标风险。这是你的决定，我按你说的做。

---

## 🔴 Q3. License

**仓库现在没有 LICENSE 文件 = 保留全部权利** = 没人能贡献、fork 或打包分发，Homebrew/发行版打包直接被堵死。

**我的推荐：MIT OR Apache-2.0 双许可**（Rust 生态惯例）。

需要注意的依赖许可（聚合义务）：

| 依赖 | 许可 | 注意 |
|---|---|---|
| nucleo 0.5.0 | **MPL-2.0** | 弱 copyleft，文件级。用它要知道 |
| termina 0.4.0 | MIT **OR** MPL-2.0 | **明确选 MIT** |
| ropey 1.6.1 | MIT only | ropey 2.0 才是 MIT OR Apache-2.0 |
| Helix 的代码 / tree-house | MPL-2.0 | **所以只抄设计，不要 git 依赖它的 crate** |

---

## 🔴 Q4. 你写过 Rust 吗？

这不是客套问题。调研里最诚实的一条风险是：

> 借用检查器 vs 编辑器天然的环形数据模型（buffer ↔ view ↔ cursor ↔ undo）
> 是**整个项目最大的排期风险**。Helix 的解法是一个巨大的 `Editor` struct + 整数索引；
> Zed 不得不发明 GPUI 的 `Entity<T>` 系统。

如果答案是"没写过"，那么 Go 的 **5.6 倍增量编译速度**和没有借用检查器这两点，
分量比任何一份调研给的都重，值得重新权衡（代价是放弃 tree-sitter，或者接受 macOS 构建完全无法本地复现）。

---

## 🟠 Q5. 要不要 tree-sitter？（这是架构决策，不是功能决策）

它同时决定三件事：

- **语言选择**：不要 tree-sitter，Go 就重新有竞争力了
- **本地交叉编译能力**：[实测] 任何 C 依赖都会让 16 秒的本地 `cargo check --target <msvc>` 金丝雀失效
- **Linux→macOS 交叉构建**：单个链接 macOS framework 的 crate 就能让 zigbuild 彻底失败

**我的推荐：Phase 1 不要，Phase 3 再加**，并且加的时候**自己 vendor 语法的 C 源码**，
用一个 build.rs 编译，**绝不依赖 `tree-sitter-*` wrapper crate**（[实测] 它们会拉进冲突的 runtime 版本）。
同时给每个 C 依赖加 feature gate，保住那个金丝雀。

---

## 🟠 Q6. v0.1 到底要做什么？（现在没有人写下来过）

没有这个清单，就没法判断上面任何一个子系统该不该进 v1。

**我的推荐（对标 Kibi 的 2652 行）**：打开 / 编辑 / 保存 / 撤销重做 / 搜索 / 退出 +
三平台预编译二进制 + `curl|sh` 安装。**没有**语法高亮、**没有** LSP、**没有** git 集成、**没有**插件、**没有** tokio。

---

## 🟡 Q7. 安全模型 / 工作区信任

一个会启动 LSP server、加载 tree-sitter 语法、读取 per-project 配置的编辑器，就是一个任意代码执行面：
clone 一个恶意仓库然后打开它，`.jio/config.toml` 里写什么就执行什么。VS Code 正是为此做了 Workspace Trust。

另外：tree-sitter 的 C 语法是**在内存安全的编辑器里解析不可信输入的内存不安全代码**；
Helix 的动态语法加载（从用户目录 dlopen 一个 .so）是第二条攻击路径。

**我的推荐：在做 LSP/插件之前定下信任模型**，而不是之后。v0.1 里最简单的答案是：不读 per-project 配置。

---

## 🟡 Q8. 最低支持平台契约（要写下来并被测试，不是等 bug 报告发现）

**我的推荐：**

| 平台 | 下限 | 依据 |
|---|---|---|
| Windows | **10 1809 / build 17763** | 四个方向独立提出。portable-pty 在此之下硬失败且无 winpty 回退；termina 和 WezTerm 都要求它 |
| macOS | 需明确（zig 打的是 minos 13.0，rustc 默认 11.0） | **CI 里没有任何东西在断言这个** —— 要加断言 |
| Linux | **glibc 2.28**（走 cargo-zigbuild）+ musl 静态 | 比 ubuntu-22.04 的 2.35 兼容性好得多 |

---

## 🟡 Q9. 配置与键位映射的格式和位置

大家都默认是 TOML，但没人设计过 schema、合并/覆盖语义、以及每个 OS 上放哪。

**地雷**：[实测] `directories` 6.0.0 把 Windows 映射到 **Roaming AppData**、macOS 映射到 `~/Library/Application Support`，
**而 TUI 用户预期的是 XDG**。**发布后再改是对用户不友好的**，所以要现在定。

**我的推荐**：优先 `$XDG_CONFIG_HOME` / `~/.config/jio/`（三个平台都是），
Windows 上 Roaming 只放那个小 config 文件、`LocalAppData` 放撤销历史/会话/swap/缓存。**并且写进文档。**

---

## 🟡 Q10. 崩溃恢复策略：v0.1 需要吗？

两个方向给了两套不同的方案（vim 式 WAL 日志 vs 无损字节转义），**没人核算过成本，也没人问过 v0.1 是否需要**。

**我的推荐**：v0.1 只做"保存前检查文件是否被外部修改过"（mtime + size），
WAL 日志放到 Phase 2 —— 但**默认就地写入**这个决定要从第一天就定下来，因为它决定 `jio-platform` 的形状。

---

## 🟢 Q11. 无障碍（屏幕阅读器）

只被提过一次就没人管了：AccessKit 0.25.0 是唯一真正的答案，**而它需要 GUI**。

**如果屏幕阅读器支持是发布要求，它会反过来推翻"只做 TUI"这个决定**，所以需要一个明确的是/否。

**我的推荐：明确说不。** TUI 编辑器的无障碍靠终端模拟器自身提供，这是这类软件的行业惯例。
