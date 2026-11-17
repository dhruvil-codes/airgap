use std::time::Duration;

use anyhow::{ensure, Result};
use reqwest::Client;
use serde_json::Value;

use crate::{config::Config, gmi, setup};

#[derive(Debug)]
pub struct Report {
    pub maas_ok: bool,
    pub compute_ok: bool,
    pub model_available: bool,
}

pub async fn check(config: &Config, client: &Client) -> Report {
    let models = async {
        let key = config.maas_key.as_deref()?;
        let response = client
            .get(format!("{}/models", config.maas_base_url))
            .bearer_auth(key)
            .timeout(Duration::from_secs(3))
            .send()
            .await
            .ok()?;
        gmi::response_json(response).await.ok()
    };
    let compute = async {
        let key = config.agentbox_key.as_deref()?;
        let response = client
            .get(format!("{}/eligibility", config.agentbox_base_url))
            .bearer_auth(key)
            .timeout(Duration::from_secs(3))
            .send()
            .await
            .ok()?;
        gmi::response_json(response).await.ok()
    };
    let (models, compute) = tokio::join!(models, compute);
    let model_available = models
        .as_ref()
        .and_then(|data| data.get("data"))
        .and_then(Value::as_array)
        .is_some_and(|items| {
            items
                .iter()
                .any(|item| item.get("id").and_then(Value::as_str) == Some(&config.model))
        });
    Report {
        maas_ok: models
            .as_ref()
            .and_then(|data| data.get("data"))
            .and_then(Value::as_array)
            .is_some(),
        compute_ok: compute
            .as_ref()
            .and_then(|value| value.get("eligible"))
            .and_then(Value::as_bool)
            == Some(true),
        model_available,
    }
}

fn status(ok: bool, label: &str, message: &str) {
    println!("[{}] {label}: {message}", if ok { "ok" } else { "!!" });
}

pub fn run() -> Result<()> {
    println!("airgap doctor\n=============");
    let config = Config::discover()?;
    status(
        config.agentbox_key.is_some(),
        "AgentBox key",
        if config.agentbox_key.is_some() {
            "present; checking compute eligibility"
        } else {
            "missing: set GMI_AGENTBOX_API_KEY"
        },
    );
    status(
        config.maas_key.is_some(),
        "MaaS key",
        if config.maas_key.is_some() {
            "present; checking model access"
        } else {
            "missing: set GMI_MAAS_API_KEY"
        },
    );
    let client = Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .no_proxy()
        .connect_timeout(Duration::from_secs(3))
        .timeout(Duration::from_secs(3))
        .build()?;
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    let report = runtime.block_on(check(&config, &client));
    status(
        report.maas_ok,
        "MaaS gateway",
        if report.maas_ok {
            "connected; credential accepted"
        } else {
            "unreachable, rejected, or invalid response"
        },
    );
    status(
        report.compute_ok,
        "AgentBox",
        if report.compute_ok {
            "compute eligibility confirmed"
        } else {
            "compute access not confirmed"
        },
    );
    status(
        report.model_available,
        "Configured model",
        if report.model_available {
            "available in model catalog"
        } else {
            "not confirmed in model catalog"
        },
    );
    let workspace = std::env::current_dir()?;
    let paths = setup::configured_paths(&workspace, setup::home_dir().as_deref());
    for path in &paths {
        status(true, "Editor config", &path.display().to_string());
    }
    if paths.is_empty() {
        status(false, "Editor config", "not found; run airgap setup --yes");
    }
    let ready = report.maas_ok && report.compute_ok && report.model_available && !paths.is_empty();
    if ready {
        println!("Connectivity and configuration checks passed. This does not execute a sandbox or verify client permissions.");
    }
    ensure!(
        ready,
        "some checks failed; see the diagnostic statuses above"
    );
    Ok(())
}
