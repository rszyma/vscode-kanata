use std::io::{Read, Write};
use std::process::{Command, Stdio};

#[test]
fn lsp_initialize_via_stdio_works() {
    let kls_path: String =
        std::env::var("KANATA_LS_PATH").unwrap_or("./target/debug/kanata-ls".into());

    let mut child = Command::new(kls_path)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("failed to start kanata-ls");

    let mut stdin = child.stdin.take().unwrap();
    let mut stdout = child.stdout.take().unwrap();
    let mut stderr = child.stderr.take().unwrap();

    let init = r#"{
        "jsonrpc":"2.0",
        "id":1,
        "method":"initialize",
        "params":{
            "processId":null,
            "rootUri":null,
            "capabilities":{}
        }
    }"#;

    let msg = format!("Content-Length: {}\r\n\r\n{}", init.len(), init);

    stdin.write_all(msg.as_bytes()).unwrap();
    stdin.flush().unwrap();

    let mut stdout_buf = vec![0; 4096];
    let n = stdout.read(&mut stdout_buf).unwrap();
    let stdout_str = String::from_utf8_lossy(&stdout_buf[..n]);
    println!("--- stdout start ---\n{}\n... stdout end ...", stdout_str);

    let mut stderr_buf = vec![0; 4096];
    let n = stderr.read(&mut stderr_buf).unwrap();
    let stderr_str = String::from_utf8_lossy(&stderr_buf[..n]);
    println!("--- stderr start ---\n{}\n... stderr end ...", stderr_str);

    assert!(stdout_str.contains("jsonrpc"));
    assert!(stdout_str.contains("capabilities"));
    assert!(stderr_str.contains("no initializationOptions provided, using defaults"));

    let _ = child.kill();
    let _ = child.wait();
}
