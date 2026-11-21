use std::{process::Stdio, time::Duration};

use serde_json::{json, Value};
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    process::{Child, ChildStdin, ChildStdout, Command},
};

struct Client {
    child: Child,
    input: ChildStdin,
    output: BufReader<ChildStdout>,
    _cwd: tempfile::TempDir,
}

impl Client {
    async fn new() -> Self {
        Self::with_args(&[]).await
    }

    async fn with_args(args: &[&str]) -> Self {
        let cwd = tempfile::tempdir().unwrap();
        let mut command = Command::new(env!("CARGO_BIN_EXE_airgap"));
        command
            .args(args)
            .current_dir(cwd.path())
            .env_clear()
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true);
        if let Ok(root) = std::env::var("SystemRoot") {
            command.env("SystemRoot", root);
        }
        let mut child = command.spawn().unwrap();
        let input = child.stdin.take().unwrap();
        let output = BufReader::new(child.stdout.take().unwrap());
        Self {
            child,
            input,
            output,
            _cwd: cwd,
        }
    }

    async fn send(&mut self, value: Value) {
        self.input
            .write_all(format!("{value}\n").as_bytes())
            .await
            .unwrap();
        self.input.flush().await.unwrap();
    }

    async fn request(&mut self, id: u64, method: &str, params: Value) -> Value {
        self.send(json!({"jsonrpc":"2.0", "id":id, "method":method, "params":params}))
            .await;
        let mut line = String::new();
        // Cold native process startup can include an antivirus scan on Windows CI.
        let deadline = if method == "initialize" { 15 } else { 5 };
        tokio::time::timeout(
            Duration::from_secs(deadline),
            self.output.read_line(&mut line),
        )
        .await
        .unwrap()
        .unwrap();
        let response: Value =
            serde_json::from_str(&line).expect("stdout must contain only JSON-RPC");
        assert_eq!(response["id"], id);
        response
    }

    async fn initialize(&mut self) {
        let response = self.request(1, "initialize", json!({"protocolVersion":"2025-03-26", "capabilities":{}, "clientInfo":{"name":"airgap-test","version":"1"}})).await;
        assert_eq!(response["result"]["serverInfo"]["name"], "airgap");
        let instructions = response["result"]["instructions"].as_str().unwrap();
        assert!(instructions.starts_with("MANDATORY AIRGAP AGENT ORCHESTRATION RULES:"));
        assert!(instructions.contains("'use airgap'"));
        self.send(json!({"jsonrpc":"2.0", "method":"notifications/initialized"}))
            .await;
    }

    async fn close(mut self) {
        drop(self.input);
        let status = tokio::time::timeout(Duration::from_secs(5), self.child.wait())
            .await
            .unwrap()
            .unwrap();
        assert!(status.success());
    }
}

#[tokio::test]
async fn test_cli_stdio_passthrough() {
    for args in [&[][..], &["--mcp"][..]] {
        let mut client = Client::with_args(args).await;
        client.initialize().await;
        let response = client.request(2, "tools/list", json!({})).await;
        assert_eq!(response["result"]["tools"].as_array().unwrap().len(), 7);
        client.close().await;
    }
}

#[tokio::test]
async fn initialize_discover_and_call_over_real_stdio() {
    let mut client = Client::new().await;
    client.initialize().await;
    let response = client.request(2, "tools/list", json!({})).await;
    let tools = response["result"]["tools"].as_array().unwrap();
    assert_eq!(tools.len(), 7);
    for name in [
        "airgap_sandbox_run",
        "airgap_verify_patch",
        "airgap_scan_injection",
        "airgap_redact_secrets",
        "airgap_audit_supply",
        "airgap_configure",
        "airgap_audit_repo",
    ] {
        let tool = tools.iter().find(|tool| tool["name"] == name).unwrap();
        assert!(tool["inputSchema"].is_object());
        if name != "airgap_configure" {
            assert!(
                tool["description"]
                    .as_str()
                    .unwrap()
                    .contains("TRIGGER AUTOMATICALLY"),
                "{name} lacks a trigger description"
            );
        }
    }
    let response = client.request(3, "tools/call", json!({"name":"airgap_redact_secrets", "arguments":{"raw_text":"API_KEY=example-secret"}})).await;
    assert_eq!(
        response["result"]["structuredContent"]["sanitized_text"],
        "API_KEY=[REDACTED]"
    );
    let summary = response["result"]["content"][0]["text"].as_str().unwrap();
    assert!(summary.starts_with("🛡️ **Airgap Secret Redactor** · ⏱️ **"));
    assert!(summary.ends_with("🔒 **1 Secret Redacted**\n\n> API_KEY=[REDACTED]"));
    assert!(!summary.contains("▄▀█") && !summary.contains('┌'));
    let response = client.request(6, "tools/call", json!({"name":"airgap_audit_supply", "arguments":{"manifest_type":"npm", "manifest_content":"{\"scripts\":{\"postinstall\":\"node x.js\"}}"}})).await;
    assert_eq!(
        response["result"]["structuredContent"]["risk_score"],
        "high"
    );
    let summary = response["result"]["content"][0]["text"].as_str().unwrap();
    assert!(summary.starts_with("🛡️ **Airgap Supply-Chain Audit** · ⏱️ **"));
    assert!(summary.contains(
        "🚨 **Risk Level: HIGH** (1 flagged item)\n\n- npm script postinstall: node x.js"
    ));
    let response = client
        .request(
            4,
            "tools/call",
            json!({"name":"airgap_sandbox_run", "arguments":{"command":"echo hello"}}),
        )
        .await;
    assert_eq!(response["result"]["isError"], true);
    let response = client.request(5, "tools/call", json!({"name":"airgap_configure", "arguments":{"model":"provider/model", "timeout_seconds":99}})).await;
    assert_eq!(
        response["result"]["structuredContent"]["active_model"],
        "provider/model"
    );
    client.close().await;
}

#[tokio::test]
async fn invalid_tools_and_inputs_return_protocol_errors_without_crashing() {
    let mut client = Client::new().await;
    client.initialize().await;
    for (id, params) in [
        (2, json!({"name":"missing","arguments":{}})),
        (
            3,
            json!({"name":"airgap_redact_secrets","arguments":{"raw_text":7}}),
        ),
        (
            4,
            json!({"name":"airgap_verify_patch","arguments":{"diff":"diff --git a/a b/a","test_command":"true"}}),
        ),
    ] {
        let response = client.request(id, "tools/call", params).await;
        assert!(response.get("error").is_some() || response["result"]["isError"] == true);
    }
    let response = client.request(5, "tools/list", json!({})).await;
    assert_eq!(response["result"]["tools"].as_array().unwrap().len(), 7);
    client.close().await;
}
