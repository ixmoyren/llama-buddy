# llama-buddy

> 一个快速启动和管理大语言模型 (LLM) 的命令行工具

![License](https://img.shields.io/crates/l/PROJECT.svg)

## 简介

llama-buddy 是一个用 Rust 编写的命令行工具，旨在简化本地大语言模型的管理和使用。它提供了从远程注册表拉取模型、初始化本地注册表、以及快速启动模型进行对话等功能。

## 功能特性

- 🚀 **快速启动**: 通过简单的命令即可启动本地 LLM 模型
- 📦 **模型管理**: 支持从远程注册表拉取和管理模型
- 💾 **本地注册表**: 维护本地模型元数据和配置
- 🔄 **自动更新**: 支持更新本地注册表信息
- 💬 **交互式对话**: 内置 REPL 环境，支持与模型进行交互式对话

## 安装

### 从源码构建

```bash
git clone https://github.com/yourusername/llama-buddy.git
cd llama-buddy
cargo build --release
```

构建完成后，可执行文件位于 `target/release/llama-buddy`

## 使用指南

### 1. 初始化本地注册表

在首次使用前，需要初始化本地注册表:

```bash
llama-buddy init
```

**可选参数:**

- `-r， --remote <URL>`: 指定远程注册表地址 (默认: `https://registry.ollama.com/`)
- `-p， --path <PATH>`: 指定本地注册表路径 (默认: `$DATA$/llama-buddy`)
- `-s， --save`: 将命令行参数保存到配置文件
- `--force`: 强制初始化，清除所有现有数据

**示例:**

```bash
llama-buddy init --remote https://custom-registry.com --save
```

### 2. 拉取模型

从远程注册表拉取模型到本地:

```bash
llama-buddy pull --name <模型名称>
```

**可选参数:**

- `-n， --name <NAME>`: 模型名称 (必需)
- `-c， --category <CATEGORY>`: 模型版本
- `-s， --save`: 保存配置到文件

**示例:**

```bash
llama-buddy pull --name llama3 --category latest
```

### 3. 运行模型

启动已拉取的模型进行交互式对话:

```bash
llama-buddy simple-run --name <模型名称>
```

**可选参数:**

- `-n， --name <NAME>`: 模型名称 (必需)
- `-c， --category <CATEGORY>`: 模型类别
- `-t， --text <SIZE>`: 文本上下文大小 (默认: 2048)
- `--ngl <LAYERS>`: GPU 层卸载数量 (默认: 99)

**示例:**

```bash
llama-buddy simple-run --name llama3 --text 4096 --ngl 50
```

**交互式对话:**

- 在 `Q>>` 提示符下输入问题
- 按 `Ctrl+C` 退出对话
- 按 `Ctrl+D` 结束输入

### 4. 更新本地注册表

更新本地注册表的模型信息:

```bash
llama-buddy update
```

### 5. 查看配置

输出默认配置信息:

```bash
llama-buddy config
```

## 项目结构

```
llama-buddy/
├── crates/                
│   ├── http-extra/        # HTTP 下载和重试工具
│   ├── llama-buddy-macro/ # 一些常用的宏
│   ├── llama-cpp/         # llama.cpp 封装
│   ├── llama-cpp-sys/     # llama.cpp FFI 绑定
│   ├── scalar-warpper/    # utoipa 中 scalar 包装器，用于在 axum 中展现 openapi 页
│   └── sys-extra/         # 系统相关的一些实用工具，提供识别当前编译的三元组信息，Linux/Windows/MacOS 目录规范
├── src/                   
│   ├── cmd/               # 命令实现
│   ├── config/            # 配置
│   ├── db/                # 保存模型注册表信息
│   ├── service/           # 业务
│   └── utils/             # 工具
├── Cargo.toml             # 项目配置
└── README.md              # 项目文档
```

## 技术栈

- **语言**: Rust (Edition 2024)
- **CLI 框架**: clap
- **异步运行时**: tokio
- **HTTP 客户端**: reqwest
- **数据库**: rusqlite + sqlite
- **交互式终端**: rustyline
- **日志**: tracing

## 依赖说明

核心依赖包括:

- `llama.cpp`: 用于模型推理的后端
- `reqwest`: HTTP 客户端，用于下载模型和注册表
- `rusqlite`: SQLite 数据库，用于存储元数据
- `rustyline`: 交互式命令行编辑器
- `clap`: 命令行参数解析

## 配置文件

配置文件通常位于:

- Linux: `~/.config/llama-buddy/config.toml`
- macOS: `~/Library/Application Support/llama-buddy/config.toml`
- Windows: `%APPDATA%\llama-buddy\config.toml`

## 故障排除

### 初始化失败

- 检查网络连接
- 确认远程注册表地址可访问
- 使用 `--force` 参数重新初始化

### 模型拉取失败

- 确保已完成初始化 (`llama-buddy init`)
- 检查磁盘空间是否充足
- 验证模型名称和类别是否正确

### 运行模型失败

- 确保模型已成功拉取
- 检查 GPU 驱动和 CUDA 是否正确安装 (如使用 GPU 加速)
- 调整 `--text` 参数以适应可用内存

## 许可

本项目采用双重许可，您可以选择以下任一许可:

* Apache License， Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) 或 <http://www.apache.org/licenses/LICENSE-2.0>)
* MIT License ([LICENSE-MIT](LICENSE-MIT) 或 <http://opensource.org/licenses/MIT>)

### 贡献

除非您另有明确说明，否则任何您提交的代码许可应按上述 Apache 和 MIT 双重许可，并没有任何附加条款或条件。

## 致谢

本项目基于 [llama.cpp](https://github.com/ggerganov/llama.cpp) 构建，感谢 llama.cpp 团队的出色工作。
