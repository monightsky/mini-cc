use minicc::core::providers::anthropic_provider::AnthropicProvider;
use minicc::core::providers::{LLMProvider, OnTextResponseFn};
use std::sync::{Arc, Mutex};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

async fn start_mock_anthropic_server(sse_body: String) -> (String, tokio::task::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind test server failed");
    let addr = listener.local_addr().expect("get local addr failed");
    let base_url = format!("http://{}", addr);

    let handle = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.expect("accept failed");

        let mut buf = vec![0u8; 8192];
        let _ = socket.read(&mut buf).await;

        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nCache-Control: no-cache\r\nConnection: close\r\nContent-Length: {}\r\n\r\n{}",
            sse_body.len(),
            sse_body
        );

        socket
            .write_all(response.as_bytes())
            .await
            .expect("write response failed");
        let _ = socket.shutdown().await;
    });

    (base_url, handle)
}

#[tokio::test]
async fn test_anthropic_provider_stream_text_and_tool_call() {
    let sse_body = concat!(
        "event: content_block_delta\n",
        "data: {\"delta\":{\"type\":\"text_delta\",\"text\":\"你好\"}}\n\n",
        "event: content_block_start\n",
        "data: {\"content_block\":{\"type\":\"tool_use\",\"id\":\"toolu_1\",\"name\":\"FileWriteTool\"}}\n\n",
        "event: content_block_delta\n",
        "data: {\"delta\":{\"type\":\"input_json_delta\",\"partial_json\":\"{\\\"file_path\\\":\\\"a.txt\\\"\"}}\n\n",
        "event: content_block_delta\n",
        "data: {\"delta\":{\"type\":\"input_json_delta\",\"partial_json\":\",\\\"content\\\":\\\"hello\\\",\\\"require_new\\\":true}\"}}\n\n",
        "event: content_block_stop\n",
        "data: {}\n\n"
    )
    .to_string();

    let (base_url, server_handle) = start_mock_anthropic_server(sse_body).await;
    let mut provider =
        AnthropicProvider::new_with_base_url("test-key".to_string(), "claude-test".to_string(), base_url);

    let events: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
    let events_clone = events.clone();
    let on_text_response: OnTextResponseFn = Box::new(move |text, _| {
        events_clone.lock().expect("lock events failed").push(text);
    });

    let response = provider
        .send_message("请测试".to_string(), &on_text_response)
        .await
        .expect("send_message should succeed");

    server_handle.await.expect("mock server join failed");

    assert_eq!(response.text, "你好");
    assert_eq!(response.tool_calls.len(), 1);
    assert_eq!(response.tool_calls[0].id, "toolu_1");
    assert_eq!(response.tool_calls[0].name, "FileWriteTool");
    assert_eq!(response.tool_calls[0].args["file_path"], "a.txt");
    assert_eq!(response.tool_calls[0].args["content"], "hello");
    assert_eq!(response.tool_calls[0].args["require_new"], true);

    let logged = events.lock().expect("lock events failed");
    assert!(logged.iter().any(|t| t.contains("你好")));
}

#[tokio::test]
async fn test_anthropic_provider_tool_args_parse_error() {
    let sse_body = concat!(
        "event: content_block_start\n",
        "data: {\"content_block\":{\"type\":\"tool_use\",\"id\":\"toolu_bad\",\"name\":\"BashTool\"}}\n\n",
        "event: content_block_delta\n",
        "data: {\"delta\":{\"type\":\"input_json_delta\",\"partial_json\":\"{bad-json\"}}\n\n",
        "event: content_block_stop\n",
        "data: {}\n\n"
    )
    .to_string();

    let (base_url, server_handle) = start_mock_anthropic_server(sse_body).await;
    let mut provider =
        AnthropicProvider::new_with_base_url("test-key".to_string(), "claude-test".to_string(), base_url);

    let on_text_response: OnTextResponseFn = Box::new(|_, _| {});
    let response = provider
        .send_message("请测试".to_string(), &on_text_response)
        .await
        .expect("send_message should succeed");

    server_handle.await.expect("mock server join failed");

    assert_eq!(response.tool_calls.len(), 1);
    assert_eq!(response.tool_calls[0].id, "toolu_bad");
    assert_eq!(response.tool_calls[0].name, "BashTool");
    assert_eq!(response.tool_calls[0].args["_parse_error"], true);
}
