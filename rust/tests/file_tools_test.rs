use minicc::tools::file_tools::{get_read_tool, get_write_tool};
use serde_json::json;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

fn unique_test_file_path(name: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time went backwards")
        .as_nanos();
    std::env::temp_dir()
        .join("mini_cc_rust_tests")
        .join(format!("{}_{}", name, nanos))
}

#[tokio::test]
async fn test_file_write_require_new_blocks_overwrite() {
    let write_tool = get_write_tool();
    let file_path = unique_test_file_path("require_new")
        .join("index.html")
        .to_string_lossy()
        .to_string();

    let first = (write_tool.execute)(json!({
        "file_path": file_path,
        "content": "<h1>v1</h1>",
        "require_new": true
    }))
    .await
    .expect("first write should succeed");
    assert!(first.contains("文件写入成功"));

    let second = (write_tool.execute)(json!({
        "file_path": file_path,
        "content": "<h1>v2</h1>",
        "require_new": true
    }))
    .await
    .expect("tool should return blocked message");
    assert!(second.contains("写入失败"));
    assert!(second.contains("已经存在"));
}

#[tokio::test]
async fn test_file_write_and_read_roundtrip() {
    let write_tool = get_write_tool();
    let read_tool = get_read_tool();
    let file_path = unique_test_file_path("roundtrip")
        .join("hello.txt")
        .to_string_lossy()
        .to_string();

    (write_tool.execute)(json!({
        "file_path": file_path,
        "content": "hello mini-cc",
        "require_new": true
    }))
    .await
    .expect("write should succeed");

    let content = (read_tool.execute)(json!({
        "file_path": file_path
    }))
    .await
    .expect("read should succeed");
    assert_eq!(content, "hello mini-cc");
}
