pub mod agentbox;
pub mod maas;

use anyhow::{bail, Context, Result};
use reqwest::{Client, Response};
use serde_json::Value;
use std::time::Duration;

pub fn http_client() -> Result<Client> {
    Ok(Client::builder()
        .connect_timeout(Duration::from_secs(10))
        .timeout(Duration::from_secs(30))
        .redirect(reqwest::redirect::Policy::none())
        .no_proxy()
        .user_agent(concat!("airgap/", env!("CARGO_PKG_VERSION")))
        .build()?)
}

pub async fn response_json(mut response: Response) -> Result<Value> {
    let status = response.status();
    if !status.is_success() {
        // Provider errors may echo credentials or arbitrary remote text.
        bail!("provider returned HTTP {}", status.as_u16());
    }
    let mut body = Vec::new();
    while let Some(chunk) = response
        .chunk()
        .await
        .context("provider response interrupted")?
    {
        anyhow::ensure!(
            body.len() + chunk.len() <= 3 * 1024 * 1024,
            "provider response exceeds 3 MiB limit"
        );
        body.extend_from_slice(&chunk);
    }
    if body.is_empty() {
        return Ok(Value::Null);
    }
    serde_json::from_slice(&body).context("invalid provider JSON response")
}
