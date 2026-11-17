use rmcp::schemars;
use serde::{Deserialize, Serialize};

#[derive(Clone)]
pub struct RuntimeConfig {
    pub model: String,
    pub timeout_seconds: u64,
}

#[derive(Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ConfigureRequest {
    pub model: Option<String>,
    pub timeout_seconds: Option<u64>,
}

#[derive(Serialize)]
pub struct ConfigureResult {
    pub status: &'static str,
    pub active_model: String,
    pub timeout_seconds: u64,
}

pub fn configure(
    state: &mut RuntimeConfig,
    request: ConfigureRequest,
) -> anyhow::Result<ConfigureResult> {
    if let Some(model) = &request.model {
        crate::config::validate_model(model)?;
    }
    if let Some(seconds) = request.timeout_seconds {
        crate::config::validate_timeout(seconds)?;
    }
    if let Some(model) = request.model {
        state.model = model;
    }
    if let Some(seconds) = request.timeout_seconds {
        state.timeout_seconds = seconds;
    }
    Ok(ConfigureResult {
        status: "configured",
        active_model: state.model.clone(),
        timeout_seconds: state.timeout_seconds,
    })
}
