# 0005 · 默认就地写入保存，崩溃安全另外买

**状态**：proposed

## 决策

- 默认 `save.strategy = "rewrite"`：打开已有 fd、truncate、写、`fdatasync`。**保留 inode。**
- 崩溃安全用**写前日志**买：`<state_dir>/swap/<hash>.soki`，每约 200 次击键或 4 秒空闲 fsync
- 提供 `"replace"`（原子替换）和 `"auto"`（vim `bufwrite.c` 启发式）

## 理由

**世界上最常用的两个编辑器默认都保留 inode**：VS Code 的 `pfs.ts` 默认是 truncate + write + `fdatasync`，
原子 rename 是每次调用显式 opt-in 且对符号链接直接拒绝；vim 的默认是 `backupcopy=auto`
（注意不是 Unix 上的 `yes`，那是 *Vi* 的默认）。

原子替换（rename）的四宗罪，全部实测验证：

1. rename 到只读文件**会成功**并抹掉只读位（`chmod 444` 不再有保护作用）
2. rename 到符号链接会**用普通文件替换掉软链** → 击穿所有 dotfile 管理器
3. 断硬链接、丢 xattr / ACL / SELinux label / macOS Finder 标签与资源分叉
4. 换掉 inode → `tail -f`、`crontab -e`、Docker 单文件 bind mount、别人的文件监听全部失效

而且 rename 的原子性**扛不住 SIGKILL 和断电**，日志能扛。

## 后果

- `tempfile::NamedTempFile` **禁止用于保存用户文件**（实测会把权限降成 0600、丢 xattr、断硬链接）
- Windows 上任何 rename 都要对 `raw_os_error` 5 和 32（`ERROR_SHARING_VIOLATION`）做退避重试再回退就地写。
  **GitHub 的 Windows runner 基本永远复现不了这个** → 靠设计保证，不靠 CI 保证
- 做原子替换时，Windows 用 `ReplaceFileW`（保留 ACL 和创建时间），macOS 用 `renamex_np(RENAME_SWAP)`
- 临时文件必须和目标同目录（跨设备 rename 会 EXDEV）

## 什么会推翻它

SokiText 主要面向超大文件（truncate 窗口期崩溃的概率高于用户在意元数据的概率）。
