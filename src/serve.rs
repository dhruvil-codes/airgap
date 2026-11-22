use std::{
    collections::BTreeMap,
    future::Future,
    net::{IpAddr, SocketAddr},
    sync::Arc,
    time::Duration,
};

use anyhow::{bail, ensure, Context, Result};
use axum::{
    extract::{Request, State},
    http::{
        header::{self, HeaderName, HeaderValue},
        Method, StatusCode,
    },
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::get,
    Json, Router,
};
use rmcp::transport::streamable_http_server::{
    session::local::LocalSessionManager, StreamableHttpServerConfig, StreamableHttpService,
};
use serde_json::json;
use tokio::net::TcpListener;
use tokio_util::sync::CancellationToken;

use crate::{
    config::Config,
    tenant::{
        any_secret_matches, hash_identifier, TenantContext, DEFAULT_SLOTS_PER_TENANT,
        MAX_SLOTS_PER_TENANT,
    },
    tools::Airgap,
};

pub const HEADER_GMI_AGENTBOX_KEY: &str = "x-gmi-agentbox-key";
pub const HEADER_GMI_MAAS_KEY: &str = "x-gmi-maas-key";
pub const HEADER_GMI_API_KEY: &str = "x-gmi-api-key";
pub const HEADER_GITHUB_TOKEN: &str = "x-github-token";

/// 16 MiB body cap: covers Airgap's 8 MiB file snapshot ceiling plus JSON framing overhead.
const MAX_HTTP_BODY_BYTES: usize = 16 * 1024 * 1024;
const MAX_HEADER_CREDENTIAL_LEN: usize = 4096;

const CORS_ALLOW_METHODS: &str = "GET, POST, DELETE, OPTIONS";
const CORS_ALLOW_HEADERS: &str = "Authorization, Content-Type, Accept, Mcp-Session-Id, Mcp-Protocol-Version, Last-Event-ID, X-GMI-AgentBox-Key, X-GMI-MaaS-Key, X-GMI-API-Key, X-GitHub-Token";
const CORS_EXPOSE_HEADERS: &str = "Mcp-Session-Id, Mcp-Protocol-Version";

// Do not derive Debug: holds server bearer auth keys.
#[derive(Clone)]
pub struct ServeOptions {
    pub host: String,
    pub port: u16,
    pub auth_keys: Vec<String>,
    pub allow_unauthenticated: bool,
    /// `None` disables Host validation (`--allowed-hosts *`); `Some(list)` enforces `list`.
    pub allowed_hosts: Option<Vec<String>>,
    pub allowed_origins: Vec<String>,
    pub allow_byok: bool,
    pub require_byok: bool,
    pub max_slots_per_tenant: usize,
    pub stateless: bool,
    pub json_response: bool,
}

impl Default for ServeOptions {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".into(),
            port: 8080,
            auth_keys: Vec::new(),
            allow_unauthenticated: false,
            allowed_hosts: Some(vec!["localhost".into(), "127.0.0.1".into(), "::1".into()]),
            allowed_origins: Vec::new(),
            allow_byok: true,
            require_byok: false,
            max_slots_per_tenant: DEFAULT_SLOTS_PER_TENANT,
            stateless: false,
            json_response: false,
        }
    }
}

impl ServeOptions {
    pub fn parse_args(args: &[String], env: &BTreeMap<String, String>) -> Result<Option<Self>> {
        let mut host = env_nonempty(env, "AIRGAP_HOST").unwrap_or_else(|| "127.0.0.1".into());
        let mut port: u16 =
            match env_nonempty(env, "AIRGAP_PORT").or_else(|| env_nonempty(env, "PORT")) {
                Some(raw) => raw.parse().context("invalid PORT / AIRGAP_PORT value")?,
                None => 8080,
            };
        let mut auth_keys = Vec::new();
        let mut cli_auth_override = false;
        let mut allow_unauthenticated = env_bool(env, "AIRGAP_ALLOW_UNAUTHENTICATED", false)?;
        let mut explicit_allowed_hosts: Option<String> = env_nonempty(env, "AIRGAP_ALLOWED_HOSTS");
        let mut allowed_origins = parse_csv(env_nonempty(env, "AIRGAP_ALLOWED_ORIGINS"));
        let mut allow_byok = env_bool(env, "AIRGAP_ALLOW_BYOK", true)?;
        let mut require_byok = env_bool(env, "AIRGAP_REQUIRE_BYOK", false)?;
        let mut max_slots_per_tenant: usize = match env_nonempty(env, "AIRGAP_MAX_SLOTS_PER_TENANT")
        {
            Some(raw) => raw
                .parse()
                .context("invalid AIRGAP_MAX_SLOTS_PER_TENANT value")?,
            None => DEFAULT_SLOTS_PER_TENANT,
        };
        let mut stateless = env_bool(env, "AIRGAP_STATELESS", false)?;
        let mut json_response = env_bool(env, "AIRGAP_JSON_RESPONSE", false)?;

        let mut i = 0;
        while i < args.len() {
            let arg = args[i].as_str();
            let (flag, inline_value) = match arg.split_once('=') {
                Some((k, v)) if k.starts_with("--") => (k, Some(v)),
                _ => (arg, None),
            };
            let next_value = |idx: &mut usize, name: &str| -> Result<String> {
                if let Some(v) = inline_value {
                    return Ok(v.to_owned());
                }
                *idx += 1;
                args.get(*idx)
                    .cloned()
                    .with_context(|| format!("missing value for {name}"))
            };

            match flag {
                "-h" | "--help" | "help" => {
                    return Ok(None);
                }
                "--host" => {
                    host = next_value(&mut i, "--host")?;
                }
                "--port" => {
                    let raw = next_value(&mut i, "--port")?;
                    port = raw.parse().context("invalid --port value")?;
                }
                "--auth-key" => {
                    let raw = next_value(&mut i, "--auth-key")?;
                    ensure!(!raw.trim().is_empty(), "--auth-key cannot be empty");
                    cli_auth_override = true;
                    // Support `--auth-key ENV_VAR_NAME` (preferred) as well as literal key values.
                    if let Some(from_env) = env_nonempty(env, raw.trim()) {
                        auth_keys.push(from_env);
                    } else {
                        auth_keys.push(raw);
                    }
                }
                "--auth-key-env" => {
                    let var_name = next_value(&mut i, "--auth-key-env")?;
                    let value = env_nonempty(env, var_name.trim()).with_context(|| {
                        format!("environment variable '{var_name}' is not set or empty")
                    })?;
                    cli_auth_override = true;
                    auth_keys.push(value);
                }
                "--allow-unauthenticated" => {
                    ensure!(
                        inline_value.is_none(),
                        "--allow-unauthenticated takes no value"
                    );
                    allow_unauthenticated = true;
                }
                "--allowed-hosts" => {
                    explicit_allowed_hosts = Some(next_value(&mut i, "--allowed-hosts")?);
                }
                "--allowed-origins" => {
                    allowed_origins = parse_csv(Some(next_value(&mut i, "--allowed-origins")?));
                }
                "--allow-byok" => {
                    ensure!(inline_value.is_none(), "--allow-byok takes no value");
                    allow_byok = true;
                }
                "--no-byok" => {
                    ensure!(inline_value.is_none(), "--no-byok takes no value");
                    allow_byok = false;
                    require_byok = false;
                }
                "--require-byok" => {
                    ensure!(inline_value.is_none(), "--require-byok takes no value");
                    require_byok = true;
                    allow_byok = true;
                }
                "--max-slots-per-tenant" => {
                    let raw = next_value(&mut i, "--max-slots-per-tenant")?;
                    max_slots_per_tenant = raw
                        .parse()
                        .context("invalid --max-slots-per-tenant value")?;
                }
                "--stateless" => {
                    ensure!(inline_value.is_none(), "--stateless takes no value");
                    stateless = true;
                }
                "--json-response" => {
                    ensure!(inline_value.is_none(), "--json-response takes no value");
                    json_response = true;
                }
                other => bail!("unknown serve option '{other}'; run airgap serve --help"),
            }
            i += 1;
        }

        if !cli_auth_override {
            if let Some(single) = env_nonempty(env, "AIRGAP_AUTH_KEY") {
                auth_keys.push(single);
            }
            for key in parse_csv(env_nonempty(env, "AIRGAP_AUTH_KEYS")) {
                if !auth_keys.contains(&key) {
                    auth_keys.push(key);
                }
            }
        }

        ensure!(
            (1..=MAX_SLOTS_PER_TENANT).contains(&max_slots_per_tenant),
            "max-slots-per-tenant must be between 1 and {MAX_SLOTS_PER_TENANT}"
        );
        ensure!(!host.trim().is_empty(), "host cannot be empty");

        if require_byok {
            allow_byok = true;
        }

        for origin in &allowed_origins {
            validate_origin_entry(origin)?;
        }

        let allowed_hosts = resolve_allowed_hosts(explicit_allowed_hosts, &host, env);

        if auth_keys.is_empty() && !allow_unauthenticated && !is_loopback_host(&host) {
            bail!(
                "refusing to bind '{host}:{port}' without bearer authentication; \
                 set AIRGAP_AUTH_KEY (or pass --auth-key <ENV_VAR>), or pass --allow-unauthenticated explicitly"
            );
        }

        Ok(Some(Self {
            host,
            port,
            auth_keys,
            allow_unauthenticated,
            allowed_hosts,
            allowed_origins,
            allow_byok,
            require_byok,
            max_slots_per_tenant,
            stateless,
            json_response,
        }))
    }
}

pub fn print_help() {
    let banner = crate::banner("serve · Streamable HTTP MCP server");
    println!(
        "{banner}Usage: airgap serve [options]\n\n\
         Starts a Streamable HTTP / SSE MCP server at /mcp and a health endpoint at /healthz.\n\n\
         Options:\n\
           --host <ADDR>                 Bind address (default: AIRGAP_HOST or 127.0.0.1)\n\
           --port <PORT>                 Listen port (default: AIRGAP_PORT, PORT, or 8080)\n\
           --auth-key <ENV_VAR_OR_KEY>   Bearer auth token, or env var name holding it\n\
           --auth-key-env <ENV_VAR>      Env var holding the bearer auth token\n\
           --allow-unauthenticated       Allow non-loopback bind without bearer auth\n\
           --allowed-hosts <CSV>         Allowed HTTP Host headers ('*' disables Host check)\n\
           --allowed-origins <CSV>       Allowed browser Origin headers for CORS protection\n\
           --allow-byok                  Allow per-request X-GMI-* / X-GitHub-Token headers (default)\n\
           --no-byok                     Disable BYOK headers; use server credentials only\n\
           --require-byok                Require callers to supply GMI keys via headers\n\
           --max-slots-per-tenant <N>    Concurrent sandbox/inference slots per tenant (1..=64, default: 4)\n\
           --stateless                   Run in stateless HTTP mode (no MCP session state)\n\
           --json-response               Prefer JSON responses in stateless mode\n\
           -h, --help                    Show this help message\n\n\
         Environment variables:\n\
           AIRGAP_AUTH_KEY               Single bearer token\n\
           AIRGAP_AUTH_KEYS              Comma-separated per-tenant bearer tokens\n\
           AIRGAP_ALLOWED_HOSTS          Comma-separated allowed Host headers (auto-includes Fly/Railway domains)\n\
           AIRGAP_ALLOWED_ORIGINS        Comma-separated allowed browser origins\n\
           AIRGAP_REQUIRE_BYOK=1         Enforce caller-supplied X-GMI-AgentBox-Key / X-GMI-MaaS-Key"
    );
}

#[derive(Clone)]
struct MiddlewareState {
    auth_keys: Arc<Vec<String>>,
    allowed_origins: Arc<Vec<String>>,
    allow_byok: bool,
    require_byok: bool,
}

pub fn build_router(server: Airgap, options: &ServeOptions, cancel: CancellationToken) -> Router {
    let mut http_config = StreamableHttpServerConfig::default()
        .with_sse_keep_alive(Some(Duration::from_secs(15)))
        .with_sse_retry(Some(Duration::from_secs(3)))
        .with_legacy_session_mode(!options.stateless)
        .with_json_response(options.json_response)
        .with_max_request_body_bytes(MAX_HTTP_BODY_BYTES)
        .with_cancellation_token(cancel);

    match &options.allowed_hosts {
        Some(hosts) => {
            http_config = http_config.with_allowed_hosts(hosts.clone());
        }
        None => {
            http_config = http_config.disable_allowed_hosts();
        }
    }

    if options.allowed_origins.is_empty() {
        http_config = http_config.enforce_origin_validation();
    } else {
        http_config = http_config.with_allowed_origins(options.allowed_origins.clone());
    }

    let server_for_service = server.clone();
    let mcp_service: StreamableHttpService<Airgap, LocalSessionManager> =
        StreamableHttpService::new(
            move || Ok(server_for_service.clone()),
            Arc::new(LocalSessionManager::default()),
            http_config,
        );

    let middleware_state = MiddlewareState {
        auth_keys: Arc::new(options.auth_keys.clone()),
        allowed_origins: Arc::new(options.allowed_origins.clone()),
        allow_byok: options.allow_byok,
        require_byok: options.require_byok,
    };

    let mcp_router =
        Router::new()
            .nest_service("/mcp", mcp_service)
            .layer(middleware::from_fn_with_state(
                middleware_state,
                auth_and_tenant_middleware,
            ));

    Router::new()
        .route("/healthz", get(healthz))
        .merge(mcp_router)
}

async fn healthz() -> impl IntoResponse {
    Json(json!({
        "status": "ok",
        "service": "airgap",
        "version": env!("CARGO_PKG_VERSION"),
    }))
}

async fn auth_and_tenant_middleware(
    State(state): State<MiddlewareState>,
    mut request: Request,
    next: Next,
) -> Response {
    let origin = match request.headers().get(header::ORIGIN) {
        Some(raw) => match raw.to_str() {
            Ok(value) => Some(value.to_owned()),
            Err(_) => {
                return json_error(StatusCode::FORBIDDEN, "invalid origin header");
            }
        },
        None => None,
    };

    let cors_origin = if let Some(ref origin_value) = origin {
        if !origin_matches_allowlist(origin_value, &state.allowed_origins) {
            return json_error(StatusCode::FORBIDDEN, "origin not allowed");
        }
        Some(origin_value.clone())
    } else {
        None
    };

    if request.method() == Method::OPTIONS {
        let mut response = StatusCode::NO_CONTENT.into_response();
        let headers = response.headers_mut();
        headers.insert(header::ALLOW, HeaderValue::from_static(CORS_ALLOW_METHODS));
        if let Some(allowed) = cors_origin {
            apply_cors_headers(headers, &allowed, true);
        }
        return response;
    }

    let bearer_token = if !state.auth_keys.is_empty() {
        let token = request
            .headers()
            .get(header::AUTHORIZATION)
            .and_then(|v| v.to_str().ok())
            .and_then(parse_bearer_token);
        match token {
            Some(tok) if any_secret_matches(tok, &state.auth_keys) => Some(tok.to_owned()),
            _ => {
                let mut response = json_error(StatusCode::UNAUTHORIZED, "unauthorized");
                response.headers_mut().insert(
                    header::WWW_AUTHENTICATE,
                    HeaderValue::from_static("Bearer realm=\"airgap\""),
                );
                if let Some(allowed) = cors_origin {
                    apply_cors_headers(response.headers_mut(), &allowed, false);
                }
                return response;
            }
        }
    } else {
        None
    };

    let byok = if state.allow_byok || state.require_byok {
        match extract_byok_headers(request.headers()) {
            Ok(creds) => creds,
            Err(message) => {
                let mut response = json_error(StatusCode::BAD_REQUEST, message);
                if let Some(allowed) = cors_origin {
                    apply_cors_headers(response.headers_mut(), &allowed, false);
                }
                return response;
            }
        }
    } else {
        ByokCredentials::default()
    };

    let session_hint = request
        .headers()
        .get("mcp-session-id")
        .and_then(|v| v.to_str().ok())
        .map(str::trim)
        .filter(|s| !s.is_empty());
    let tenant_id = derive_tenant_id(
        bearer_token.as_deref(),
        state.auth_keys.len() > 1,
        byok.agentbox_key.as_deref(),
        byok.maas_key.as_deref(),
        session_hint,
    );

    request.extensions_mut().insert(TenantContext {
        tenant_id,
        agentbox_key: byok.agentbox_key,
        maas_key: byok.maas_key,
        github_token: byok.github_token,
        require_byok: state.require_byok,
    });

    let mut response = next.run(request).await;
    if let Some(allowed) = cors_origin {
        apply_cors_headers(response.headers_mut(), &allowed, false);
    }
    response
}

fn derive_tenant_id(
    bearer_token: Option<&str>,
    multi_key_auth: bool,
    agentbox_key: Option<&str>,
    maas_key: Option<&str>,
    session_hint: Option<&str>,
) -> String {
    if multi_key_auth {
        if let Some(bearer) = bearer_token {
            return hash_identifier("bearer", bearer);
        }
    }
    if agentbox_key.is_some() || maas_key.is_some() {
        let compute = agentbox_key.map(|k| hash_identifier("byok-compute", k));
        let infer = maas_key.map(|k| hash_identifier("byok-infer", k));
        let bearer = bearer_token.map(|b| hash_identifier("bearer", b));
        return hash_identifier(
            "tenant-byok",
            &format!(
                "{}:{}:{}",
                bearer.as_deref().unwrap_or(""),
                compute.as_deref().unwrap_or(""),
                infer.as_deref().unwrap_or("")
            ),
        );
    }
    match bearer_token {
        Some(bearer) => hash_identifier("bearer", bearer),
        None => hash_identifier("anon", session_hint.unwrap_or("default")),
    }
}

fn parse_bearer_token(header_value: &str) -> Option<&str> {
    let (scheme, token) = header_value.trim().split_once(char::is_whitespace)?;
    if !scheme.eq_ignore_ascii_case("bearer") {
        return None;
    }
    let token = token.trim();
    if token.is_empty() || token.len() > MAX_HEADER_CREDENTIAL_LEN {
        return None;
    }
    Some(token)
}

#[derive(Default)]
struct ByokCredentials {
    agentbox_key: Option<String>,
    maas_key: Option<String>,
    github_token: Option<String>,
}

fn extract_byok_headers(
    headers: &axum::http::HeaderMap,
) -> std::result::Result<ByokCredentials, &'static str> {
    let fallback_gmi = read_credential_header(headers, HEADER_GMI_API_KEY)?;
    let agentbox_key =
        read_credential_header(headers, HEADER_GMI_AGENTBOX_KEY)?.or_else(|| fallback_gmi.clone());
    let maas_key = read_credential_header(headers, HEADER_GMI_MAAS_KEY)?.or(fallback_gmi);
    let github_token = read_credential_header(headers, HEADER_GITHUB_TOKEN)?;
    Ok(ByokCredentials {
        agentbox_key,
        maas_key,
        github_token,
    })
}

fn read_credential_header(
    headers: &axum::http::HeaderMap,
    name: &'static str,
) -> std::result::Result<Option<String>, &'static str> {
    let Some(raw) = headers.get(HeaderName::from_static(name)) else {
        return Ok(None);
    };
    let text = raw.to_str().map_err(|_| "invalid credential header")?;
    let trimmed = text.trim();
    if trimmed.is_empty()
        || trimmed.len() > MAX_HEADER_CREDENTIAL_LEN
        || trimmed.chars().any(char::is_control)
    {
        return Err("invalid credential header");
    }
    Ok(Some(trimmed.to_owned()))
}

fn apply_cors_headers(headers: &mut axum::http::HeaderMap, origin: &str, preflight: bool) {
    if let Ok(value) = HeaderValue::from_str(origin) {
        headers.insert(header::ACCESS_CONTROL_ALLOW_ORIGIN, value);
    }
    headers.insert(
        header::ACCESS_CONTROL_EXPOSE_HEADERS,
        HeaderValue::from_static(CORS_EXPOSE_HEADERS),
    );
    headers.append(header::VARY, HeaderValue::from_static("Origin"));
    if preflight {
        headers.insert(
            header::ACCESS_CONTROL_ALLOW_METHODS,
            HeaderValue::from_static(CORS_ALLOW_METHODS),
        );
        headers.insert(
            header::ACCESS_CONTROL_ALLOW_HEADERS,
            HeaderValue::from_static(CORS_ALLOW_HEADERS),
        );
        headers.insert(
            header::ACCESS_CONTROL_MAX_AGE,
            HeaderValue::from_static("600"),
        );
    }
}

fn json_error(status: StatusCode, message: &'static str) -> Response {
    (status, Json(json!({ "error": message }))).into_response()
}

pub async fn serve_with_listener(
    listener: TcpListener,
    server: Airgap,
    options: ServeOptions,
    shutdown: impl Future<Output = ()> + Send + 'static,
) -> Result<()> {
    let cancel = CancellationToken::new();
    let router = build_router(server.clone(), &options, cancel.clone());
    let shutdown_cancel = cancel.clone();

    let serve_result = axum::serve(listener, router)
        .with_graceful_shutdown(async move {
            shutdown.await;
            shutdown_cancel.cancel();
        })
        .await;

    cancel.cancel();
    server.shutdown().await;
    serve_result.context("HTTP server exited with error")?;
    Ok(())
}

pub async fn run(
    args: &[String],
    shutdown: impl Future<Output = ()> + Send + 'static,
) -> Result<()> {
    let env: BTreeMap<String, String> = std::env::vars().collect();
    let Some(options) = ServeOptions::parse_args(args, &env)? else {
        print_help();
        return Ok(());
    };
    let config = Config::discover()?;
    let server = Airgap::with_tenant_isolation(config, options.max_slots_per_tenant)?;
    let bind_addr = format!("{}:{}", options.host, options.port);
    let listener = TcpListener::bind(&bind_addr)
        .await
        .with_context(|| format!("failed to bind {bind_addr}"))?;
    let local_addr: SocketAddr = listener.local_addr()?;

    let auth_summary = if options.auth_keys.is_empty() {
        "unauthenticated".to_owned()
    } else {
        format!("bearer ({} key(s))", options.auth_keys.len())
    };
    let cred_summary = if options.require_byok {
        "require-byok"
    } else if options.allow_byok {
        "managed+byok"
    } else {
        "managed-only"
    };
    let mode_summary = if options.stateless {
        "stateless"
    } else {
        "stateful"
    };
    eprintln!(
        "airgap v{} listening on http://{local_addr}/mcp (health: http://{local_addr}/healthz, auth: {auth_summary}, credentials: {cred_summary}, slots/tenant: {}, mode: {mode_summary})",
        env!("CARGO_PKG_VERSION"),
        options.max_slots_per_tenant
    );

    serve_with_listener(listener, server, options, shutdown).await
}

fn env_nonempty(env: &BTreeMap<String, String>, key: &str) -> Option<String> {
    env.get(key)
        .map(|v| v.trim())
        .filter(|v| !v.is_empty())
        .map(ToOwned::to_owned)
}

fn env_bool(env: &BTreeMap<String, String>, key: &str, default: bool) -> Result<bool> {
    let Some(raw) = env_nonempty(env, key) else {
        return Ok(default);
    };
    match raw.to_ascii_lowercase().as_str() {
        "1" | "true" | "yes" | "on" => Ok(true),
        "0" | "false" | "no" | "off" => Ok(false),
        _ => bail!("invalid boolean value for {key}: expected true/false or 1/0"),
    }
}

fn parse_csv(raw: Option<String>) -> Vec<String> {
    raw.map(|value| {
        value
            .split(',')
            .map(str::trim)
            .filter(|item| !item.is_empty())
            .map(ToOwned::to_owned)
            .collect()
    })
    .unwrap_or_default()
}

fn resolve_allowed_hosts(
    explicit: Option<String>,
    bind_host: &str,
    env: &BTreeMap<String, String>,
) -> Option<Vec<String>> {
    if let Some(raw) = explicit {
        let items = parse_csv(Some(raw));
        if items.iter().any(|h| h == "*") {
            return None;
        }
        if !items.is_empty() {
            return Some(items);
        }
    }

    let mut hosts = vec!["localhost".into(), "127.0.0.1".into(), "::1".into()];
    if !matches!(bind_host, "0.0.0.0" | "::" | "[::]") && !hosts.iter().any(|h| h == bind_host) {
        hosts.push(bind_host.to_owned());
    }
    if let Some(app) = env_nonempty(env, "FLY_APP_NAME") {
        hosts.push(format!("{app}.fly.dev"));
        hosts.push(format!("{app}.internal"));
    }
    if let Some(domain) = env_nonempty(env, "RAILWAY_PUBLIC_DOMAIN") {
        hosts.push(domain);
    }
    if let Some(domain) = env_nonempty(env, "RAILWAY_PRIVATE_DOMAIN") {
        hosts.push(domain);
    }
    Some(hosts)
}

fn is_loopback_host(host: &str) -> bool {
    if host.eq_ignore_ascii_case("localhost") {
        return true;
    }
    let trimmed = host.trim_start_matches('[').trim_end_matches(']');
    trimmed.parse::<IpAddr>().is_ok_and(|ip| ip.is_loopback())
}

fn validate_origin_entry(entry: &str) -> Result<()> {
    if entry == "null" {
        return Ok(());
    }
    let normalized = entry.strip_suffix(":*").unwrap_or(entry);
    let url = reqwest::Url::parse(normalized)
        .with_context(|| format!("invalid allowed origin '{entry}'"))?;
    ensure!(
        matches!(url.scheme(), "http" | "https")
            && url.host_str().is_some()
            && url.username().is_empty()
            && url.password().is_none()
            && url.path() == "/"
            && url.query().is_none()
            && url.fragment().is_none(),
        "allowed origin '{entry}' must be an origin like https://example.com or http://localhost:*"
    );
    Ok(())
}

pub fn origin_matches_allowlist(origin: &str, allowed: &[String]) -> bool {
    if allowed.is_empty() {
        return false;
    }
    if origin == "null" {
        return allowed.iter().any(|entry| entry == "null");
    }
    let Ok(origin_url) = reqwest::Url::parse(origin) else {
        return false;
    };
    let Some(origin_host) = origin_url.host_str() else {
        return false;
    };
    let origin_port = origin_url.port_or_known_default();

    for entry in allowed {
        if entry == "null" {
            continue;
        }
        let any_port = entry.ends_with(":*");
        let normalized = entry.strip_suffix(":*").unwrap_or(entry);
        let Ok(entry_url) = reqwest::Url::parse(normalized) else {
            continue;
        };
        let Some(entry_host) = entry_url.host_str() else {
            continue;
        };
        if !origin_url.scheme().eq_ignore_ascii_case(entry_url.scheme()) {
            continue;
        }
        if !origin_host.eq_ignore_ascii_case(entry_host) {
            continue;
        }
        if any_port || origin_port == entry_url.port_or_known_default() {
            return true;
        }
    }
    false
}
