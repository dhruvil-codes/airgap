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
use tokio::sync::{oneshot, Mutex, Semaphore};
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
    template_lock: Arc<Mutex<()>>,
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cleanup_warning: Option<String>,
}

impl AgentBox {
    pub fn new(client: Client, config: Arc<Config>) -> Self {
        Self {
            client,
            config,
            jobs: TaskTracker::new(),
            shutdown: CancellationToken::new(),
            slots: Arc::new(Semaphore::new(4)),
            template_lock: Arc::new(Mutex::new(())),
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
            !command.trim().is_empty() && command.len() <= 64 * 1024 && !command.contains('\0'),
            "command must contain 1..65536 bytes without NUL"
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
        let cancel = CancellationToken::new();
        let worker_cancel = cancel.clone();
        let guard = cancel.drop_guard();
        // A cancelled MCP caller cancels execution immediately while cleanup still runs to completion.
        self.jobs.spawn(async move {
            let _permit = permit;
            let result = this
                .run_job(command, archive, image, timeout_seconds, worker_cancel)
                .await;
            let _ = sender.send(result);
        });
        let result = receiver.await.context("sandbox worker stopped")?;
        guard.disarm();
        result
    }

    /// Runs a self-contained POSIX shell script inside a disposable sandbox without
    /// packing or extracting a file archive. Uses a single execution call (`sh /tmp/airgap-run.sh`)
    /// to save an API round-trip and avoid provider command-filter rejections on complex inline shells.
    pub async fn run_script(
        &self,
        script: String,
        image: String,
        timeout_seconds: u64,
    ) -> Result<ExecutionResult> {
        validate_timeout(timeout_seconds)?;
        ensure!(
            !script.trim().is_empty() && script.len() <= 128 * 1024 && !script.contains('\0'),
            "script must contain 1..131072 bytes without NUL"
        );
        ensure!(
            self.config.agentbox_key.is_some(),
            "set GMI_AGENTBOX_API_KEY (compute scope) or GMI_API_KEY"
        );
        ensure!(!self.shutdown.is_cancelled(), "server is shutting down");
        ensure!(
            !image.trim().is_empty() && image.len() <= 512 && !image.chars().any(char::is_control),
            "invalid image reference"
        );
        let permit = self
            .slots
            .clone()
            .try_acquire_owned()
            .context("four sandbox jobs already running; retry later")?;
        let this = self.clone();
        let (sender, receiver) = oneshot::channel();
        let cancel = CancellationToken::new();
        let worker_cancel = cancel.clone();
        let guard = cancel.drop_guard();
        // A cancelled MCP caller cancels execution immediately while cleanup still runs to completion.
        self.jobs.spawn(async move {
            let _permit = permit;
            let result = this
                .run_script_job(script, image, timeout_seconds, worker_cancel)
                .await;
            let _ = sender.send(result);
        });
        let result = receiver.await.context("sandbox worker stopped")?;
        guard.disarm();
        result
    }

    pub fn default_node_image(&self) -> &str {
        &self.config.node_image
    }

    pub fn cache_base_url(&self) -> Option<&str> {
        self.config.cache_base_url.as_deref()
    }

    async fn run_job(
        &self,
        command: String,
        archive: Vec<u8>,
        image: String,
        seconds: u64,
        cancel: CancellationToken,
    ) -> Result<ExecutionResult> {
        let mut sandbox = None;
        let start = Instant::now();
        // Provisioning gets its own budget: building a new image can take minutes.
        let job = async {
            let agent_slug = self.template(&image).await?;
            let id = self.launch_task(&agent_slug, seconds).await?;
            sandbox = Some(id.clone());
            self.wait_running(&id).await?;
            self.upload(&id, archive).await?;
            let setup = self
                .execute(
                    &id,
                    // /tmp is a 1 GiB tmpfs; package managers need the 20 GiB root disk.
                    "mkdir -p /workspace /root/tmp && export TMPDIR=/root/tmp && tar -xzf /tmp/airgap-input.tar.gz -C /workspace && (corepack enable 2>/dev/null || true)".into(),
                    30,
                )
                .await?;
            ensure!(setup.exit_code == 0, "remote workspace extraction failed");
            self.execute(
                &id,
                format!(
                    "cd /workspace && export TMPDIR=/root/tmp && export PNPM_HOME=/root/.local/share/pnpm && export PATH=\"$PNPM_HOME:$PATH\" && exec sh -c {}",
                    shell_quote(&command)
                ),
                seconds,
            )
            .await
        };
        let result = tokio::select! {
            _ = self.shutdown.cancelled() => Err(anyhow::anyhow!("server shutting down; sandbox cancelled")),
            _ = cancel.cancelled() => Err(anyhow::anyhow!("caller cancelled sandbox request")),
            result = tokio::time::timeout(Duration::from_secs(seconds + 600), job) => result.context("sandbox provisioning or execution deadline exceeded").and_then(|r| r),
        };
        self.finalize_job(result, sandbox.as_deref(), start).await
    }

    async fn run_script_job(
        &self,
        script: String,
        image: String,
        seconds: u64,
        cancel: CancellationToken,
    ) -> Result<ExecutionResult> {
        let mut sandbox = None;
        let start = Instant::now();
        let job = async {
            let agent_slug = self.template(&image).await?;
            let id = self.launch_task(&agent_slug, seconds).await?;
            sandbox = Some(id.clone());
            self.wait_running(&id).await?;
            self.upload_bytes(
                &id,
                "/tmp/airgap-run.sh",
                "airgap-run.sh",
                "text/x-shellscript",
                script.into_bytes(),
            )
            .await?;
            self.execute(&id, "sh /tmp/airgap-run.sh".into(), seconds)
                .await
        };
        let result = tokio::select! {
            _ = self.shutdown.cancelled() => Err(anyhow::anyhow!("server shutting down; sandbox cancelled")),
            _ = cancel.cancelled() => Err(anyhow::anyhow!("caller cancelled sandbox request")),
            result = tokio::time::timeout(Duration::from_secs(seconds + 600), job) => result.context("sandbox provisioning or execution deadline exceeded").and_then(|r| r),
        };
        self.finalize_job(result, sandbox.as_deref(), start).await
    }

    async fn launch_task(&self, agent_slug: &str, seconds: u64) -> Result<String> {
        let launched = self
            .request(
                Method::POST,
                &format!("/deployments/{agent_slug}/tasks"),
                Some(json!({
                    "idc_name": self.config.idc, "instance_type": self.config.instance_type,
                    "timeout_seconds": seconds + 180, "env": [], "ports": [],
                    "metadata": {"application": "airgap"}
                })),
            )
            .await?;
        identifier(&launched, &["task_id", "id"])
    }

    async fn wait_running(&self, id: &str) -> Result<()> {
        loop {
            let state = self
                .request(Method::GET, &format!("/tasks/{id}"), None)
                .await?;
            match state
                .get("task_status")
                .or_else(|| state.get("status"))
                .and_then(Value::as_str)
            {
                Some("running") => return Ok(()),
                Some("error" | "deleted" | "stopped" | "stopping") => {
                    bail!("AgentBox sandbox failed to start")
                }
                _ => tokio::time::sleep(Duration::from_millis(250)).await,
            }
        }
    }

    async fn finalize_job(
        &self,
        result: Result<ExecutionResult>,
        sandbox: Option<&str>,
        start: Instant,
    ) -> Result<ExecutionResult> {
        // Explicit awaited cleanup, including failures. Drop cannot await network IO.
        let cleanup = tokio::time::timeout(Duration::from_secs(30), self.cleanup(sandbox)).await;
        let cleanup_failed = !matches!(cleanup, Ok(Ok(())));
        const CLEANUP_WARNING: &str =
            "remote cleanup failed; inspect AgentBox resources (sandbox lifetime is bounded)";
        match result {
            Ok(mut result) => {
                result.duration_ms = start.elapsed().as_millis() as u64;
                if cleanup_failed {
                    result.cleanup_warning = Some(CLEANUP_WARNING.into());
                }
                Ok(result)
            }
            Err(error) if cleanup_failed => Err(error.context(CLEANUP_WARNING)),
            Err(error) => Err(error),
        }
    }

    /// Returns the slug of a ready template for `image`, reusing a warm one when the
    /// provider already has it. Templates are never deleted, so later runs skip the build.
    async fn template(&self, image: &str) -> Result<String> {
        // Serialize template lookup and creation so concurrent cold runs do not race
        // to create duplicate permanent templates on GMI Cloud.
        let _guard = self.template_lock.lock().await;
        let title = template_title(image, &self.config.instance_type);
        // Discovery is an optimization; a failed listing falls back to creating a template.
        if let Ok(listing) = self.request(Method::GET, "/deployments", None).await {
            if let Some(slug) = ready_template(&listing, &title, image) {
                return Ok(slug);
            }
            // Live GMI `GET /deployments` omits `template_build_status` from list items;
            // verify every matching deployment via `GET /deployments/{slug}` before creating a duplicate.
            for slug in unlisted_status_templates(&listing, &title, image) {
                if let Ok(detail) = self
                    .request(Method::GET, &format!("/deployments/{slug}"), None)
                    .await
                {
                    if detail.get("template_build_status").and_then(Value::as_str) == Some("ready")
                    {
                        return Ok(slug);
                    }
                }
            }
        }
        let agent = self.request(Method::POST, "/deployments", Some(json!({
            "title": title,
            "image_url": image, "idc": self.config.idc, "instance_type": self.config.instance_type,
            "runtime": "sandbox", "deployment_type": "gmi-ce", "start_cmd": "sleep infinity",
            "env": [], "ports": []
        }))).await?;
        let agent_slug = identifier(&agent, &["slug"])?;
        let mut state = agent;
        loop {
            match state.get("template_build_status").and_then(Value::as_str) {
                Some("ready") => return Ok(agent_slug),
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
    }

    async fn upload(&self, id: &str, archive: Vec<u8>) -> Result<()> {
        self.upload_bytes(
            id,
            "/tmp/airgap-input.tar.gz",
            "airgap-input.tar.gz",
            "application/gzip",
            archive,
        )
        .await
    }

    async fn upload_bytes(
        &self,
        id: &str,
        remote_path: &str,
        file_name: &str,
        mime: &str,
        bytes: Vec<u8>,
    ) -> Result<()> {
        let form = multipart::Form::new().part(
            "file",
            multipart::Part::bytes(bytes)
                .file_name(file_name.to_owned())
                .mime_str(mime)?,
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
            .query(&[("path", remote_path)])
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
                    // Timed-out, cancelled and errored runs report a null exit code.
                    let exit_code = match execution.get("exit_code").and_then(Value::as_i64) {
                        Some(code) => i32::try_from(code).unwrap_or(-1),
                        None => match status {
                            "timed_out" => 124,
                            "cancelled" | "canceled" => 130,
                            _ => -1,
                        },
                    };
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
                    let passed = !matches!(
                        status,
                        "failed" | "error" | "cancelled" | "canceled" | "timed_out"
                    ) && crate::tools::sandbox::output_passed(
                        exit_code.into(),
                        &stdout,
                        &stderr,
                    );
                    return Ok(ExecutionResult {
                        exit_code,
                        stdout,
                        stderr,
                        duration_ms: start.elapsed().as_millis() as u64,
                        passed,
                        stdout_truncated: execution
                            .get("stdout_truncated")
                            .and_then(Value::as_bool)
                            .unwrap_or(false),
                        stderr_truncated: execution
                            .get("stderr_truncated")
                            .and_then(Value::as_bool)
                            .unwrap_or(false),
                        cleanup_warning: None,
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
        for key in [
            &self.config.agentbox_key,
            &self.config.maas_key,
            &self.config.github_token,
        ]
        .into_iter()
        .flatten()
        {
            if !key.is_empty() {
                output = output.replace(key, "[REDACTED]");
            }
        }
        redact(&output).sanitized_text
    }

    /// Deletes only the ephemeral task; the deployment template stays warm for reuse.
    async fn cleanup(&self, sandbox: Option<&str>) -> Result<()> {
        match sandbox {
            Some(id) => self.delete(&format!("/tasks/{id}")).await,
            None => Ok(()),
        }
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

/// Deterministic template title, so every run with the same image and instance type
/// finds the same warm template.
pub fn template_title(image: &str, instance_type: &str) -> String {
    let safe_image = image
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect::<String>()
        .trim_matches('-')
        .to_string();
    format!("airgap-{safe_image}-{instance_type}")
}

/// Finds a ready template named `title` in a `GET /deployments` response, which may be a
/// bare array or an object wrapping the array.
fn ready_template(listing: &Value, title: &str, image: &str) -> Option<String> {
    let deployments = listing.as_array().or_else(|| {
        ["deployments", "data", "items", "results"]
            .iter()
            .find_map(|key| listing.get(*key).and_then(Value::as_array))
    })?;
    deployments.iter().find_map(|deployment| {
        let matches = deployment.get("title").and_then(Value::as_str) == Some(title)
            && deployment.get("template_build_status").and_then(Value::as_str) == Some("ready")
            // Distinct images can sanitize to the same title.
            && deployment
                .get("image_url")
                .and_then(Value::as_str)
                .is_none_or(|url| url == image);
        if matches {
            identifier(deployment, &["slug"]).ok()
        } else {
            None
        }
    })
}

/// Finds all matching deployments in `GET /deployments` whose list items omit
/// `template_build_status` (the real GMI list schema), so `template()` can confirm
/// readiness via `GET /deployments/{slug}` without stopping at a stale/failed match.
fn unlisted_status_templates(listing: &Value, title: &str, image: &str) -> Vec<String> {
    let Some(deployments) = listing.as_array().or_else(|| {
        ["deployments", "data", "items", "results"]
            .iter()
            .find_map(|key| listing.get(*key).and_then(Value::as_array))
    }) else {
        return Vec::new();
    };
    deployments
        .iter()
        .filter_map(|deployment| {
            let matches = deployment.get("title").and_then(Value::as_str) == Some(title)
                && deployment.get("template_build_status").is_none()
                && deployment
                    .get("image_url")
                    .and_then(Value::as_str)
                    .is_none_or(|url| url == image);
            matches
                .then(|| identifier(deployment, &["slug"]).ok())
                .flatten()
        })
        .collect()
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
    if files.contains_key("Cargo.toml") || files.keys().any(|p| p.ends_with(".rs")) {
        "docker.io/library/rust:1-bookworm"
    } else if files.contains_key("package.json")
        || files.keys().any(|p| {
            p.ends_with(".ts")
                || p.ends_with(".tsx")
                || p.ends_with(".js")
                || p.ends_with(".jsx")
                || p.ends_with(".mjs")
                || p.ends_with(".cjs")
        })
    {
        crate::config::DEFAULT_NODE_IMAGE
    } else {
        "docker.io/library/python:3.12-bookworm"
    }
}
