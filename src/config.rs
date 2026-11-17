use std::{collections::BTreeMap, path::Path};

use anyhow::{ensure, Context, Result};

pub const DEFAULT_MODEL: &str = "deepseek-ai/DeepSeek-V4.1-Flash";

// Do not derive Debug: this structure holds credentials.
#[derive(Clone)]
pub struct Config {
    pub agentbox_key: Option<String>,
    pub maas_key: Option<String>,
    pub agentbox_base_url: String,
    pub maas_base_url: String,
    pub idc: String,
    pub model: String,
    pub timeout_seconds: u64,
}

impl Config {
    pub fn discover() -> Result<Self> {
        Self::load(&std::env::current_dir()?, std::env::vars().collect())
    }

    pub fn load(cwd: &Path, environment: BTreeMap<String, String>) -> Result<Self> {
        let mut values = BTreeMap::new();
        for dir in cwd.ancestors().take(5) {
            for name in [".env", "cloudvet.env"] {
                let path = dir.join(name);
                if !path.try_exists()? {
                    continue;
                }
                for entry in
                    dotenvy::from_path_iter(&path).context("cannot read environment file")?
                {
                    // Never include parse errors: dotenv errors can contain credential values.
                    let (key, value) =
                        entry.map_err(|_| anyhow::anyhow!("invalid environment file syntax"))?;
                    values.entry(key).or_insert(value);
                }
            }
        }
        values.extend(environment);
        Self::from_values(values)
    }

    pub fn from_values(values: BTreeMap<String, String>) -> Result<Self> {
        let get = |name: &str| values.get(name).filter(|v| !v.trim().is_empty()).cloned();
        let config = Self {
            agentbox_key: get("GMI_AGENTBOX_API_KEY").or_else(|| get("GMI_API_KEY")),
            maas_key: get("GMI_MAAS_API_KEY").or_else(|| get("GMI_API_KEY")),
            agentbox_base_url: get("GMI_AGENTBOX_BASE_URL")
                .unwrap_or_else(|| "https://api.gmi-serving.com/v1/agents".into())
                .trim_end_matches('/')
                .into(),
            maas_base_url: get("GMI_MAAS_BASE_URL")
                .unwrap_or_else(|| "https://api.gmi-serving.com/v1".into())
                .trim_end_matches('/')
                .into(),
            idc: get("GMI_DEFAULT_IDC").unwrap_or_else(|| "us-central-iowa16".into()),
            model: get("GMI_MODEL").unwrap_or_else(|| DEFAULT_MODEL.into()),
            timeout_seconds: 120,
        };
        for base in [&config.agentbox_base_url, &config.maas_base_url] {
            let url = reqwest::Url::parse(base)
                .map_err(|_| anyhow::anyhow!("invalid provider base URL"))?;
            ensure!(
                url.scheme() == "https"
                    && url.host_str().is_some()
                    && url.username().is_empty()
                    && url.password().is_none()
                    && url.query().is_none()
                    && url.fragment().is_none(),
                "provider base URLs must use HTTPS without userinfo, query, or fragment"
            );
        }
        validate_model(&config.model)?;
        ensure!(!config.idc.trim().is_empty(), "IDC cannot be empty");
        Ok(config)
    }
}

pub fn validate_timeout(seconds: u64) -> Result<()> {
    ensure!(
        (1..=1800).contains(&seconds),
        "timeout must be between 1 and 1800 seconds"
    );
    Ok(())
}

pub fn validate_model(model: &str) -> Result<()> {
    ensure!(
        !model.trim().is_empty() && model.len() <= 200 && !model.chars().any(char::is_control),
        "invalid model identifier"
    );
    Ok(())
}
