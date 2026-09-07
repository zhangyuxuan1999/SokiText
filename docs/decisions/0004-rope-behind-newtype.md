# 0004 · Rope，藏在自己的 Buffer 类型后面

**状态**：accepted

## 决策

- 数据结构用 **rope**，v1 用 **ropey 1.6.1**，`default-features = false, features = ["simd", "cr_lines"]`
- **所有 rope 调用点只出现在一个模块里**，`jio::Buffer` 对外只暴露**字节偏移**
- 光标/编辑抄 Helix 的代数：`Range{anchor,head}` / `Selection{ranges,primary}` / `ChangeSet{Retain|Delete|Insert}`
- rope 里只存 UTF-8、只存 LF；编码和换行风格记在 `Document` 上，保存时还原

## 理由

实测（100MB 文件、10000 次随机单字符插入）：ropey 2.097µs、crop 2.882µs、jumprope 3.431µs、
**std String 2.3985ms（慢 1143 倍）**。String 每秒只能承受约 400 次击键。

**ropey vs crop 没有明确赢家**，所以这个决定必须做成可逆的：

- crop 的架构优势是真的（字节索引，和 tree-sitter `InputEdit`、LSP utf-8 positionEncoding、
  ripgrep 结果、Rust `str` 全都一致；ropey 1.x 的 char 索引让"字节/字符搞混"能编译通过、在 ASCII 上正常、
  **只在 CJK/emoji 上出错**——正是没有硬件就发现不了的 bug 类别）
- ropey 的优势也是真的（Helix 三平台出货在用、文档最好、`from_reader` 省 42% 内存、crop 基本单人维护）
- 局部编辑 crop 更快、均匀随机插入 ropey 更快 —— 两组数据不矛盾，是不同工况

选 ropey 是因为"无聊即安全"；那层包装才是真正的决策，它把换 rope 变成约 200 行的改动。

⚠️ **必须关掉 ropey 的默认 features**：默认会把 U+2028/VT/NEL/FF 也算成换行，导致行号和 LSP 诊断静默错位。

## 后果

- 撤销用**反转 ChangeSet 日志 + 打字突发合并**，不是每次击键存 rope 快照（快照每个 2.5~3.1KB，2 万次击键 = 50~60MiB）
- `map_pos` 用同一段代码变换光标、标记、诊断、折叠状态 → 撤销/多光标/未来协同是**一套**机制
- 多光标免费；vim 式单光标只是 `ranges.len() == 1`

## 什么会推翻它

字节/字符索引混淆在真实开发中反复出 bug → 换成 crop（这正是那层包装存在的意义）。
