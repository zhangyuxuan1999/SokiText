# 0003 · 纯函数核心 + 极薄 IO 外壳

**状态**：accepted —— 这是让整个项目在没有 macOS/Windows 硬件的情况下可行的那个决定。

## 决策

```
bytes/events --[decode]--> Key --[keymap]--> Cmd --[Core::apply]--> (State', Vec<Effect>)
                                    State --[render]--> ratatui Buffer --[backend]--> bytes
```

`Core::apply` 返回 I/O 的**描述**（`Effect::WriteFile{..}` / `SetClipboard(..)` / `Quit`），自己不做 I/O。
不纯的外壳（raw mode、event::read、fs::write、terminal.draw）控制在 **100~150 行**。

Workspace：`soki-core`（零平台依赖、零 C build script）/ `soki-render` / `soki-platform` / `SokiText`。
对应 Helix 的 helix-core / helix-view / helix-term —— 正是让 Helix 可测的那个形状。

**CI 强制核心纯净性**：`cargo tree -p soki-core | grep -qE 'crossterm|ratatui|termina' && exit 1`

## 理由

这让 90% 的行为变成三个 OS 上完全相同的普通 `cargo test`。
在无法手动测试 macOS 和 Windows 的前提下，这不是"好的工程实践"，而是**唯一可行的验证途径**。

抄 Helix 的 `Application::event_loop_until_idle(&mut rx_stream)` + `parse_macro("ihello<esc>")` ——
三个独立方向的调研都把它认定为整份资料里最值得抄的单个构件。

## 后果

- 测试金字塔：60% 纯核心 / 15% 属性与差分 / 12% 渲染快照 / 8% 平台层 / 4% PTY / 1% 定时
- **macOS 和 Windows 的真实覆盖率主要来自那 8% 的"平台层普通 cargo test"**，不是来自 PTY
- 终端后端藏在自己的 `Backend` trait 后面 → crossterm→termina 的决定保持可逆
- 第一周就要立起三平台 CI 矩阵。**测试骨架如果不是第一个做的，就永远不会做。**
