use minicc::tools::bash_tool::get_tool;
use serde_json::json;

#[tokio::test]
async fn test_bash_tool_blocks_dangerous_command_variants() {
    let tool = get_tool();

    let result = (tool.execute)(json!({
        "command": "sudo timeout 10 watch -n 1 rm -f -r /"
    }))
    .await
    .expect("tool should return intercepted message");

    assert!(result.contains("安全沙盒已拦截"));
}

#[tokio::test]
async fn test_bash_tool_executes_safe_command() {
    let tool = get_tool();

    let result = (tool.execute)(json!({
        "command": "echo rust_test_ok"
    }))
    .await
    .expect("safe command should execute");

    assert!(result.contains("rust_test_ok"));
}
