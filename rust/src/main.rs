use dotenv::dotenv;
use minicc::core::agent::Agent;
use minicc::core::providers::openai_provider::OpenAIProvider;
use minicc::core::providers::anthropic_provider::AnthropicProvider;
use minicc::core::providers::LLMProvider;
use minicc::buddy::companion::spawn_buddy;
use std::env;
use std::io::{self, Write};
use std::path::PathBuf;
use rustyline::{error::ReadlineError, DefaultEditor};
use tokio;

/// 交互式配置向导：如果缺失 API Key，自动引导用户配置并保存到 ~/.mini-cc-env
fn interactive_config(global_env_path: &PathBuf) {
    println!("\x1b[33m⚠️ 未检测到 API Key，进入初始化配置向导...\x1b[0m");

    let mut rl = DefaultEditor::new().unwrap();

    let api_key = match rl.readline("? 请输入您的 OPENAI_API_KEY: ") {
        Ok(line) => line.trim().to_string(),
        Err(_) => String::new(),
    };

    if api_key.is_empty() {
        println!("\x1b[31mAPI Key 不能为空，已退出。\x1b[0m");
        std::process::exit(1);
    }

    let model_name = match rl.readline("? 请输入模型名称 (默认: qwen3.6-plus): ") {
        Ok(line) => {
            let line = line.trim();
            if line.is_empty() { "qwen3.6-plus".to_string() } else { line.to_string() }
        },
        Err(_) => "qwen3.6-plus".to_string(),
    };

    let base_url = match rl.readline("? 如果您使用的是兼容接口，请输入 BASE_URL (可选, 默认: https://api.openai.com/v1): ") {
        Ok(line) => {
            let line = line.trim();
            if line.is_empty() { "https://api.openai.com/v1".to_string() } else { line.to_string() }
        },
        Err(_) => "https://api.openai.com/v1".to_string(),
    };

    let content = format!("OPENAI_API_KEY={}\nMODEL_NAME={}\nOPENAI_BASE_URL={}\n", api_key, model_name, base_url);
    if let Err(e) = std::fs::write(global_env_path, content) {
        println!("\x1b[31m保存配置文件失败: {}\x1b[0m", e);
        std::process::exit(1);
    }

    println!("\x1b[32m✓ 配置已成功保存至 {}\x1b[0m", global_env_path.display());

    env::set_var("OPENAI_API_KEY", &api_key);
    env::set_var("MODEL_NAME", &model_name);
    env::set_var("OPENAI_BASE_URL", &base_url);
}

fn print_banner(version: &str, model: &str) {
    let user_name = env::var("USER").unwrap_or_else(|_| "developer".to_string());
    let cwd = env::current_dir().unwrap_or_default().display().to_string();
    let home = env::var("HOME").unwrap_or_default();
    let display_cwd = if cwd.starts_with(&home) {
        cwd.replacen(&home, "~", 1)
    } else {
        cwd
    };

    let truncate = |s: &str, len: usize| -> String {
        let chars: Vec<char> = s.chars().collect();
        if chars.len() > len {
            let mut truncated: String = chars.into_iter().take(len - 3).collect();
            truncated.push_str("...");
            truncated
        } else {
            s.to_string()
        }
    };

    let u_disp = truncate(&user_name, 15);
    let m_disp = truncate(model, 22);
    let c_disp = truncate(&display_cwd, 35);
    let provider_display = "OpenAI / Compatible";

    // 修复排版：使用纯文本计算宽度，并用空格填充，防止 ANSI 颜色转义符干扰计算
    // 总宽度假设为 82，减去左右边框等
    let header_line = format!("mini-cc CLI {} (Rust Edition)", version);
    let header_pad = " ".repeat(46usize.saturating_sub(header_line.chars().count()));
    
    let u_disp_line = format!("Welcome back, {}", u_disp);
    let u_disp_pad = " ".repeat(40usize.saturating_sub(u_disp_line.chars().count()));
    
    let m_disp_line = format!("Model: {}", m_disp);
    let m_disp_pad = " ".repeat(40usize.saturating_sub(m_disp_line.chars().count()));
    
    let p_disp_line = format!("Provider: {}", provider_display);
    let p_disp_pad = " ".repeat(40usize.saturating_sub(p_disp_line.chars().count()));
    
    let c_disp_pad = " ".repeat(40usize.saturating_sub(c_disp.chars().count()));

    println!("\x1b[38;2;204;255;0m╭────────────────────────────────────────────────────────────────────────────────────╮\x1b[0m");
    println!("\x1b[38;2;204;255;0m│\x1b[0m \x1b[36m\x1b[1mmini-cc CLI {} (Rust Edition)\x1b[0m{} \x1b[38;2;204;255;0m│\x1b[0m", version, header_pad);
    println!("\x1b[38;2;204;255;0m│\x1b[0m \x1b[38;2;204;255;0m▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄\x1b[0m{:<65} \x1b[38;2;204;255;0m│\x1b[0m", "");
    println!("\x1b[38;2;204;255;0m│\x1b[0m \x1b[38;2;204;255;0m█\x1b[48;2;5;5;5m               \x1b[0m\x1b[38;2;204;255;0m█\x1b[0m \x1b[36m\x1b[1mAnnouncements\x1b[0m{:<49} \x1b[38;2;204;255;0m│\x1b[0m", "");
    println!("\x1b[38;2;204;255;0m│\x1b[0m \x1b[38;2;204;255;0m█\x1b[48;2;5;5;5m  \x1b[0m\x1b[38;2;204;255;0m\x1b[1mcc\x1b[0m\x1b[48;2;5;5;5m       \x1b[38;2;229;229;229m■\x1b[0m\x1b[48;2;5;5;5m   \x1b[0m\x1b[38;2;204;255;0m█\x1b[0m Try MINI-CC{:<51} \x1b[38;2;204;255;0m│\x1b[0m", "");
    println!("\x1b[38;2;204;255;0m│\x1b[0m \x1b[38;2;204;255;0m█\x1b[48;2;5;5;5m               \x1b[0m\x1b[38;2;204;255;0m█\x1b[0m Website: \x1b[34m\x1b[4mhttps://mini-cc.raingpt.top/\x1b[0m{:<25} \x1b[38;2;204;255;0m│\x1b[0m", "");
    println!("\x1b[38;2;204;255;0m│\x1b[0m \x1b[38;2;204;255;0m▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀\x1b[0m Github: \x1b[34m\x1b[4mhttps://github.com/you-want/mini-cc\x1b[0m{:<19} \x1b[38;2;204;255;0m│\x1b[0m", "");
    println!("\x1b[38;2;204;255;0m│\x1b[0m {:<82} \x1b[38;2;204;255;0m│\x1b[0m", "");
    println!("\x1b[38;2;204;255;0m│\x1b[0m \x1b[36mWelcome back, {}\x1b[0m{} \x1b[90m────────────────────────────────────────\x1b[0m \x1b[38;2;204;255;0m│\x1b[0m", u_disp, u_disp_pad);
    println!("\x1b[38;2;204;255;0m│\x1b[0m \x1b[36mModel: {}\x1b[0m{} \x1b[36m\x1b[1mDid you know?\x1b[0m                           \x1b[38;2;204;255;0m│\x1b[0m", m_disp, m_disp_pad);
    println!("\x1b[38;2;204;255;0m│\x1b[0m \x1b[36mProvider: {}\x1b[0m{} You can use \x1b[33m/buddy\x1b[0m to summon a pet!     \x1b[38;2;204;255;0m│\x1b[0m", provider_display, p_disp_pad);
    println!("\x1b[38;2;204;255;0m│\x1b[0m \x1b[90m{}\x1b[0m{} Type \x1b[33m/clear\x1b[0m to clear context.           \x1b[38;2;204;255;0m│\x1b[0m", c_disp, c_disp_pad);
    println!("\x1b[38;2;204;255;0m╰────────────────────────────────────────────────────────────────────────────────────╯\x1b[0m\n");
}

#[tokio::main]
async fn main() {
    // 加载 .env 文件中的环境变量
    dotenv().ok();
    
    // 兜底加载用户主目录下的 ~/.mini-cc-env 全局配置，方便全局使用 CLI
    let home = env::var("HOME").unwrap_or_default();
    let global_env_path = PathBuf::from(&home).join(".mini-cc-env");
    if global_env_path.exists() {
        dotenv::from_path(&global_env_path).ok();
    }

    let args: Vec<String> = env::args().collect();
    if args.len() == 2 && (args[1] == "--version" || args[1] == "-v") {
        println!("mini-cc rust v1.0.0");
        return;
    }

    // 检查 API Key，如果没有则启动交互式向导引导用户填写
    let mut openai_api_key = env::var("OPENAI_API_KEY").unwrap_or_default();
    if openai_api_key.is_empty() {
        interactive_config(&global_env_path);
        openai_api_key = env::var("OPENAI_API_KEY").unwrap_or_default();
    }

    // 解析并获取其他的环境配置信息
    let provider_name = env::var("PROVIDER").unwrap_or_else(|_| "openai".to_string());
    let anthropic_api_key = env::var("ANTHROPIC_API_KEY").unwrap_or_default();
    let base_url = env::var("OPENAI_BASE_URL").unwrap_or_default();
    let model = env::var("MODEL_NAME").unwrap_or_else(|_| "qwen3.6-plus".to_string());

    // 根据配置初始化相应的 LLM Provider (大模型接口)
    let provider: Box<dyn LLMProvider> = if provider_name.to_lowercase() == "anthropic" || (!anthropic_api_key.is_empty() && model.contains("claude")) {
        Box::new(AnthropicProvider::new(anthropic_api_key.clone(), model.clone()))
    } else {
        Box::new(OpenAIProvider::new(openai_api_key.clone(), base_url.clone(), model.clone()))
    };

    // 这是一个非常关键的修复：
    // 在中文（CJK）环境下，像 “（”、“）”、“：” 这种全角标点符号在终端中占据 2 个字符宽度。
    // 但底层库 go-runewidth / unicode-width 默认把它们当做 1 个宽度（Ambiguous width 歧义宽度）。
    // 强行设置 RUNEWIDTH_EASTASIAN=1 可以让底层正确识别全角标点，彻底解决终端光标错位、文字重叠的问题。
    env::set_var("RUNEWIDTH_EASTASIAN", "1");

    // 打印带有炫酷色彩的终端横幅
    print_banner("1.0.0", &model);
    
    // 实例化核心 Agent
    let mut agent = Agent::new(provider);

    // 初始化 Rustyline 用于终端交互，它提供了类似 Bash 的上下键历史记录、光标左右移动功能
    let mut rl = DefaultEditor::new().unwrap();
    let history_file = PathBuf::from(&home).join(".mini-cc_history");
    let _ = rl.load_history(&history_file);

    // REPL (Read-Eval-Print Loop) 主对话循环
    loop {
        // 注意：Prompt 需要避免包含 ANSI 颜色转义符，以防止 readline 在计算光标长度时出错
        let readline = rl.readline("mini-cc> ");
        match readline {
            Ok(line) => {
                let user_input = line.trim();
                if user_input.is_empty() {
                    continue;
                }

                // 将有效输入加入历史记录并保存到磁盘
                rl.add_history_entry(user_input).unwrap();
                let _ = rl.save_history(&history_file);

                let lower_input = user_input.to_lowercase();
                // 拦截并处理退出命令
                if lower_input == "exit" || lower_input == "quit" || lower_input == "/exit" {
                    println!("\x1b[32m再见！\x1b[0m");
                    break;
                }

                // 拦截并处理帮助命令
                if lower_input == "/help" {
                    println!("\x1b[36m\n=== 可用命令 ===\x1b[0m");
                    println!("\x1b[90m  /help     - 显示此帮助信息\x1b[0m");
                    println!("\x1b[90m  /clear    - 清空当前对话历史\x1b[0m");
                    println!("\x1b[90m  /buddy    - 召唤一只专属电子宠物\x1b[0m");
                    println!("\x1b[90m  /exit     - 退出程序\x1b[0m");
                    println!("\x1b[36m==============\n\x1b[0m");
                    continue;
                }

                // 处理电子宠物彩蛋命令
                if lower_input.starts_with("/buddy") {
                    let parts: Vec<&str> = user_input.splitn(2, ' ').collect();
                    let seed = if parts.len() > 1 { Some(parts[1]) } else { None };
                    spawn_buddy(seed);
                    continue;
                }

                // 处理清除对话历史命令（通过实例化一个新的 Agent 即可清空上下文）
                if lower_input == "/clear" {
                    let p: Box<dyn LLMProvider> = if provider_name.to_lowercase() == "anthropic" || (!anthropic_api_key.is_empty() && model.contains("claude")) {
                        Box::new(AnthropicProvider::new(anthropic_api_key.clone(), model.clone()))
                    } else {
                        Box::new(OpenAIProvider::new(
                            openai_api_key.clone(),
                            base_url.clone(),
                            model.clone(),
                        ))
                    };
                    agent = Agent::new(p);
                    println!("\x1b[33m✓ 上下文已清空。\x1b[0m");
                    continue;
                }

                println!("\x1b[2m\n[Agent] 已收到指令，正在思考中...\n\x1b[0m");

                // 定义处理大模型流式输出的回调函数 (闭包)
                let on_text_response: Box<dyn Fn(String, bool) + Send + Sync> =
                    Box::new(|text: String, is_thinking: bool| {
                        if is_thinking {
                            // 思维链 (reasoning_content) 用灰色文字显示
                            print!("\x1b[90m{}\x1b[0m", text);
                        } else {
                            // 正式的普通回答用绿色文本显示
                            print!("\x1b[32m{}\x1b[0m", text);
                        }
                        io::stdout().flush().unwrap();
                    });

                // 将用户输入传递给 Agent 处理，触发 ReAct (推理与行动) 循环
                agent.chat(user_input.to_string(), on_text_response).await;
                println!();
            },
            Err(ReadlineError::Interrupted) | Err(ReadlineError::Eof) => {
                // 优雅处理 Ctrl+C (Interrupted) 或 Ctrl+D (Eof) 退出
                println!("\x1b[32m\n再见！\x1b[0m");
                break;
            },
            Err(err) => {
                println!("\x1b[31m读取输入错误: {:?}\x1b[0m", err);
                break;
            }
        }
    }
}