use super::{Tool, ToolExecuteFn};
use serde_json::{json, Value};
use std::fs;

pub fn get_read_tool() -> Tool {
    Tool {
        name: "FileReadTool".to_string(),
        description: "读取本地系统上的文件内容。\n用于获取代码文件、配置文件或者日志。\n注意：\n- 请提供需要读取的文件的绝对路径，不要使用相对路径。\n- 如果遇到过大文件（如日志），该工具只会返回前 1000 行。".to_string(),
        input_schema: json!({
            "type": "object",
            "properties": {
                "file_path": {
                    "type": "string",
                    "description": "需要读取文件的绝对路径"
                }
            },
            "required": ["file_path"]
        }),
        execute: execute_read as ToolExecuteFn,
    }
}

fn execute_read(args: Value) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<String, String>> + Send>> {
    Box::pin(async move {
        let file_path = match args.get("file_path").and_then(|v| v.as_str()) {
            Some(p) => p.to_string(),
            None => return Err("读取文件时出错：file_path 不能为空".to_string()),
        };

        // 解析工作区隔离路径
        let mut path_buf = std::path::PathBuf::from(&file_path);
        if !path_buf.is_absolute() {
            let current_dir = std::env::current_dir().unwrap_or_default();
            let workspace_dir = current_dir.parent().unwrap_or(&current_dir).join("test_file");
            path_buf = workspace_dir.join(file_path.clone());
        }

        match fs::read_to_string(&path_buf) {
            Ok(content) => {
                let lines: Vec<&str> = content.lines().collect();
                if lines.len() > 1000 {
                    let truncated = lines[0..1000].join("\n");
                    Ok(format!("{}\n\n... (文件已截断，仅显示前 1000 行)", truncated))
                } else {
                    Ok(content)
                }
            }
            Err(e) => Err(format!("读取文件时出错：{}", e)),
        }
    })
}

pub fn get_write_tool() -> Tool {
    Tool {
        name: "FileWriteTool".to_string(),
        description: "将内容写入到指定文件。\n注意：\n- 当你打算新建一个文件时，你必须将 require_new 设为 true，以防意外覆盖旧文件。\n- 此操作会完全覆盖目标文件。如果要修改现有文件，请将 require_new 设为 false，并确保提供完整的更新后内容。\n- 如果目录不存在，系统会自动为你递归创建所需的父目录。".to_string(),
        input_schema: json!({
            "type": "object",
            "properties": {
                "file_path": {
                    "type": "string",
                    "description": "目标文件的路径"
                },
                "content": {
                    "type": "string",
                    "description": "要写入的完整文件内容"
                },
                "require_new": {
                    "type": "boolean",
                    "description": "安全机制：如果是新建文件，必须设为 true。如果文件已存在会报错拦截。"
                }
            },
            "required": ["file_path", "content", "require_new"]
        }),
        execute: execute_write as ToolExecuteFn,
    }
}

fn execute_write(args: Value) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<String, String>> + Send>> {
    Box::pin(async move {
        let file_path = match args.get("file_path").and_then(|v| v.as_str()) {
            Some(p) => p.to_string(),
            None => return Err("写入文件时出错：file_path 不能为空".to_string()),
        };

        let content = match args.get("content").and_then(|v| v.as_str()) {
            Some(c) => c.to_string(),
            None => return Err("写入文件时出错：content 不能为空".to_string()),
        };

        let require_new = args.get("require_new").and_then(|v| v.as_bool()).unwrap_or(false);

        // 解析工作区隔离路径
        let mut path_buf = std::path::PathBuf::from(&file_path);
        if !path_buf.is_absolute() {
            let current_dir = std::env::current_dir().unwrap_or_default();
            let workspace_dir = current_dir.parent().unwrap_or(&current_dir).join("test_file");
            path_buf = workspace_dir.join(file_path.clone());
        }

        // 安全机制：防覆盖检查
        if require_new && path_buf.exists() {
            return Ok(format!("写入失败：文件 {} 已经存在！为了保护你的旧代码不被意外覆盖，本次写入已被拒绝。如果你是想修改它，请将 require_new 设为 false。如果你想创建一个新文件，请更换一个不同的文件名。", file_path));
        }

        if let Some(parent) = path_buf.parent() {
            if let Err(e) = fs::create_dir_all(parent) {
                return Err(format!("创建目录时出错：{}", e));
            }
        }

        match fs::write(&path_buf, content) {
            Ok(_) => Ok(format!("文件写入成功：{}", file_path)),
            Err(e) => Err(format!("写入文件时出错：{}", e)),
        }
    })
}
