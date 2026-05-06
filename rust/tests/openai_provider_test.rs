use minicc::core::providers::openai_provider::OpenAIProvider;
use minicc::core::providers::{LLMProvider, OnTextResponseFn};
use std::sync::{Arc, Mutex};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

async fn start_mock_openai_server(sse_body: String) -> (String, tokio::task::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind test server failed");
    let addr = listener.local_addr().expect("get local addr failed");
    let base_url = format!("http://{}/v1", addr);

    let handle = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.expect("accept failed");

        // 读取请求头，避免客户端在我们提前响应时出现连接异常
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
async fn test_openai_provider_stream_text_reasoning_and_tool_calls() {
    let sse_body = concat!(
        "data: {\"choices\":[{\"delta\":{\"reasoning_content\":\"思考中...\"}}]}\n\n",
        "data: {\"choices\":[{\"delta\":{\"content\":\"你好\"}}]}\n\n",
        "data: {\"choices\":[{\"delta\":{\"tool_calls\":[{\"index\":0,\"id\":\"call_1\",\"function\":{\"name\":\"FileWriteTool\",\"arguments\":\"{\\\"file_path\\\":\\\"a.txt\\\"\"}}]}}]}\n\n",
        "data: {\"choices\":[{\"delta\":{\"tool_calls\":[{\"index\":0,\"function\":{\"name\":\"\",\"arguments\":\",\\\"content\\\":\\\"hello\\\",\\\"require_new\\\":true}\"}}]}}]}\n\n",
        "data: [DONE]\n\n"
    )
    .to_string();

    let (base_url, server_handle) = start_mock_openai_server(sse_body).await;
    let mut provider = OpenAIProvider::new("test-key".to_string(), base_url, "qwen-test".to_string());

    let events: Arc<Mutex<Vec<(String, bool)>>> = Arc::new(Mutex::new(Vec::new()));
    let events_clone = events.clone();
    let on_text_response: OnTextResponseFn = Box::new(move |text, is_thinking| {
        events_clone
            .lock()
            .expect("lock events failed")
            .push((text, is_thinking));
    });

    let response = provider
        .send_message("请测试".to_string(), &on_text_response)
        .await
        .expect("send_message should succeed");

    server_handle.await.expect("mock server join failed");

    assert_eq!(response.text, "你好");
    assert_eq!(response.tool_calls.len(), 1);
    assert_eq!(response.tool_calls[0].id, "call_1");
    assert_eq!(response.tool_calls[0].name, "FileWriteTool");
    assert_eq!(response.tool_calls[0].args["file_path"], "a.txt");
    assert_eq!(response.tool_calls[0].args["content"], "hello");
    assert_eq!(response.tool_calls[0].args["require_new"], true);

    let logged = events.lock().expect("lock events failed");
    assert!(logged.iter().any(|(t, thinking)| *thinking && t.contains("思考中")));
    assert!(logged.iter().any(|(t, thinking)| !*thinking && t.contains("你好")));
}

#[tokio::test]
async fn test_openai_provider_tool_arguments_parse_error() {
    let sse_body = concat!(
        "data: {\"choices\":[{\"delta\":{\"tool_calls\":[{\"index\":0,\"id\":\"call_bad\",\"function\":{\"name\":\"BashTool\",\"arguments\":\"{bad-json\"}}]}}]}\n\n",
        "data: [DONE]\n\n"
    )
    .to_string();

    let (base_url, server_handle) = start_mock_openai_server(sse_body).await;
    let mut provider = OpenAIProvider::new("test-key".to_string(), base_url, "qwen-test".to_string());

    let on_text_response: OnTextResponseFn = Box::new(|_, _| {});
    let response = provider
        .send_message("请测试".to_string(), &on_text_response)
        .await
        .expect("send_message should succeed");

    server_handle.await.expect("mock server join failed");

    assert_eq!(response.tool_calls.len(), 1);
    assert_eq!(response.tool_calls[0].id, "call_bad");
    assert_eq!(response.tool_calls[0].name, "BashTool");
    assert_eq!(response.tool_calls[0].args["_parse_error"], true);
}
