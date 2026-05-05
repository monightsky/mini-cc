# 🦀 mini-cc (Rust Edition)

这是 `mini-cc` 智能体的 **Rust 语言实现版本**。

基于 Rust 的“所有权”、“内存安全”以及“无畏并发”理念，本项目不仅提供了原生二进制级别的高性能执行效率，也通过 `tokio`、`reqwest` 异步框架实现了极致的流式流转与事件响应。

在实现上，Rust 版本延续了项目“极简”、“纯函数式架构”的基因。

我们自己手动实现了 SSE (Server-Sent Events) 的流式解析，从而深度接管大模型的 Tool Use 与 Reasoning Content 推理。

## ✨ 已实现功能

1. **强安全与零开销抽象**：避免了 `class` 以及繁重的面向对象系统，状态安全由生命周期与借用检查器严格保证。
2. **多模型 SSE 流式输出**：通过 `eventsource-stream` 解析大模型的打字机流式响应，包括思维链。
3. **Agent 核心闭环 (Tool Use)**：
   - `BashTool`：利用 `std::process::Command` 高效并安全地执行外壳命令。支持多层子 Shell 及包装命令拦截。
   - `FileReadTool`：借用 `std::fs` 实现文件读取。
   - `FileWriteTool`：严格的 `require_new` 机制，防止大模型意外覆盖核心旧代码。
4. **终端交互与中文支持**：
   - 引入 `rustyline` 支持上下键调出历史记录与命令回溯。
   - 彻底修复终端下的 **全角中文标点防错位 Bug**（基于 `RUNEWIDTH_EASTASIAN=1`），保证删除光标精准。
5. **交互式配置向导**：首次启动自动引导小白用户配置 API Key，告别报错与繁琐手动配表。

## 🚀 安装与使用

### 方法一：全局一键安装 (推荐)

只要你的电脑上安装了 [Rust 环境](https://rustup.rs/) (1.70+)，你可以在终端中随时随地通过以下命令直接全局安装 `mini-cc`：

```bash
cargo install --git https://github.com/you-want/mini-cc.git --bin minicc
```

安装完成后，你可以在任何目录下直接输入命令唤醒 AI 编程助手：

```bash
minicc
```

### 方法二：源码克隆与运行

如果你希望阅读、学习甚至魔改它的源码：

```bash
# 1. 克隆代码
git clone https://github.com/you-want/mini-cc.git
cd mini-cc/rust

# 2. 本地运行体验
cargo run

# 3. 编译出最高性能的 release 版本
cargo build --release
# 执行编译出的二进制文件
./target/release/minicc
```

### 环境配置机制

`mini-cc` 采用**双层回退加载机制**，极大地提升了作为全局工具使用的便捷性：
1. **全局默认配置**：首次运行时如果没有检测到 API Key，会自动进入向导模式引导输入，并将配置保存在你电脑的主目录下 (`~/.mini-cc-env`)。
2. **局部项目覆盖**：如果在你当前执行 `minicc` 命令的代码项目目录下存在 `.env` 文件，它将具有最高优先级，覆盖全局配置。
