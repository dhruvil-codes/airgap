use anyhow::{Context, Result};
use rmcp::schemars;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum ManifestType {
    Npm,
    Python,
    Cargo,
}

#[derive(Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SupplyRequest {
    pub manifest_type: ManifestType,
    pub manifest_content: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RiskScore {
    Low,
    High,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SupplyResult {
    pub risk_score: RiskScore,
    pub flagged_scripts: Vec<String>,
    pub limitations: String,
}

pub fn audit(request: SupplyRequest) -> Result<SupplyResult> {
    crate::check_text(&request.manifest_content)?;
    let mut flagged = Vec::new();
    match request.manifest_type {
        ManifestType::Npm => {
            let manifest: serde_json::Value =
                serde_json::from_str(&request.manifest_content).context("invalid package.json")?;
            anyhow::ensure!(manifest.is_object(), "package.json must be an object");
            if let Some(scripts) = manifest.get("scripts") {
                let scripts = scripts
                    .as_object()
                    .context("npm scripts must be an object")?;
                for (name, value) in scripts {
                    let script = value.as_str().context("npm script must be a string")?;
                    if matches!(
                        name.as_str(),
                        "preinstall" | "install" | "postinstall" | "prepare"
                    ) || dangerous(script)
                    {
                        flagged.push(format!(
                            "npm script {name}: {}",
                            super::secrets::redact(script).sanitized_text
                        ));
                    }
                }
            }
            if let Some(dependencies) = manifest.as_object() {
                for key in ["dependencies", "devDependencies", "optionalDependencies"] {
                    if let Some(values) = dependencies.get(key).and_then(|v| v.as_object()) {
                        for (name, value) in values {
                            if value.as_str().is_some_and(remote) {
                                flagged.push(format!(
                                    "{key}/{name}: remote or local dependency requires review"
                                ));
                            }
                        }
                    }
                }
            }
        }
        ManifestType::Python => {
            for (index, line) in request.manifest_content.lines().enumerate() {
                let line = line.trim();
                if !line.is_empty()
                    && !line.starts_with('#')
                    && (remote(line)
                        || line.contains("--extra-index-url")
                        || line.contains("--index-url")
                        || line.starts_with("-r")
                        || line.starts_with("--requirement")
                        || line.starts_with("-c")
                        || line.starts_with("--constraint")
                        || line.contains("--trusted-host"))
                {
                    flagged.push(format!("requirements.txt line {}: external source, include, or index requires review", index + 1));
                }
            }
        }
        ManifestType::Cargo => {
            let manifest: toml::Value =
                toml::from_str(&request.manifest_content).context("invalid Cargo.toml")?;
            if let Some(build) = manifest.get("package").and_then(|v| v.get("build")) {
                if build.as_bool() != Some(false) {
                    flagged.push("package.build: build script executes code".into());
                }
            }
            for key in [
                "dependencies",
                "dev-dependencies",
                "build-dependencies",
                "patch",
                "replace",
                "target",
                "workspace",
            ] {
                if let Some(value) = manifest.get(key) {
                    inspect_cargo(key, value, &mut flagged);
                }
            }
        }
    }
    Ok(SupplyResult {
        risk_score: if flagged.is_empty() { RiskScore::Low } else { RiskScore::High },
        flagged_scripts: flagged,
        limitations: "Manifest heuristics only; low does not mean safe. No advisory database, package source, transitive dependency, implicit Cargo build.rs, or Python build backend inspection.".into(),
    })
}

fn inspect_cargo(path: &str, value: &toml::Value, flags: &mut Vec<String>) {
    if path == "build-dependencies" || path.ends_with("/build-dependencies") {
        flags.push(format!("{path}: dependencies execute at build time"));
    }
    if let Some(table) = value.as_table() {
        for (key, value) in table {
            let path = format!("{path}/{key}");
            if matches!(key.as_str(), "git" | "path" | "registry") {
                flags.push(format!(
                    "{path}: alternate dependency source requires review"
                ));
            }
            inspect_cargo(&path, value, flags);
        }
    }
}

fn remote(value: &str) -> bool {
    let value = value.to_ascii_lowercase();
    [
        "http://",
        "https://",
        "git+",
        "git://",
        "github:",
        "file:",
        "../",
        "./",
        " @ ",
        "-e ",
        "--editable",
    ]
    .iter()
    .any(|needle| value.contains(needle))
}

fn dangerous(value: &str) -> bool {
    let value = value.to_ascii_lowercase();
    [
        "curl ",
        "wget ",
        "powershell",
        "invoke-webrequest",
        "eval(",
        "base64",
        "chmod +x",
        "child_process",
        "/dev/tcp",
    ]
    .iter()
    .any(|needle| value.contains(needle))
}
