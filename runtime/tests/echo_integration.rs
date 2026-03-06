//! Integration test for the echo handler.
//!
//! Spawns the arena-runtime with the guest WASM, sends a request, and asserts
//! the echoed response. Enforces a 15-second max duration to fail fast.

use std::env;
use std::io::Read;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

/// Path to the arena-runtime binary.
fn runtime_binary() -> PathBuf {
    if let Ok(p) = env::var("CARGO_BIN_EXE_arena_runtime") {
        return PathBuf::from(p);
    }
    let target_dir = env::var("CARGO_TARGET_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../target"));
    target_dir.join("debug/arena-runtime")
}

/// Workspace root (for current_dir when spawning the runtime).
fn workspace_root() -> PathBuf {
    let bin = runtime_binary();
    // Binary is at workspace/target/debug/arena-runtime
    bin.parent()
        .and_then(|p| p.parent())
        .and_then(|p| p.parent())
        .expect("binary path has parent dirs")
        .to_path_buf()
}

/// Path to the guest WASM, derived from the binary's target directory.
fn guest_wasm() -> PathBuf {
    let bin = runtime_binary();
    // Binary is at target/debug/arena-runtime, wasm at target/wasm32-wasip2/debug/arena_guest.wasm
    let target_dir = bin
        .parent()
        .and_then(|p| p.parent())
        .expect("binary path has parent dirs");
    target_dir
        .join("wasm32-wasip2")
        .join("debug")
        .join("arena_guest.wasm")
}

/// Poll until the server responds or timeout.
fn wait_for_server(timeout: Duration) -> bool {
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(1))
        .build()
        .unwrap();
    let start = std::time::Instant::now();
    while start.elapsed() < timeout {
        if client.get("http://127.0.0.1:8080/").send().is_ok() {
            return true;
        }
        thread::sleep(Duration::from_millis(50));
    }
    false
}

const TEST_TIMEOUT: Duration = Duration::from_secs(15);

#[test]
fn echo_returns_request_details() {
    let (tx, rx) = mpsc::channel();
    let test_handle = thread::spawn(move || {
        let result = std::panic::catch_unwind(|| run_echo_test());
        tx.send(result).unwrap();
    });
    match rx.recv_timeout(TEST_TIMEOUT) {
        Ok(Ok(())) => {}
        Ok(Err(e)) => std::panic::resume_unwind(e),
        Err(mpsc::RecvTimeoutError::Timeout) => {
            panic!("Integration test exceeded {}s timeout", TEST_TIMEOUT.as_secs());
        }
        Err(mpsc::RecvTimeoutError::Disconnected) => {
            let _ = test_handle.join();
        }
    }
}

fn run_echo_test() {
    let bin = runtime_binary();
    let wasm = guest_wasm();

    if !bin.exists() || !wasm.exists() {
        panic!(
            "Build artifacts missing. Run:\n  cargo build -p arena-guest --target wasm32-wasip2\n  cargo build -p arena-runtime"
        );
    }

    let mut child = Command::new(&bin)
        .args(["run", wasm.to_str().unwrap()])
        .current_dir(workspace_root())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to spawn arena-runtime");

    let server_timeout = Duration::from_secs(5);
    if !wait_for_server(server_timeout) {
        let stderr = child
            .stderr
            .as_mut()
            .and_then(|s| {
                let mut buf = String::new();
                s.read_to_string(&mut buf).ok()?;
                Some(buf)
            })
            .unwrap_or_default();
        let _ = child.kill();
        let _ = child.wait();
        panic!(
            "Server did not become ready within {:?}. Runtime stderr:\n{}",
            server_timeout, stderr
        );
    }

    // Ensure child is killed on panic or early return
    let _guard = ChildGuard(&mut child);

    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(3))
        .build()
        .expect("Failed to create HTTP client");

    let response = client
        .post("http://127.0.0.1:8080/echo/test")
        .header("X-Custom-Header", "test-value")
        .body(r#"{"hello":"world"}"#)
        .send()
        .expect("HTTP request failed");

    assert!(response.status().is_success(), "Expected 2xx, got {}", response.status());

    let json: serde_json::Value = response.json().expect("Response is not JSON");

    assert_eq!(json["method"], "POST");
    assert_eq!(json["path"], "/echo/test");
    assert_eq!(json["headers"]["x-custom-header"], "test-value");
    assert_eq!(json["body"], r#"{"hello":"world"}"#);
}

/// Kills the child process when dropped.
struct ChildGuard<'a>(&'a mut Child);

impl Drop for ChildGuard<'_> {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}
