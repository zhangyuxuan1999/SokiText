# 0002 · 单进程终端 TUI

**状态**：accepted

## 决策

终端 TUI，**一个进程**。不做 GUI，不做 Electron/Tauri，不做"核心 + 独立前端进程"。

## 理由

**GUI 被约束否决，不是被审美否决**：eframe/egui 是 38 倍二进制体积、345 个 crate、5 分钟编译，
而且在开发机上 apt 装 X 库和 Mesa 之前**根本跑不起来**——你会写出一个自己都用不了的编辑器。
Tauri 的 Linux webview 是系统 webkit2gtk 依赖，无头环境完全无法运行，
且官方承认 `tauri-driver` 没有 macOS 支持。Electron/Node SEA 的 hello-world 就 124.8MB。

**多进程被 xi-editor 作者亲口否定**："我现在坚定地认为，前端和核心的进程分离不是个好主意"、
"async 是复杂度的乘数"。而且 xi 坏掉的功能恰恰是**滚动**和**带折行的 resize**——jio 第一天就要有的东西。

对照：Neovim 和 VS Code 把进程边界放在**表现层**和**不可信扩展**上，**不在模型层**。这是可以抄的。

## 后果

- 键位设计必须按**最差的终端**来：GNOME Terminal 发不出 `Ctrl+Shift+*` 和 `Ctrl+数字`，
  `Tab == Ctrl+I`、`Enter == Ctrl+M`。kitty 协议只用来解锁额外绑定，永不承载默认绑定
- 显示宽度只能当成**渲染提示**：unicode-width / tmux / vt100 三方不一致，且没有任何 CI 能抓。
  缓解：每画一行都重发绝对定位 `CSI row;col H`，让分歧只毁一行
- 无障碍（屏幕阅读器）不在范围内 —— 见 [Q11](../research/06-open-questions.md)

## 什么会推翻它

- jio 的真实目标其实是**渲染**展示（连字、比例字体、平滑滚动、内联图片）——终端单元格模型是硬天花板
- 屏幕阅读器支持成为发布要求
- 项目所有者拿到了一台笔记本（GUI 的反馈回路问题就基本消失了）
