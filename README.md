# tkm — 统一 Token 管理工具

[English](#english) | 中文

一个用 Rust 编写的终端 Token 管理工具，用一个主密码管理所有开发平台的 Token。类似 1Password，但专为开发者设计。

## 特性

- **双存储后端** — macOS Keychain（原生安全）+ AES-256-GCM 加密文件（跨平台）
- **TUI 交互界面** — 基于 ratatui，支持列表浏览、搜索、新增、删除、复制
- **CLI 命令行** — `tkm get github` 一条命令获取 Token，可嵌入脚本
- **一键导入** — 自动扫描 gh / glab / Docker / Gradle / Pencil 已有凭证
- **安全优先** — Argon2id 密钥派生，secrecy + zeroize 内存安全，剪贴板 30 秒自动清除
- **批量导出** — `eval $(tkm env github nexus)` 一次导出多个环境变量

## 安装

```bash
# 从源码编译
git clone https://github.com/hgDendi/tkm.git
cd tkm
cargo build --release

# 将二进制文件放到 PATH 中
cp target/release/tkm ~/.cargo/bin/
```

## 快速开始

```bash
# 1. 初始化（设置主密码）
tkm init

# 2. 添加 Token
tkm set github                          # 交互式输入
tkm set nexus --backend keychain        # 存入系统 Keychain

# 3. 获取 Token
tkm get github                          # 输出到 stdout
tkm get github --clip                   # 复制到剪贴板
export GITHUB_TOKEN=$(tkm get github)   # 注入环境变量

# 4. 从已有工具导入
tkm import all                          # 扫描所有来源
tkm import gh                           # 仅导入 GitHub CLI

# 5. 批量导出
eval $(tkm env github nexus-maven)

# 6. 启动 TUI
tkm                                     # 直接运行即启动
```

## CLI 命令

```
tkm                    启动 TUI 界面
tkm init               初始化：设置主密码，创建加密保险库
tkm get <service>      获取 Token（--clip 复制 / --json 输出 JSON）
tkm set <service>      添加或更新 Token（--backend keychain|file）
tkm rm <service>       删除 Token
tkm list               列出所有 Token 元数据（不含密钥值）
tkm import <source>    导入：gh / glab / docker / gradle / pencil / all
tkm env <service>...   输出 eval 式 export 语句
tkm lock               锁定会话
tkm passwd             修改主密码
```

## TUI 快捷键

| 按键 | 功能 |
|------|------|
| `j` / `k` / `↑` / `↓` | 上下导航 |
| `Enter` | 查看详情 |
| `a` | 新增 Token |
| `d` | 删除 Token |
| `c` | 复制到剪贴板 |
| `/` | 搜索 |
| `v` | 显示 / 隐藏密钥值（详情页） |
| `q` / `Ctrl+C` | 退出 |

## 架构

```
tkm/
├── crypto/          Argon2id KDF + AES-256-GCM 加解密
├── core/            Token 模型 + Registry（明文索引）
├── storage/         StorageBackend trait
│   ├── keychain     macOS Keychain 后端
│   └── encrypted_file  加密文件后端
├── cli/             clap CLI 命令
├── tui/             ratatui TUI 界面
└── integrations/    gh / glab / docker / gradle / pencil 导入
```

**存储设计**：密钥值存储在 Keychain 或加密文件中，元数据（服务名、标签、过期时间）以明文 TOML 存储在 `~/.tkm/registry.toml`，实现快速列表查询而无需解密。

## 安全模型

| 组件 | 方案 |
|------|------|
| 密钥派生 | Argon2id（64MB / 3 次迭代 / 4 并行） |
| 加密算法 | AES-256-GCM（12B nonce + 16B auth tag） |
| 内存安全 | `secrecy::SecretString` + `zeroize` |
| 剪贴板 | 复制后 30 秒自动清除 |
| 存储 | Keychain 由 OS 保护；文件后端全量加密 |

## 许可证

MIT

---

<a id="english"></a>

# tkm — Unified Token Manager

[中文](#) | English

A terminal-based token manager written in Rust. One master password to manage all your developer tokens — like 1Password, but built for developers.

## Features

- **Dual storage backends** — macOS Keychain (native security) + AES-256-GCM encrypted file (cross-platform)
- **TUI interface** — Built with ratatui: browse, search, add, delete, copy tokens
- **CLI first** — `tkm get github` in scripts, pipes, and shell substitutions
- **One-click import** — Auto-scan credentials from gh / glab / Docker / Gradle / Pencil
- **Security first** — Argon2id KDF, secrecy + zeroize for memory safety, 30s clipboard auto-clear
- **Bulk export** — `eval $(tkm env github nexus)` to export multiple env vars at once

## Installation

```bash
# Build from source
git clone https://github.com/hgDendi/tkm.git
cd tkm
cargo build --release

# Add to PATH
cp target/release/tkm ~/.cargo/bin/
```

## Quick Start

```bash
# 1. Initialize (set master password)
tkm init

# 2. Add tokens
tkm set github                          # interactive input
tkm set nexus --backend keychain        # store in system Keychain

# 3. Retrieve tokens
tkm get github                          # print to stdout
tkm get github --clip                   # copy to clipboard
export GITHUB_TOKEN=$(tkm get github)   # inject into env

# 4. Import from existing tools
tkm import all                          # scan all sources
tkm import gh                           # GitHub CLI only

# 5. Bulk export
eval $(tkm env github nexus-maven)

# 6. Launch TUI
tkm                                     # run without args
```

## CLI Commands

```
tkm                    Launch TUI interface
tkm init               Initialize: set master password, create vault
tkm get <service>      Get token (--clip to copy / --json for JSON)
tkm set <service>      Add or update token (--backend keychain|file)
tkm rm <service>       Remove a token
tkm list               List all token metadata (no secret values)
tkm import <source>    Import: gh / glab / docker / gradle / pencil / all
tkm env <service>...   Print eval-able export statements
tkm lock               Lock the session
tkm passwd             Change master password
```

## TUI Keybindings

| Key | Action |
|-----|--------|
| `j` / `k` / `↑` / `↓` | Navigate |
| `Enter` | View details |
| `a` | Add token |
| `d` | Delete token |
| `c` | Copy to clipboard |
| `/` | Search |
| `v` | Reveal / hide secret (detail view) |
| `q` / `Ctrl+C` | Quit |

## Architecture

```
tkm/
├── crypto/          Argon2id KDF + AES-256-GCM encryption
├── core/            Token model + Registry (plaintext index)
├── storage/         StorageBackend trait
│   ├── keychain     macOS Keychain backend
│   └── encrypted_file  Encrypted file backend
├── cli/             clap CLI commands
├── tui/             ratatui TUI interface
└── integrations/    gh / glab / docker / gradle / pencil importers
```

**Storage design**: Secret values live in Keychain or encrypted files. Metadata (service name, tags, expiry) is stored as plaintext TOML in `~/.tkm/registry.toml` for fast listing without decryption.

## Security Model

| Component | Approach |
|-----------|----------|
| Key derivation | Argon2id (64MB / 3 iterations / 4 parallelism) |
| Encryption | AES-256-GCM (12B nonce + 16B auth tag) |
| Memory safety | `secrecy::SecretString` + `zeroize` |
| Clipboard | Auto-clear after 30 seconds |
| Storage | Keychain protected by OS; file backend fully encrypted |

## License

MIT
