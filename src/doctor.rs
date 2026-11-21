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

struct Check {
    ok: bool,
    label: String,
    detail: String,
    remedy: &'static str,
}

fn check_line(check: &Check) {
    if check.ok {
        println!(
            "  \x1b[32m✓\x1b[0m \x1b[1m{:<18}\x1b[0m \x1b[90m{}\x1b[0m",
            check.label, check.detail
        );
    } else {
        println!(
            "  \x1b[31m✗\x1b[0m \x1b[1;31m{:<18}\x1b[0m \x1b[31m{}\x1b[0m",
            check.label, check.detail
        );
        println!("    \x1b[33m→ {}\x1b[0m", check.remedy);
    }
}

const SPINNER: [char; 10] = ['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'];

async fn check_with_spinner(config: &Config, client: &Client) -> Report {
    use std::io::{IsTerminal, Write};

    let mut stderr = std::io::stderr();
    if !stderr.is_terminal() {
        return check(config, client).await;
    }
    let work = check(config, client);
    tokio::pin!(work);
    let mut tick = tokio::time::interval(Duration::from_millis(80));
    let mut frame = 0;
    let report = loop {
        tokio::select! {
            report = &mut work => break report,
            _ = tick.tick() => {
                let _ = write!(
                    stderr,
                    "\r\x1b[2K  \x1b[36m{}\x1b[0m Diagnosing GMI gateways, compute & editor configs...",
                    SPINNER[frame % SPINNER.len()]
                );
                let _ = stderr.flush();
                frame += 1;
            }
        }
    };
    let _ = write!(stderr, "\r\x1b[2K");
    let _ = stderr.flush();
    report
}

fn client_names(paths: &[(&'static str, std::path::PathBuf)]) -> Vec<&'static str> {
    let mut names = Vec::new();
    for (label, _) in paths {
        let name = label.split(" (").next().unwrap_or(label);
        if !names.contains(&name) {
            names.push(name);
        }
    }
    names
}

pub fn run(verbose: bool) -> Result<()> {
    println!("{}", crate::banner("doctor"));
    let config = Config::discover()?;
    let client = Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .no_proxy()
        .connect_timeout(Duration::from_secs(3))
        .timeout(Duration::from_secs(3))
        .build()?;
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    let report = runtime.block_on(check_with_spinner(&config, &client));
    let workspace = std::env::current_dir()?;
    let paths = setup::configured_paths(&workspace, setup::home_dir().as_deref());
    let model = config.model.rsplit('/').next().unwrap_or(&config.model);

    let mut checks = vec![
        Check {
            ok: config.agentbox_key.is_some(),
            label: "AgentBox key".into(),
            detail: if config.agentbox_key.is_some() {
                "present"
            } else {
                "missing"
            }
            .into(),
            remedy: "set GMI_AGENTBOX_API_KEY, then run airgap setup --yes",
        },
        Check {
            ok: config.maas_key.is_some(),
            label: "MaaS key".into(),
            detail: if config.maas_key.is_some() {
                "present"
            } else {
                "missing"
            }
            .into(),
            remedy: "set GMI_MAAS_API_KEY, then run airgap setup --yes",
        },
        Check {
            ok: report.maas_ok,
            label: "MaaS gateway".into(),
            detail: if report.maas_ok {
                "connected; credential accepted"
            } else {
                "unreachable, rejected, or invalid response"
            }
            .into(),
            remedy: "check GMI_MAAS_API_KEY and network access to the MaaS gateway",
        },
        Check {
            ok: report.compute_ok,
            label: "AgentBox compute".into(),
            detail: if report.compute_ok {
                "compute eligibility confirmed"
            } else {
                "compute access not confirmed"
            }
            .into(),
            remedy: "check GMI_AGENTBOX_API_KEY and that AgentBox is enabled for this account",
        },
        Check {
            ok: report.model_available,
            label: "Configured model".into(),
            detail: if report.model_available {
                format!("{} available", config.model)
            } else {
                format!("{} not confirmed in model catalog", config.model)
            },
            remedy: "set GMI_MODEL to a model in your catalog, or use airgap_configure",
        },
    ];
    checks.extend(paths.iter().map(|(label, path)| Check {
        ok: true,
        label: (*label).into(),
        detail: path.display().to_string(),
        remedy: "",
    }));
    if paths.is_empty() {
        checks.push(Check {
            ok: false,
            label: "Editor config".into(),
            detail: "no airgap registration found".into(),
            remedy: "run airgap setup --yes",
        });
    }

    let failed = checks.iter().filter(|check| !check.ok).count();
    if verbose {
        for check in &checks {
            check_line(check);
        }
        println!();
    } else if failed > 0 {
        for check in checks.iter().filter(|check| !check.ok) {
            check_line(check);
        }
        println!();
    } else {
        println!(
            "  \x1b[32m✓\x1b[0m \x1b[1mGateways:\x1b[0m AgentBox compute & MaaS model access confirmed \x1b[90m({model})\x1b[0m"
        );
        println!(
            "  \x1b[32m✓\x1b[0m \x1b[1mClients:\x1b[0m  {} active configuration{} \x1b[90m({})\x1b[0m · Host protected\n",
            paths.len(),
            if paths.len() == 1 { "" } else { "s" },
            client_names(&paths).join(", ")
        );
    }
    ensure!(
        failed == 0,
        "{failed} check{} failed{}",
        if failed == 1 { "" } else { "s" },
        if verbose {
            ""
        } else {
            "; run airgap doctor --verbose for all checks"
        }
    );
    Ok(())
}
