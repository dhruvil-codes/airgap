use std::{
    collections::BTreeMap,
    io::Cursor,
    sync::Arc,
    time::{Duration, Instant},
};

use anyhow::{bail, ensure, Context, Result};
use flate2::{write::GzEncoder, Compression};
use reqwest::{multipart, Client, Method};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tokio::sync::{oneshot, Semaphore};
use tokio_util::{sync::CancellationToken, task::TaskTracker};

use crate::{
    config::{validate_timeout, Config},
    gmi::response_json,
    tools::secrets::redact,
};

pub type Files = BTreeMap<String, String>;
pub const MAX_FILES_BYTES: usize = 8 * 1024 * 1024;

#[derive(Clone)]
pub struct AgentBox {
    client: Client,
    config: Arc<Config>,
    jobs: TaskTracker,
    shutdown: CancellationToken,
    slots: Arc<Semaphore>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionResult {
    pub exit_code: i32,
    pub stdout: String,
    pub stderr: String,
    pub duration_ms: u64,
    pub passed: bool,
    pub stdout_truncated: bool,
    pub stderr_truncated: bool,
}

impl AgentBox {
    pub fn new(client: Client, config: Arc<Config>) -> Self {
        Self {
            client,
            config,
            jobs: TaskTracker::new(),
            shutdown: CancellationToken::new(),
            slots: Arc::new(Semaphore::new(4)),
        }
    }

    pub async fn shutdown(&self) {
        self.shutdown.cancel();
        self.jobs.close();
        let _ = tokio::time::timeout(Duration::from_secs(35), self.jobs.wait()).await;
    }

    async fn request(&self, method: Method, path: &str, body: Option<Value>) -> Result<Value> {
        let operation_path = path
            .split('/')
            .enumerate()
            .map(
                |(index, part)| {
                    if matches!(index, 2 | 4) {
                        ":id"
                    } else {
                        part
                    }
                },
            )
            .collect::<Vec<_>>()
            .join("/");
        let operation = format!("AgentBox {method} {operation_path}");
        let key = self
            .config
            .agentbox_key
            .as_deref()
            .context("set GMI_AGENTBOX_API_KEY (compute scope) or GMI_API_KEY")?;
        let mut request = self
            .client
            .request(method, format!("{}{path}", self.config.agentbox_base_url))
            .bearer_auth(key);
        if let Some(body) = body {
            request = request.json(&body);
        }
        let response = request
            .send()
            .await
            .map_err(|_| anyhow::anyhow!("{operation}: network request failed"))?;
        response_json(response)
            .await
            .map_err(|error| anyhow::anyhow!("{operation}: {error}"))
    }

    pub async fn run(
        &self,
        command: String,
        files: Files,
        image: Option<String>,
        timeout_seconds: u64,
    ) -> Result<ExecutionResult> {
        validate_timeout(timeout_seconds)?;
        ensure!(
            !command.trim().is_empty() && command.len() <= 64 * 1024,
            "command must contain 1..65536 bytes"
        );
        validate_files(&files)?;
        ensure!(
            self.config.agentbox_key.is_some(),
            "set GMI_AGENTBOX_API_KEY (compute scope) or GMI_API_KEY"
        );
        ensure!(!self.shutdown.is_cancelled(), "server is shutting down");
        let permit = self
            .slots
            .clone()
            .try_acquire_owned()
            .context("four sandbox jobs already running; retry later")?;
        let image = image.unwrap_or_else(|| detect_image(&files).into());
        ensure!(
            !image.trim().is_empty() && image.len() <= 512 && !image.chars().any(char::is_control),
            "invalid image reference"
        );
        let archive = pack_files(&files)?;
        let this = self.clone();
        let (sender, receiver) = oneshot::channel();
        // A cancelled MCP caller cannot drop the cleanup future.
        self.jobs.spawn(async move {
            let _permit = permit;
            let result = this.run_job(command, archive, image, timeout_seconds).await;
            let _ = sender.send(result);
        });
        receiver.await.context("sandbox worker stopped")?
    }

    async fn run_job(
        &self,
        command: String,
        archive: Vec<u8>,
        image: String,
        seconds: u64,
    ) -> Result<ExecutionResult> {
        let mut slug = None;
        let mut sandbox = None;
        let start = Instant::now();
        // Provisioning gets its own budget: building a new image can take minutes.
        let job = async {
            let agent = self.request(Method::POST, "/deployments", Some(json!({
                "title": format!("airgap-{}", uuid::Uuid::new_v4().simple()),
                "image_url": image, "idc": self.config.idc, "instance_type": "gmi.sandbox.small",
                "runtime": "sandbox", "deployment_type": "gmi-ce", "start_cmd": "sleep infinity",
                "env": [], "ports": []
            }))).await?;
            let agent_slug = identifier(&agent, &["slug"])?;
            slug = Some(agent_slug.clone());
            let mut state = agent;
            loop {
                match state.get("template_build_status").and_then(Value::as_str) {
                    Some("ready") => break,
                    Some("error" | "deleted" | "delete_failed") => {
                        bail!("AgentBox template build failed")
                    }
                    _ => {}
                }
                tokio::time::sleep(Duration::from_millis(500)).await;
                state = self
                    .request(Method::GET, &format!("/deployments/{agent_slug}"), None)
                    .await?;
            }
            let launched = self
                .request(
                    Method::POST,
                    &format!("/deployments/{agent_slug}/tasks"),
                    Some(json!({
                        "idc_name": self.config.idc, "instance_type": "gmi.sandbox.small",
                        "timeout_seconds": seconds + 180, "env": [], "ports": [],
                        "metadata": {"application": "airgap"}
                    })),
                )
                .await?;
            let id = identifier(&launched, &["task_id", "id"])?;
            sandbox = Some(id.clone());
            loop {
                let state = self
                    .request(Method::GET, &format!("/tasks/{id}"), None)
                    .await?;
                match state
                    .get("task_status")
                    .or_else(|| state.get("status"))
                    .and_then(Value::as_str)
                {
                    Some("running") => break,
                    Some("error" | "deleted" | "stopped" | "stopping") => {
                        bail!("AgentBox sandbox failed to start")
                    }
                    _ => tokio::time::sleep(Duration::from_millis(500)).await,
                }
            }
            self.upload(&id, archive).await?;
            let setup = self
                .execute(
                    &id,
                    "mkdir -p /workspace && tar -xzf /tmp/airgap-input.tar.gz -C /workspace".into(),
                    30,
                )
                .await?;
            ensure!(setup.exit_code == 0, "remote workspace extraction failed");
            self.execute(
                &id,
                format!("cd /workspace && exec sh -c {}", shell_quote(&command)),
                seconds,
            )
            .await
        };
        let result = tokio::select! {
            _ = self.shutdown.cancelled() => Err(anyhow::anyhow!("server shutting down; sandbox cancelled")),
            result = tokio::time::timeout(Duration::from_secs(seconds + 600), job) => result.context("sandbox provisioning or execution deadline exceeded").and_then(|r| r),
        };
        // Explicit awaited cleanup, including failures. Drop cannot await network IO.
        let cleanup = tokio::time::timeout(
            Duration::from_secs(30),
            self.cleanup(sandbox.as_deref(), slug.as_deref()),
        )
        .await;
        match (result, cleanup) {
            (Ok(mut result), Ok(Ok(()))) => {
                result.duration_ms = start.elapsed().as_millis() as u64;
                Ok(result)
            }
            (Err(error), Ok(Ok(()))) => Err(error),
            (_, _) => bail!("sandbox job ended but remote cleanup failed; inspect AgentBox resources (sandbox lifetime is bounded)"),
        }
    }

    async fn upload(&self, id: &str, archive: Vec<u8>) -> Result<()> {
        let form = multipart::Form::new().part(
            "file",
            multipart::Part::bytes(archive)
                .file_name("airgap-input.tar.gz")
                .mime_str("application/gzip")?,
        );
        let response = self
            .client
            .post(format!(
                "{}/tasks/{id}/files",
                self.config.agentbox_base_url
            ))
            .bearer_auth(
                self.config
                    .agentbox_key
                    .as_deref()
                    .context("missing compute key")?,
            )
            .query(&[("path", "/tmp/airgap-input.tar.gz")])
            .multipart(form)
            .send()
            .await
            .map_err(|_| anyhow::anyhow!("AgentBox file upload failed"))?;
        response_json(response)
            .await
            .map_err(|error| anyhow::anyhow!("AgentBox file upload: {error}"))?;
        Ok(())
    }

    async fn execute(&self, id: &str, command: String, seconds: u64) -> Result<ExecutionResult> {
        let start = Instant::now();
        let mut execution = self
            .request(
                Method::POST,
                &format!("/tasks/{id}/executions"),
                Some(json!({"command": command, "wait": false})),
            )
            .await?;
        let execution_id = identifier(&execution, &["execution_id", "id"])?;
        let polling = async {
            loop {
                let status = execution
                    .get("status")
                    .and_then(Value::as_str)
                    .unwrap_or("");
                if matches!(
                    status,
                    "completed"
                        | "failed"
                        | "succeeded"
                        | "finished"
                        | "exited"
                        | "error"
                        | "cancelled"
                        | "canceled"
                        | "timed_out"
                ) {
                    let code = execution
                        .get("exit_code")
                        .and_then(Value::as_i64)
                        .context("terminal execution has no exit code")?;
                    let exit_code = i32::try_from(code).context("invalid execution exit code")?;
                    let stdout = self.sanitize(
                        execution
                            .get("stdout")
                            .and_then(Value::as_str)
                            .unwrap_or(""),
                    );
                    let stderr = self.sanitize(
                        execution
                            .get("stderr")
                            .and_then(Value::as_str)
                            .unwrap_or(""),
                    );
                    return Ok(ExecutionResult {
                        exit_code,
                        stdout,
                        stderr,
                        duration_ms: start.elapsed().as_millis() as u64,
                        passed: exit_code == 0
                            && !matches!(
                                status,
                                "failed" | "error" | "cancelled" | "canceled" | "timed_out"
                            ),
                        stdout_truncated: execution
                            .get("stdout_truncated")
                            .and_then(Value::as_bool)
                            .unwrap_or(false),
                        stderr_truncated: execution
                            .get("stderr_truncated")
                            .and_then(Value::as_bool)
                            .unwrap_or(false),
                    });
                }
                tokio::time::sleep(Duration::from_millis(250)).await;
                execution = self
                    .request(
                        Method::GET,
                        &format!("/tasks/{id}/executions/{execution_id}"),
                        None,
                    )
                    .await?;
            }
        };
        match tokio::time::timeout(Duration::from_secs(seconds), polling).await {
            Ok(result) => result,
            Err(_) => {
                let _ = self
                    .request(
                        Method::POST,
                        &format!("/tasks/{id}/executions/{execution_id}/cancel"),
                        None,
                    )
                    .await;
                bail!("remote command exceeded {seconds} second deadline");
            }
        }
    }

    fn sanitize(&self, text: &str) -> String {
        let mut output = text.to_owned();
        for key in [&self.config.agentbox_key, &self.config.maas_key]
            .into_iter()
            .flatten()
        {
            if !key.is_empty() {
                output = output.replace(key, "[REDACTED]");
            }
        }
        redact(&output).sanitized_text
    }

    async fn cleanup(&self, sandbox: Option<&str>, slug: Option<&str>) -> Result<()> {
        if let Some(id) = sandbox {
            self.delete(&format!("/tasks/{id}")).await?;
        }
        if let Some(slug) = slug {
            self.delete(&format!("/deployments/{slug}")).await?;
        }
        Ok(())
    }

    async fn delete(&self, path: &str) -> Result<()> {
        for attempt in 0..5 {
            let response = self
                .client
                .delete(format!("{}{path}", self.config.agentbox_base_url))
                .bearer_auth(
                    self.config
                        .agentbox_key
                        .as_deref()
                        .context("missing compute key")?,
                )
                .send()
                .await
                .map_err(|_| anyhow::anyhow!("AgentBox cleanup network error"))?;
            if response.status().is_success() || response.status() == reqwest::StatusCode::NOT_FOUND
            {
                return Ok(());
            }
            if !matches!(response.status().as_u16(), 409 | 429 | 500..=599) {
                bail!("AgentBox cleanup rejected");
            }
            tokio::time::sleep(Duration::from_millis(250 * (attempt + 1))).await;
        }
        bail!("AgentBox cleanup retries exhausted")
    }
}

fn identifier(value: &Value, keys: &[&str]) -> Result<String> {
    let id = keys
        .iter()
        .find_map(|key| value.get(*key).and_then(Value::as_str))
        .context("provider response missing resource identifier")?;
    ensure!(
        !id.is_empty()
            && id.len() <= 200
            && id
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_'),
        "invalid provider resource identifier"
    );
    Ok(id.into())
}

pub fn shell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\"'\"'"))
}

pub fn validate_files(files: &Files) -> Result<()> {
    ensure!(files.len() <= 2000, "maximum 2000 files per snapshot");
    let mut total = 0usize;
    for (path, content) in files {
        ensure!(
            !path.is_empty()
                && path.len() <= 240
                && !path.contains(['\\', ':'])
                && !path.chars().any(char::is_control),
            "invalid snapshot path"
        );
        ensure!(
            path.split('/').all(|part| !part.is_empty()
                && part != "."
                && part != ".."
                && !part.eq_ignore_ascii_case(".git")),
            "paths must be relative without traversal or .git metadata"
        );
        total = total
            .checked_add(path.len() + content.len())
            .context("snapshot size overflow")?;
        ensure!(total <= MAX_FILES_BYTES, "snapshot exceeds 8 MiB limit");
    }
    Ok(())
}

pub fn pack_files(files: &Files) -> Result<Vec<u8>> {
    validate_files(files)?;
    let mut archive = tar::Builder::new(GzEncoder::new(Vec::new(), Compression::fast()));
    for (path, content) in files {
        let mut header = tar::Header::new_gnu();
        header.set_size(content.len() as u64);
        header.set_mode(0o644);
        header.set_mtime(0);
        header.set_cksum();
        archive.append_data(&mut header, path, Cursor::new(content.as_bytes()))?;
    }
    Ok(archive.into_inner()?.finish()?)
}

pub fn detect_image(files: &Files) -> &'static str {
    if files.contains_key("Cargo.toml") {
        "docker.io/library/rust:1-bookworm"
    } else if files.contains_key("package.json") {
        "docker.io/library/node:20-bookworm"
    } else {
        "docker.io/library/python:3.12-bookworm"
    }
}
