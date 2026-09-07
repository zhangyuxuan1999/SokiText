# jio 开发会话日志 / Session Log

> 这个文件是跨会话的"记忆"。每个 Claude Code 会话开始前先读它，结束前更新它。
> 开发容器是临时的（会被回收），所以任何有价值的中间产物都必须 commit 进这个仓库。

## 目录约定

| 路径 | 用途 |
|---|---|
| `docs/research/` | 调研报告（人读的结论） |
| `docs/research/raw/` | 调研过程的原始产物（agent 输出的 JSON、命令输出、实测数据） |
| `docs/decisions/` | ADR（Architecture Decision Record），一个决策一个文件 |
| `notes/` | 开发过程笔记、TODO、会话日志 |
| `notes/scratch/` | 临时脚本、基准测试脚本等（可以脏，但要能复现） |

## 时间线

### 2026-09-07 — Session 1：立项 + 调研

- 仓库初始状态：只有一行 `Readme.md`（"a text editor."）。
- 开发分支：`claude/cross-platform-text-editor-h0vslo`
- 关键约束：**开发者手头没有电脑**，全部开发在无 GUI 的 Linux 云容器里进行；
  macOS / Windows 只能通过 GitHub Actions runner 验证，没有任何手动测试的可能。
  => 这条约束会反过来决定架构：**不能在 CI 里验证的设计 = 会带着 bug 发版的设计**。
- 开发机环境：Ubuntu 24.04 x86_64 / 4 vCPU / 15GB RAM；
  rustc+cargo 1.94.1、go、node 22、python 3.11、gcc 13、clang 18、cmake 3.28；无 zig、无 dotnet。
- 网络：crates.io sparse index / npm / pypi / goproxy 可达；
  直接 curl github.com 与 crates.io HTTP API 被代理拦截（400/403），网页调研需走 WebSearch/WebFetch。

（后续进展见下方追加）
