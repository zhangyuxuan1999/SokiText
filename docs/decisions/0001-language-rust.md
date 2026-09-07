# 0001 · 用 Rust

**状态**：proposed —— 在 [Q4（你是否写过 Rust）](../research/06-open-questions.md) 回答之前不算最终决定。

## 背景

jio 要在 Linux / macOS / Windows 上跑，而全部开发在一台无头 Linux 容器里进行，
macOS 和 Windows 只能通过 GitHub Actions 验证。

## 决策

用 Rust。

## 理由（实测，非偏好）

带 tree-sitter C 语法的 339 crate 真实依赖集，从本机成功交叉编译到
musl / windows-gnu / aarch64-linux / **macOS arm64 / macOS x86_64 / macOS universal2**，
tree-sitter 的 C 运行时产出了真正的 Mach-O arm64 目标文件。

Go 做不到同一件事：`GOOS=darwin go env CGO_ENABLED` 静默返回 0，
用 `zig cc` 也失败，因为 Go 运行时需要真 macOS SDK 里的 `-lresolv` 和 `-framework CoreFoundation`。
而两个还在维护的 Go tree-sitter binding 都是 cgo。

次要理由：Unicode 表领先两个大版本（17.0.0 vs Go uniseg 的 15.0.0，实测对印地语分割结果不同）；
有生产级 rope（ropey，125MB 文件 1.91µs/次插入）；二进制小 4.5 倍。

## 明确**不是**理由

- ❌ "Go 的 GC 会毁掉编辑器"——实测 STW 最大 0.182ms，这个论点已经过时了
- ❌ "Rust 交叉编译更强"——对 Windows 而言 Go 更强；Rust 只是在 macOS+C 依赖这个组合上更强

## 后果

- 增量编译比 Go 慢 5.6 倍，`target/` 大 40 倍（是 CI 缓存问题）
- **`lto = true` 不能放进 `[profile.release]`**，放到单独的 `[profile.dist]`
- MSVC 目标只能在 CI 上链接（本机 `aka.ms` 被代理封锁，cargo-xwin 用不了）
- 最大的排期风险是借用检查器 vs 编辑器的环形数据模型。缓解：抄 Helix 的做法——
  一个大 `Editor` struct + 整数索引，不要到处 `Rc<RefCell<>>`

## 什么会推翻它

1. jio 决定不用 tree-sitter（Go 的 cgo 问题随之消失，它的迭代速度对单人项目就是压倒性的）
2. 项目所有者没有 Rust 经验，且优先考虑推进速度
