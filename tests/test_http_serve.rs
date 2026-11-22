use std::{
    collections::BTreeMap,
    net::SocketAddr,
    process::{Command, Stdio},
    time::Duration,
};

use airgap::{
    config::Config,
    serve::{serve_with_listener, ServeOptions},
    tools::Airgap,
};
use reqwest::{header, Client, StatusCode};
use serde_json::{json, Value};
use tokio::{net::TcpListener, sync::oneshot};
use wiremock::{
    matchers::{header as wiremock_header, method, path},
    Mock, MockServer, ResponseTemplate,
};

struct TestHttpServer {
    addr: SocketAddr,
    client: Client,
    shutdown: Option<oneshot::Sender<()>>,
    handle: tokio::task::JoinHandle<()>,
}

impl TestHttpServer {
    async fn start(config: Config, options: ServeOptions) -> Self {
        let server = Airgap::with_tenant_isolation(config, options.max_slots_per_tenant).unwrap();
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let (tx, rx) = oneshot::channel::<()>();
        let handle = tokio::spawn(async move {
            let _ = serve_with_listener(listener, server, options, async move {
                let _ = rx.await;
            })
            .await;
        });
        let client = Client::builder()
            .timeout(Duration::from_secs(10))
            .no_proxy()
            .build()
            .unwrap();
        Self {
            addr,
            client,
            shutdown: Some(tx),
            handle,
        }
    }

    fn url(&self, path: &str) -> String {
        format!("http://{}{path}", self.addr)
    }

    async fn initialize(
        &self,
        bearer: Option<&str>,
        extra_headers: &[(&str, &str)],
    ) -> (String, Value) {
        let body = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": "2025-03-26",
                "capabilities": {},
                "clientInfo": {"name": "airgap-http-test", "version": "1.0"}
            }
        });
        let mut req = self
            .client
            .post(self.url("/mcp"))
            .header(header::ACCEPT, "application/json, text/event-stream")
            .json(&body);
        if let Some(tok) = bearer {
            req = req.bearer_auth(tok);
        }
        for (k, v) in extra_headers {
            req = req.header(*k, *v);
        }
        let resp = req.send().await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let session_id = resp
            .headers()
            .get("mcp-session-id")
            .expect("initialize must return mcp-session-id")
            .to_str()
            .unwrap()
            .to_owned();
        let payload = parse_mcp_response(resp).await;

        let mut init_notif = self
            .client
            .post(self.url("/mcp"))
            .header(header::ACCEPT, "application/json, text/event-stream")
            .header("mcp-session-id", &session_id)
            .json(&json!({
                "jsonrpc": "2.0",
                "method": "notifications/initialized"
            }));
        if let Some(tok) = bearer {
            init_notif = init_notif.bearer_auth(tok);
        }
        for (k, v) in extra_headers {
            init_notif = init_notif.header(*k, *v);
        }
        let notif_resp = init_notif.send().await.unwrap();
        assert_eq!(notif_resp.status(), StatusCode::ACCEPTED);
        (session_id, payload)
    }

    async fn rpc(
        &self,
        session_id: &str,
        bearer: Option<&str>,
        extra_headers: &[(&str, &str)],
        id: u64,
        method: &str,
        params: Value,
    ) -> Value {
        let mut req = self
            .client
            .post(self.url("/mcp"))
            .header(header::ACCEPT, "application/json, text/event-stream")
            .header("mcp-session-id", session_id)
            .json(&json!({
                "jsonrpc": "2.0",
                "id": id,
                "method": method,
                "params": params
            }));
        if let Some(tok) = bearer {
            req = req.bearer_auth(tok);
        }
        for (k, v) in extra_headers {
            req = req.header(*k, *v);
        }
        let resp = req.send().await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        parse_mcp_response(resp).await
    }

    async fn stop(mut self) {
        if let Some(tx) = self.shutdown.take() {
            let _ = tx.send(());
        }
        let _ = tokio::time::timeout(Duration::from_secs(5), self.handle).await;
    }
}

async fn parse_mcp_response(resp: reqwest::Response) -> Value {
    let content_type = resp
        .headers()
        .get(header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_owned();
    let text = resp.text().await.unwrap();
    if content_type.contains("application/json") {
        return serde_json::from_str(&text).expect("valid JSON-RPC response");
    }
    // Extract last JSON-RPC `data:` frame from SSE stream.
    let mut last_json = None;
    for line in text.lines() {
        if let Some(data) = line.strip_prefix("data:") {
            let trimmed = data.trim();
            if trimmed.starts_with('{') {
                last_json = Some(serde_json::from_str::<Value>(trimmed).unwrap());
            }
        }
    }
    last_json.unwrap_or_else(|| panic!("expected JSON-RPC frame in SSE body: {text:?}"))
}

fn test_config() -> Config {
    Config::from_values(BTreeMap::new()).unwrap()
}

fn maas_clean_body() -> Value {
    json!({
        "choices": [{
            "finish_reason": "stop",
            "message": {
                "content": "{\"is_safe\":true,\"threat_level\":\"clean\",\"findings\":[]}"
            }
        }]
    })
}

#[test]
fn serve_cli_option_parsing_and_fail_closed_security() {
    // Non-loopback without auth key must fail closed.
    let env = BTreeMap::new();
    let err = ServeOptions::parse_args(
        &[
            "--host".into(),
            "0.0.0.0".into(),
            "--port".into(),
            "8080".into(),
        ],
        &env,
    )
    .err()
    .expect("non-loopback without auth must fail");
    assert!(err.to_string().contains("refusing to bind"));

    // Explicit --allow-unauthenticated opts in.
    let opts = ServeOptions::parse_args(
        &[
            "--host".into(),
            "0.0.0.0".into(),
            "--allow-unauthenticated".into(),
        ],
        &env,
    )
    .unwrap()
    .unwrap();
    assert!(opts.allow_unauthenticated);

    // `--auth-key ENV_VAR` resolves from env when that variable is set.
    let mut env = BTreeMap::new();
    env.insert("MY_SECRET_VAR".into(), "super-secret-token".into());
    env.insert("FLY_APP_NAME".into(), "airgap-mcp".into());
    env.insert(
        "RAILWAY_PUBLIC_DOMAIN".into(),
        "airgap.up.railway.app".into(),
    );
    let opts = ServeOptions::parse_args(
        &[
            "--host".into(),
            "0.0.0.0".into(),
            "--auth-key".into(),
            "MY_SECRET_VAR".into(),
        ],
        &env,
    )
    .unwrap()
    .unwrap();
    assert_eq!(opts.auth_keys, vec!["super-secret-token"]);
    let hosts = opts.allowed_hosts.unwrap();
    assert!(hosts.iter().any(|h| h == "airgap-mcp.fly.dev"));
    assert!(hosts.iter().any(|h| h == "airgap.up.railway.app"));

    // Multi-key env support via AIRGAP_AUTH_KEYS.
    let mut env = BTreeMap::new();
    env.insert("AIRGAP_AUTH_KEYS".into(), "tok-a, tok-b , tok-c".into());
    let opts = ServeOptions::parse_args(&["--host".into(), "0.0.0.0".into()], &env)
        .unwrap()
        .unwrap();
    assert_eq!(opts.auth_keys, vec!["tok-a", "tok-b", "tok-c"]);

    // `--allowed-hosts *` disables Host validation.
    let opts = ServeOptions::parse_args(&["--allowed-hosts".into(), "*".into()], &env)
        .unwrap()
        .unwrap();
    assert!(opts.allowed_hosts.is_none());

    // `--auth-key-env` fails if env var is unset.
    assert!(ServeOptions::parse_args(
        &["--auth-key-env".into(), "UNSET_VAR".into()],
        &BTreeMap::new()
    )
    .is_err());

    // Invalid slot count fails.
    assert!(ServeOptions::parse_args(
        &["--max-slots-per-tenant".into(), "0".into()],
        &BTreeMap::new()
    )
    .is_err());

    // `airgap serve --help` prints help and exits 0.
    let output = Command::new(env!("CARGO_BIN_EXE_airgap"))
        .args(["serve", "--help"])
        .stdin(Stdio::null())
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("airgap serve"));
    assert!(stdout.contains("--require-byok"));
}

#[tokio::test]
async fn streamable_http_health_auth_and_mcp_roundtrip() {
    let options = ServeOptions {
        auth_keys: vec!["secret-token-1".into()],
        ..ServeOptions::default()
    };
    let srv = TestHttpServer::start(test_config(), options).await;

    // Health endpoint is public (no auth header required).
    let health = srv.client.get(srv.url("/healthz")).send().await.unwrap();
    assert_eq!(health.status(), StatusCode::OK);
    let health_json: Value = health.json().await.unwrap();
    assert_eq!(health_json["status"], "ok");
    assert_eq!(health_json["service"], "airgap");

    // Missing or wrong bearer token -> 401 Unauthorized.
    let init_body = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "protocolVersion": "2025-03-26",
            "capabilities": {},
            "clientInfo": {"name": "test", "version": "1"}
        }
    });
    let unauth = srv
        .client
        .post(srv.url("/mcp"))
        .header(header::ACCEPT, "application/json, text/event-stream")
        .json(&init_body)
        .send()
        .await
        .unwrap();
    assert_eq!(unauth.status(), StatusCode::UNAUTHORIZED);
    assert!(unauth
        .headers()
        .get(header::WWW_AUTHENTICATE)
        .unwrap()
        .to_str()
        .unwrap()
        .contains("Bearer"));

    let bad_auth = srv
        .client
        .post(srv.url("/mcp"))
        .bearer_auth("wrong-token")
        .header(header::ACCEPT, "application/json, text/event-stream")
        .json(&init_body)
        .send()
        .await
        .unwrap();
    assert_eq!(bad_auth.status(), StatusCode::UNAUTHORIZED);

    // Valid token -> full initialize, tools/list, and tools/call over Streamable HTTP.
    let (session_id, init_resp) = srv.initialize(Some("secret-token-1"), &[]).await;
    assert_eq!(init_resp["result"]["serverInfo"]["name"], "airgap");

    let list_resp = srv
        .rpc(
            &session_id,
            Some("secret-token-1"),
            &[],
            2,
            "tools/list",
            json!({}),
        )
        .await;
    assert_eq!(list_resp["result"]["tools"].as_array().unwrap().len(), 8);

    let call_resp = srv
        .rpc(
            &session_id,
            Some("secret-token-1"),
            &[],
            3,
            "tools/call",
            json!({
                "name": "airgap_redact_secrets",
                "arguments": {"raw_text": "API_KEY=example-secret"}
            }),
        )
        .await;
    assert_eq!(
        call_resp["result"]["structuredContent"]["sanitized_text"],
        "API_KEY=[REDACTED]"
    );

    // Clean session termination via DELETE /mcp.
    let del = srv
        .client
        .delete(srv.url("/mcp"))
        .bearer_auth("secret-token-1")
        .header("mcp-session-id", &session_id)
        .send()
        .await
        .unwrap();
    assert!(del.status().is_success());

    srv.stop().await;
}

#[tokio::test]
async fn cors_and_origin_protection_enforced() {
    // 1. Default (empty allowlist): reject any Origin header, allow non-browser requests.
    let default_srv = TestHttpServer::start(
        test_config(),
        ServeOptions {
            auth_keys: vec!["tok".into()],
            ..ServeOptions::default()
        },
    )
    .await;

    let blocked = default_srv
        .client
        .post(default_srv.url("/mcp"))
        .bearer_auth("tok")
        .header(header::ORIGIN, "https://evil.example")
        .header(header::ACCEPT, "application/json, text/event-stream")
        .json(&json!({"jsonrpc":"2.0","id":1,"method":"tools/list","params":{}}))
        .send()
        .await
        .unwrap();
    assert_eq!(blocked.status(), StatusCode::FORBIDDEN);
    default_srv.stop().await;

    // 2. Explicit allowlist: allow matching origins and preflight, block others.
    let cors_srv = TestHttpServer::start(
        test_config(),
        ServeOptions {
            auth_keys: vec!["tok".into()],
            allowed_origins: vec![
                "https://app.example.com".into(),
                "http://localhost:*".into(),
            ],
            ..ServeOptions::default()
        },
    )
    .await;

    let preflight_ok = cors_srv
        .client
        .request(reqwest::Method::OPTIONS, cors_srv.url("/mcp"))
        .header(header::ORIGIN, "https://app.example.com")
        .send()
        .await
        .unwrap();
    assert_eq!(preflight_ok.status(), StatusCode::NO_CONTENT);
    assert_eq!(
        preflight_ok
            .headers()
            .get(header::ACCESS_CONTROL_ALLOW_ORIGIN)
            .unwrap(),
        "https://app.example.com"
    );
    assert!(preflight_ok
        .headers()
        .get(header::ACCESS_CONTROL_ALLOW_HEADERS)
        .unwrap()
        .to_str()
        .unwrap()
        .contains("X-GMI-AgentBox-Key"));

    let preflight_denied = cors_srv
        .client
        .request(reqwest::Method::OPTIONS, cors_srv.url("/mcp"))
        .header(header::ORIGIN, "https://evil.example")
        .send()
        .await
        .unwrap();
    assert_eq!(preflight_denied.status(), StatusCode::FORBIDDEN);

    let (_sid, _init) = cors_srv
        .initialize(Some("tok"), &[("origin", "http://localhost:5173")])
        .await;

    cors_srv.stop().await;
}

#[tokio::test]
async fn multi_tenant_runtime_config_byok_and_slot_isolation() {
    let mock_gmi = MockServer::start().await;

    // Tenant A's BYOK key hits a slow endpoint so Tenant A holds its single concurrency slot.
    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .and(wiremock_header("authorization", "Bearer byok-key-tenant-a"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_delay(Duration::from_millis(600))
                .set_body_json(maas_clean_body()),
        )
        .mount(&mock_gmi)
        .await;

    // Tenant B's BYOK key responds immediately.
    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .and(wiremock_header("authorization", "Bearer byok-key-tenant-b"))
        .respond_with(ResponseTemplate::new(200).set_body_json(maas_clean_body()))
        .mount(&mock_gmi)
        .await;

    let mut cfg = test_config();
    cfg.maas_base_url = mock_gmi.uri();
    cfg.agentbox_base_url = mock_gmi.uri();
    // Server has managed keys, but require_byok=true must prevent falling back to them.
    cfg.maas_key = Some("server-managed-key-must-not-be-used".into());
    cfg.agentbox_key = Some("server-managed-compute-key".into());

    let options = ServeOptions {
        auth_keys: vec!["tenant-a-bearer".into(), "tenant-b-bearer".into()],
        require_byok: true,
        allow_byok: true,
        max_slots_per_tenant: 1,
        ..ServeOptions::default()
    };
    let srv = TestHttpServer::start(cfg, options).await;

    let (sid_a, _) = srv.initialize(Some("tenant-a-bearer"), &[]).await;
    let (sid_b, _) = srv.initialize(Some("tenant-b-bearer"), &[]).await;

    // 1. Runtime config isolation: Tenant A changes model/timeout; Tenant B keeps defaults.
    let cfg_a = srv
        .rpc(
            &sid_a,
            Some("tenant-a-bearer"),
            &[],
            2,
            "tools/call",
            json!({
                "name": "airgap_configure",
                "arguments": {"model": "custom/tenant-a-model", "timeout_seconds": 42}
            }),
        )
        .await;
    assert_eq!(
        cfg_a["result"]["structuredContent"]["active_model"],
        "custom/tenant-a-model"
    );
    assert_eq!(cfg_a["result"]["structuredContent"]["timeout_seconds"], 42);

    let cfg_b = srv
        .rpc(
            &sid_b,
            Some("tenant-b-bearer"),
            &[],
            2,
            "tools/call",
            json!({"name": "airgap_configure", "arguments": {}}),
        )
        .await;
    assert_ne!(
        cfg_b["result"]["structuredContent"]["active_model"],
        "custom/tenant-a-model"
    );
    assert_eq!(cfg_b["result"]["structuredContent"]["timeout_seconds"], 120);

    // 2. Strict BYOK enforcement: calling without X-GMI-MaaS-Key / X-GMI-API-Key fails even though server has keys.
    let missing_byok = srv
        .rpc(
            &sid_b,
            Some("tenant-b-bearer"),
            &[],
            3,
            "tools/call",
            json!({
                "name": "airgap_scan_injection",
                "arguments": {"source_label": "issue", "text_content": "hello world"}
            }),
        )
        .await;
    assert_eq!(missing_byok["result"]["isError"], true);
    assert!(missing_byok["result"]["content"][0]["text"]
        .as_str()
        .unwrap()
        .contains("GMI_MAAS_API_KEY"));

    // 3. Per-tenant slot isolation: Tenant A occupies its 1 slot with a 600ms request.
    //    - A second concurrent request for Tenant A must fail immediately (slot saturated).
    //    - A concurrent request for Tenant B must succeed immediately (independent semaphore).
    let a_slow = srv.rpc(
        &sid_a,
        Some("tenant-a-bearer"),
        &[("x-gmi-maas-key", "byok-key-tenant-a")],
        10,
        "tools/call",
        json!({
            "name": "airgap_scan_injection",
            "arguments": {"source_label": "issue", "text_content": "slow tenant a"}
        }),
    );

    let concurrent_check = async {
        tokio::time::sleep(Duration::from_millis(120)).await;
        // Second concurrent call for Tenant A hits Tenant A's 1-slot limit.
        let a_second = srv
            .rpc(
                &sid_a,
                Some("tenant-a-bearer"),
                &[("x-gmi-maas-key", "byok-key-tenant-a")],
                11,
                "tools/call",
                json!({
                    "name": "airgap_scan_injection",
                    "arguments": {"source_label": "issue", "text_content": "overflow tenant a"}
                }),
            )
            .await;
        assert_eq!(a_second["result"]["isError"], true);
        assert!(a_second["result"]["content"][0]["text"]
            .as_str()
            .unwrap()
            .contains("already running"));

        // Meanwhile Tenant B succeeds immediately with its own BYOK key and its own semaphore!
        let b_call = srv
            .rpc(
                &sid_b,
                Some("tenant-b-bearer"),
                &[("x-gmi-maas-key", "byok-key-tenant-b")],
                12,
                "tools/call",
                json!({
                    "name": "airgap_scan_injection",
                    "arguments": {"source_label": "issue", "text_content": "fast tenant b"}
                }),
            )
            .await;
        assert_ne!(b_call["result"]["isError"], true);
        assert_eq!(
            b_call["result"]["structuredContent"]["threat_level"],
            "clean"
        );
    };

    let (a_first, ()) = tokio::join!(a_slow, concurrent_check);
    assert_ne!(a_first["result"]["isError"], true);
    assert_eq!(
        a_first["result"]["structuredContent"]["threat_level"],
        "clean"
    );

    srv.stop().await;
}
