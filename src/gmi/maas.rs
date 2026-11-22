use anyhow::{ensure, Context, Result};
use reqwest::Client;
use serde::de::DeserializeOwned;
use serde_json::{json, Value};
use std::{sync::Arc, time::Duration};
use tokio::sync::Semaphore;

use crate::{check_text, config::Config, gmi::response_json};

#[derive(Clone)]
pub struct Maas {
    client: Client,
    config: Arc<Config>,
    slots: Arc<Semaphore>,
}

pub struct Completion<T> {
    pub value: T,
    pub usage: Option<Value>,
}

impl Maas {
    pub fn new(client: Client, config: Arc<Config>) -> Self {
        Self {
            client,
            config,
            slots: Arc::new(Semaphore::new(4)),
        }
    }

    /// Returns a tenant-scoped handle sharing the HTTP connection pool while
    /// isolating credentials and per-tenant concurrency slots.
    pub fn scoped(&self, config: Arc<Config>, slots: Arc<Semaphore>) -> Self {
        Self {
            client: self.client.clone(),
            config,
            slots,
        }
    }

    pub async fn json<T: DeserializeOwned>(
        &self,
        model: &str,
        timeout_seconds: u64,
        system: &str,
        data: &Value,
    ) -> Result<Completion<T>> {
        let key = self
            .config
            .maas_key
            .as_deref()
            .context("set GMI_MAAS_API_KEY or GMI_API_KEY")?;
        let _permit = self
            .slots
            .clone()
            .try_acquire_owned()
            .context("four inference requests already running; retry later")?;
        let content = serde_json::to_string(data)?;
        check_text(&content)?;
        let response = self.client.post(format!("{}/chat/completions", self.config.maas_base_url))
            .bearer_auth(key).timeout(Duration::from_secs(timeout_seconds))
            .json(&json!({"model": model, "temperature": 0.0, "stream": false, "max_tokens": 4096,
                "messages": [{"role": "system", "content": system}, {"role": "user", "content": content}]}))
            .send().await.map_err(|_| anyhow::anyhow!("MaaS network request failed or timed out"))?;
        let response = response_json(response).await?;
        let choice = response
            .pointer("/choices/0")
            .context("MaaS response missing choice")?;
        ensure!(
            choice.get("finish_reason").and_then(Value::as_str) == Some("stop"),
            "MaaS response incomplete or refused"
        );
        let text = choice
            .pointer("/message/content")
            .and_then(Value::as_str)
            .context("MaaS response missing text")?;
        let value = parse_json(text)?;
        Ok(Completion {
            value,
            usage: response.get("usage").cloned(),
        })
    }
}

pub fn parse_json<T: DeserializeOwned>(text: &str) -> Result<T> {
    let mut text = text.trim();
    // Only a leading reasoning block is stripped; JSON string values are preserved.
    if text.starts_with("<think>") {
        let end = text.find("</think>").context("unclosed reasoning block")?;
        text = text[end + "</think>".len()..].trim();
    }
    if let Some(rest) = text
        .strip_prefix("```json")
        .or_else(|| text.strip_prefix("```"))
    {
        text = rest
            .trim()
            .strip_suffix("```")
            .context("unclosed JSON code fence")?
            .trim();
    }
    serde_json::from_str(text).context("MaaS response did not match required JSON schema")
}
