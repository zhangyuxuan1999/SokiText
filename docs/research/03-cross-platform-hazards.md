# 03 · 跨平台陷阱清单

> 这一篇是"验收清单"。这里的每一条都是**实测验证过的**，而且绝大多数的失败模式是**静默的**——
> 不报错、不崩溃，只是悄悄改坏用户的文件。
>
> 在只有 Linux + ASCII 文件的开发环境里，这些坑**一个都不会暴露**。
> 最可能让 jio 被人记住的不是"jio 崩了"，而是"**jio 把我文件吃了**"。

## 🔴 A. 保存路径：三个"看起来对"的默认写法会毁数据

### A1. `tempfile::NamedTempFile` + persist → 权限降成 0600、丢 xattr、断硬链接

它是为**存密钥**设计的，不是为替换用户文件。用了它，用户每个保存过的文件都会变成只有自己能读。
非 root 用户对一个 0444 的文件保存**居然会成功**，并把它重置成 0644。

→ 用 `atomic-write-file` 0.3.1 + `preserve_mode(true)` / `try_preserve_owner`，或者从原文件 stat 出来 fchmod/fchown 临时 fd。

### A2. `encoding_rs::Encoding::encode()` 对 UTF-16 静默输出 UTF-8

```rust
UTF_16LE.output_encoding()  // == UTF-8   ← 字面意义上就是这样
UTF_16LE.encode(s)          // 返回 UTF-8 字节，had_errors == false
```

Windows 上 UTF-16LE 文件到处都是（PowerShell `>` 重定向、旧版记事本、`.reg` 导出、很多 `.ini`）。
jio 会正确解码，然后写出一个**标着 UTF-16 实际是 UTF-8 的文件**，且没有任何错误标志。

→ **保存路径绝不能走 `Encoding::encode`**。UTF-16 输出必须手写：`str::encode_utf16()` + `to_le_bytes()` + BOM。

### A3. `Encoding::encode()` 把无法映射的字符替换成 HTML 数字实体

用户打开一个 GBK/cp1252/Shift_JIS 文件，敲一个 emoji，保存 → 文件里出现字面量 `&#128578;`。

→ 用 `encode_from_utf8` 的 `..._without_replacement` 变体，**写之前**先检测不可映射字符，
然后像 VS Code 那样弹出"转成 UTF-8 / 丢弃该字符"的选择，而不是默默写坏。

### A4. 原子替换（rename）本身的四宗罪

| 问题 | 后果 |
|---|---|
| rename 到只读文件**会成功**，并抹掉只读位 | `chmod 444 important.conf` 不再有任何保护作用 |
| rename 到符号链接会**用普通文件替换掉符号链接** | 击穿所有 dotfile 管理器（stow / chezmoi / yadm）：用户编辑 `~/.config/nvim/init.lua`，保存后软链没了、仓库没变 |
| 断开硬链接、丢 xattr / ACL / SELinux label / macOS Finder 标签与资源分叉 | 用户看不见的元数据静默消失 |
| 换掉 inode | `tail -f`、`crontab -e`、Docker 单文件 bind mount、别人的文件监听全部失效 |

### ✅ 推荐：默认**就地重写**，崩溃安全另外买

**VS Code 和 vim 的实际默认都是保留 inode 的就地写入**（这一点两个来源分别核实过：
VS Code 的 `pfs.ts` 默认是 truncate + write + `fdatasync`，原子 rename 是每次调用显式 opt-in 且对符号链接直接拒绝；
vim 的默认是 `backupcopy=auto` —— 注意不是 Unix 上的 `yes`，那是 *Vi* 的默认）。

- **默认 `save.strategy = "rewrite"`**：打开已有 fd、truncate、写、`fdatasync`。
- 崩溃安全用**日志**买，不要用 rename 的原子性买：
  `<state_dir>/swap/<hash>.jio` 写前日志，每约 200 次击键或 4 秒空闲 fsync 一次（vim 的 `updatecount`/`updatetime` 模型），
  内容含原文件身份（Unix: dev+ino；Windows: volume serial + file index）、mtime、size、redo log。
  **这能扛住 SIGKILL 和断电，rename 的原子性扛不住。**
- 提供 `"replace"` 和 `"auto"`，`auto` 实现 vim 的 `bufwrite.c` 启发式：
  `nlink > 1`、`symlink_metadata` 说是符号链接、目录里建不了探测文件、
  或者对探测文件 fchown/fchmod 复现不了原 uid/gid/mode → 一律退回就地重写。
  **这套判断完全由 `fs::symlink_metadata` + 一次探测写决定，所以 Linux CI 上就能完整测。**
  > vim 的探测文件字面叫 `4913`（然后 5036、5159，每次 +123），这就是它会触发 webpack/cargo-watch 误重建的原因。
  > jio 应该用隐藏名 `.jio-probe-<pid>-<rand>`，并且**按目录缓存结果**，不要每次保存都探测。

### A5. Windows rename 的额外规则

- Rust 1.94.1 的 `fs::rename` 在 Windows 上是 `MoveFileExW(MOVEFILE_REPLACE_EXISTING)`，
  只在 `ERROR_ACCESS_DENIED` 时回退到 `FileRenameInfoEx`。**它不重试，也不处理 `ERROR_SHARING_VIOLATION`(32)**。
  而 32 正是别的进程（记事本、Office、Windows Search 索引、OneDrive、杀毒软件）持有文件时的错误码。
- VS Code 对 EACCES/EPERM/EBUSY 用递增退避**重试最多 60 秒**。
- → jio 必须把 `raw_os_error` 5 和 32 映射到重试循环，再退回就地写。
  **GitHub 的 Windows runner 基本永远复现不了这个**（没有第三方杀软、没有 OneDrive、工作目录没有索引服务），
  所以这是"靠设计保证"，不是"靠 CI 保证"。
- 若要做原子替换，Windows 上该用 **`ReplaceFileW`** 而不是 `MoveFileExW` —— 只有前者保留原文件的 ACL 和创建时间。
  macOS 上对应的是 `renamex_np(RENAME_SWAP)` / `-[NSFileManager replaceItemAtURL:]`。
- 临时文件**必须和目标同目录**：跨文件系统 rename 会 `EXDEV`。别用 `std::env::temp_dir()`。
  Linux 上可以用 `atomic-write-file` 的 `unnamed-tmpfile`（O_TMPFILE + linkat），这样临时文件从不可见。

## 🔴 B. 编码探测

- **BOM-less 的 UTF-16LE 会作为合法 UTF-8 无错解码**（NUL 是合法 UTF-8 字节），于是静默变成满是 NUL 的乱码。
- `Encoding::decode()` 会做 BOM 嗅探**覆盖你指定的编码**并静默剥掉 BOM；它返回的 bool 是 `had_errors` 不是 `had_bom`。
  有损解码不可逆。
- chardetng 当前是 1.0.0，**API 不是网上广传的 `new()` / `guess(None, true)`**，而是
  `EncodingDetector::new(Iso2022JpDetection)` / `guess(tld, Utf8Detection)`，且对短输入不可靠。

**探测阶梯（纯逻辑，用字节语料做单元测试即可，不需要任何 OS 特定 CI）：**

1. BOM
2. 前 ~8KiB 的 NUL 密度启发式（奇/偶位置 NUL > ~5% ⇒ UTF-16LE/BE）
3. 严格 UTF-8 校验
4. chardetng（**至少读 8~64 KiB，不要只读开头几个字节，不要对极小文件自动探测**）
5. 配置的兜底编码

必须记录 `(encoding, had_bom)` 并在保存时还原；用 `decode_without_bom_handling_and_without_replacement`
（返回 `Option`，遇到畸形输入是 `None`）判断能否无损表示。**不能无损解码的文件应该以只读/二进制模式打开，
而不是静默 U+FFFD 化之后再写回去。**永远提供一个"用指定编码重新打开"的命令——探测一定会有错的时候。

## 🟠 C. 路径

- **路径绝不能经过 `String` 往返**。Linux 上非 UTF-8 文件名 `to_str()` 返回 `None`，
  `Path::display()` 产出的有损字符串**打不开那个文件**。Windows 的镜像问题是 UTF-16 里的非配对代理项。
  → 全程 `PathBuf`/`OsString`，`display()` 只用于渲染。**这条在 Linux CI 上就能测**。
- **绝不能用字符串比较判断"这是不是已经打开的那个文件"**。
  macOS 的 `macos-latest` 默认是**大小写不敏感**的 APFS，Windows 也是。
  `README.md` 和 `readme.md` 会变成两个 buffer 互相覆盖。
  → 用 OS 级身份：Unix 的 dev+ino，Windows 的 `GetFileInformationByHandle` volume serial + file index
  （`same-file` crate 或 notify 的 `file-id`）。**这一条只有在 macOS/Windows runner 上才测得到，而且那里测得到。**
- **Windows 文件名规则必须在 Save-As 时校验**，别等到写的时候才发现：
  保留设备名 `CON/PRN/AUX/NUL/COM1-9/LPT1-9`（**还有 ISO-8859-1 上标形式 COM¹/²/³、LPT¹/²/³**），
  在任何目录下、带任何扩展名都保留（`NUL.txt` == `NUL`）；
  保留字符 `< > : " / \ | ? *` 和 0x01-0x1F；结尾不能有空格或句点。
  → 纯逻辑，写成表驱动单元测试跑在 Linux runner 上就够。
- **长路径**：GitHub 的 Windows runner 镜像里 `LongPathsEnabled=1`，**比用户的默认机器更宽松**——
  长路径测试在 CI 上通过说明不了任何问题。Rust std 对 ≥248 UTF-16 单元的路径会透明加 `\\?\` 前缀，
  但 `std::process::Command` 和你自己调的 Win32 API 不会。
  → 若 jio 要 shell-out（formatter / LSP / git），需要通过 build script 嵌入 `longPathAware` manifest，
  并且在 CI 里加一个把 `LongPathsEnabled` 关掉的 job。

## 🟠 D. 终端生命周期与信号

- **[实测] crossterm 0.29.0 只注册 SIGWINCH，完全没有 SIGTSTP/SIGCONT 处理**，
  而且 `enable_raw_mode()` 在其全局状态已是 `Some` 时**返回 `Ok(())` 但什么都不做（静默 no-op）**。
  → Ctrl-Z 陷阱：如果在 SIGCONT 时**没有先 `disable_raw_mode()`** 就重新 enable，那次 enable 是空操作，
  用户 `fg` 回来后终端是坏的。正确顺序：SIGTSTP → 离开备用屏、`disable_raw_mode`、
  `emulate_default_handler(SIGTSTP)`；SIGCONT → `enable_raw_mode`、进备用屏、全量重绘。
  Windows 没有 SIGWINCH 也没有 SIGTSTP，resize 通过 `WINDOW_BUFFER_SIZE_EVENT` 到达，crossterm 已归一化成 `Event::Resize`。
- **需要三条独立的恢复路径**：① 正常退出 ② panic hook ③ SIGTERM/SIGHUP handler。
  ratatui 0.30.2 的 `init`/`run` 会装一个恢复终端的 panic hook，但**必须装在你自己的 hook 之后**，
  而且 panic hook 对 SIGKILL/SIGSEGV/`process::abort` 无效。SIGKILL 和断电只能靠崩溃恢复日志。
  → CI 可以测 ①②：在 pty 里跑 jio，退出后检查 termios 的 ECHO/ICANON 已恢复、且发出了 `ESC[?1049l`。
- **能力探测必须打到 `/dev/tty`，绝不能打到 stdout**，且要有硬超时（~100ms），并且用 is-terminal 判断门控。
  **裸 PTY 不是终端**——它不回答任何能力查询。[实测] 三种方式复现：探测发出 `ESC[?u ESC[c` 后超时挂住，
  而同一个二进制在 tmux pane 里立刻得到回复。
  ⚠️ **crossterm 自己的 `supports_keyboard_enhancement()` 硬编码了 2000ms 超时**，
  会让 jio 在 tmux/CI/dumb 终端下启动卡两秒。要加一个"把 stdout 接管道后断言不卡死"的回归测试。

## 🟡 E. 剪贴板

- [实测] **arboard 3.6.1 在无头机器（包括默认的 `ubuntu-latest` runner）上直接失败**（X11 连接错误，实测 1 秒卡顿）。
- Linux 上 X11/Wayland 让**复制方进程本身充当数据服务器**，所以 jio 退出后用户复制的内容就没了（除非有剪贴板管理器）。
- **→ 三层策略**：内部寄存器永远是主力（yank/put 永远可用）；OSC 52 作为主要的对外通道；arboard 作为可选降级，
  **懒初始化、失败不致命、绝不阻塞启动**（要开 `wayland-data-control` feature）。
- **OSC 52 是最优解还有一个原因：它在 PTY 测试里完全可观测**（vt100 有 `copy_to_clipboard` 回调），
  把整个项目里最难测的子系统变成一个 40 行的确定性测试。它也是 SSH 场景下唯一能用的方案。
  注意：OSC 52 是**无应答**的，所以**不能因为发出了 OSC 52 就向用户报告"已复制"**。

## 🟡 F. 文件监听

- **必须监听父目录并按文件名过滤，绝不能 `watcher.watch(file)`**。
  [实测] inotify 对**文件**的监听在别的编辑器做一次原子替换后**永久失效**；对**目录**的监听能存活。
  Windows 上 notify 总是通过监听父目录来模拟单文件监听——**同一份 jio 代码在两个 OS 上语义不同**。
  这是少数几个能在 Linux runner 上便宜抓到的跨平台分歧。
- notify 8.2.0 的已知限制：网络文件系统（NFS/SMB/WSL）可能完全不发事件；
  macOS FSEvents 无法可靠观察非本人拥有的文件；Windows 用固定 16KiB 缓冲、溢出时要求全量重扫；
  Linux 上每个文件**和文件夹**都算一个 `max_user_watches`。
  → 只监听**已打开 buffer 所在的目录**（不是整个项目），收到重扫事件时对所有打开的 buffer 重新 stat，
  `recommended_watcher` 报 ENOSPC 时退回 `PollWatcher`。

## 🟡 G. 配置目录

- [实测] `directories` 6.0.0 / `dirs` 7.0.0 把 Windows 的 `config_dir`/`data_dir` 映射到 **Roaming AppData**，
  macOS 映射到 `~/Library/Application Support`，**不是 XDG**。
- Roaming AppData 在受管 Windows 机器上**会同步到域账户配置**——把撤销历史、会话状态、swap 文件、
  插件缓存放那儿会让用户每次登录都变慢。
  → 大的/机器相关的东西用 `data_local_dir`/`config_local_dir`（LocalAppData），Roaming 只放那个小配置文件。
- macOS 上应当**优先尊重 `$XDG_CONFIG_HOME` / `~/.config`（若存在）**，再退回 Application Support ——
  这是多数 CLI 类工具的做法，也是 TUI 用户的预期。**并且要写进文档**。发布后再改是对用户不友好的。

## 🟢 H. 换行符与 git

- 内部统一 LF，在 `Document` 上记录原始风格 + 是否混合，保存时还原。
- **必须提交 `.gitattributes`**，否则 Windows runner 上 git 的 autocrlf 会重写你的测试夹具，
  让换行符测试**因为错误的原因**通过或失败：

```gitattributes
* text=auto eol=lf
*.snap text eol=lf
tests/fixtures/** binary
```

## 无法在 CI 里捕捉的（只能靠设计保证）

| 项 | 为什么 CI 抓不到 |
|---|---|
| Windows 杀软/OneDrive 导致的 `ERROR_SHARING_VIOLATION` | runner 上没有杀软、没有 OneDrive、工作目录没索引服务 |
| 真实 Wayland 剪贴板 | CI 里根本没有 Wayland |
| 终端宽度分歧（emoji/组合字符/印地语） | [实测] unicode-width、tmux 3.4、vt100 在 12 个字符串里有 7 个三方不一致。**没有任何一层能抓** |
| 用户实际使用的 Windows 终端 | GitHub 的 Windows runner 跑的是 conhost，不是 Windows Terminal |
| 长路径的真实行为 | runner 镜像已经 `LongPathsEnabled=1` |
| 大小写不敏感 / 无 xattr / 无硬链接的文件系统 | 本开发机的 microVM 内核缺模块，vfat 和 ext4-casefold 回环挂载都失败 → **必须靠真 macOS/Windows runner** |
