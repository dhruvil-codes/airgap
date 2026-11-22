use std::{collections::BTreeMap, path::Path};

use anyhow::{ensure, Context, Result};

pub const DEFAULT_MODEL: &str = "deepseek-ai/DeepSeek-V4.1-Flash";
pub const DEFAULT_AGENTBOX_BASE_URL: &str = "https://api.gmi-serving.com/v1/agents";
pub const DEFAULT_MAAS_BASE_URL: &str = "https://api.gmi-serving.com/v1";
pub const DEFAULT_IDC: &str = "us-central-iowa16";
/// 2 vCPU / 4 GiB; the 2 GiB small SKU OOM-kills common Node and Rust test runners.
pub const DEFAULT_INSTANCE_TYPE: &str = "gmi.sandbox.medium";
pub const DEFAULT_NODE_IMAGE: &str = "docker.io/library/node:22-bookworm";

// Do not derive Debug: this structure holds credentials.
#[derive(Clone)]
pub struct Config {
    pub agentbox_key: Option<String>,
    pub maas_key: Option<String>,
    pub github_token: Option<String>,
    pub agentbox_base_url: String,
    pub maas_base_url: String,
    pub cache_base_url: Option<String>,
    pub node_image: String,
    pub idc: String,
    pub instance_type: String,
    pub model: String,
    pub timeout_seconds: u64,
}

impl Config {
    pub fn discover() -> Result<Self> {
        Self::load(&std::env::current_dir()?, std::env::vars().collect())
    }

    pub fn load(cwd: &Path, environment: BTreeMap<String, String>) -> Result<Self> {
        let mut file_values = BTreeMap::new();
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
                    // Critical security: never allow workspace .env files to override provider
                    // URLs, container images, compute SKUs, regions, or the classifier model.
                    if matches!(
                        key.as_str(),
                        "GMI_AGENTBOX_BASE_URL"
                            | "GMI_MAAS_BASE_URL"
                            | "AIRGAP_CACHE_URL"
                            | "AIRGAP_NODE_IMAGE"
                            | "GMI_AGENTBOX_INSTANCE_TYPE"
                            | "GMI_DEFAULT_IDC"
                            | "GMI_MODEL"
                    ) {
                        continue;
                    }
                    file_values.entry(key).or_insert(value);
                }
            }
        }
        // Process credentials (including fallback GMI_API_KEY / GH_TOKEN) must strictly take
        // precedence over .env files so an untrusted workspace cannot shadow process credentials.
        let env_has = |name: &str| environment.get(name).is_some_and(|v| !v.trim().is_empty());
        if env_has("GMI_API_KEY") {
            file_values.remove("GMI_AGENTBOX_API_KEY");
            file_values.remove("GMI_MAAS_API_KEY");
        }
        if env_has("GH_TOKEN") {
            file_values.remove("GITHUB_TOKEN");
        }
        let mut values = file_values;
        values.extend(environment);
        Self::from_values(values)
    }

    pub fn from_values(values: BTreeMap<String, String>) -> Result<Self> {
        let get = |name: &str| values.get(name).filter(|v| !v.trim().is_empty()).cloned();
        let config = Self {
            agentbox_key: get("GMI_AGENTBOX_API_KEY").or_else(|| get("GMI_API_KEY")),
            maas_key: get("GMI_MAAS_API_KEY").or_else(|| get("GMI_API_KEY")),
            github_token: get("GITHUB_TOKEN").or_else(|| get("GH_TOKEN")),
            agentbox_base_url: get("GMI_AGENTBOX_BASE_URL")
                .unwrap_or_else(|| DEFAULT_AGENTBOX_BASE_URL.into())
                .trim_end_matches('/')
                .into(),
            maas_base_url: get("GMI_MAAS_BASE_URL")
                .unwrap_or_else(|| DEFAULT_MAAS_BASE_URL.into())
                .trim_end_matches('/')
                .into(),
            cache_base_url: get("AIRGAP_CACHE_URL").map(|u| u.trim_end_matches('/').into()),
            node_image: get("AIRGAP_NODE_IMAGE").unwrap_or_else(|| DEFAULT_NODE_IMAGE.into()),
            idc: get("GMI_DEFAULT_IDC").unwrap_or_else(|| DEFAULT_IDC.into()),
            instance_type: get("GMI_AGENTBOX_INSTANCE_TYPE")
                .unwrap_or_else(|| DEFAULT_INSTANCE_TYPE.into()),
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
        if let Some(cache_base) = &config.cache_base_url {
            let url = reqwest::Url::parse(cache_base)
                .map_err(|_| anyhow::anyhow!("invalid cache base URL"))?;
            ensure!(
                url.scheme() == "https"
                    && url.host_str().is_some()
                    && url.username().is_empty()
                    && url.password().is_none()
                    && url.query().is_none()
                    && url.fragment().is_none(),
                "cache base URL must use HTTPS without userinfo, query, or fragment"
            );
        }
        validate_model(&config.model)?;
        ensure!(!config.idc.trim().is_empty(), "IDC cannot be empty");
        ensure!(
            !config.instance_type.is_empty()
                && config.instance_type.len() <= 100
                && config
                    .instance_type
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'-' | b'_')),
            "invalid GMI_AGENTBOX_INSTANCE_TYPE"
        );
        ensure!(
            !config.node_image.trim().is_empty()
                && config.node_image.len() <= 512
                && !config.node_image.chars().any(char::is_control),
            "invalid node image"
        );
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
