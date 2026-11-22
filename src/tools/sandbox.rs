use std::sync::LazyLock;

use anyhow::{ensure, Result};
use regex::Regex;
use rmcp::schemars;
use serde::{Deserialize, Serialize};

use crate::{
    gmi::agentbox::{AgentBox, ExecutionResult, Files},
    receipt::Receipt,
};

#[derive(Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SandboxRequest {
    pub command: String,
    pub files: Option<Files>,
    pub image: Option<String>,
    pub timeout_secs: Option<u64>,
}

#[derive(Deserialize, Serialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct VerifyRequest {
    pub diff: String,
    pub test_command: String,
    /// Required clean base snapshot. Nothing is read from the local working tree.
    pub files: Files,
    pub image: Option<String>,
    pub timeout_secs: Option<u64>,
}

#[derive(Serialize)]
pub struct VerifyResult {
    pub verified: bool,
    pub command_passed: bool,
    pub tests_passed: Option<u64>,
    pub tests_failed: Option<u64>,
    pub evidence_receipt: Receipt,
    pub execution: ExecutionResult,
}

pub async fn verify(
    agentbox: &AgentBox,
    request: VerifyRequest,
    default_timeout: u64,
) -> Result<VerifyResult> {
    crate::check_text(&request.diff)?;
    ensure!(
        request.diff.starts_with("diff --git "),
        "expected a unified git diff"
    );
    ensure!(
        !request.test_command.trim().is_empty() && request.test_command.len() <= 64 * 1024,
        "invalid test command"
    );
    ensure!(
        !request.files.is_empty(),
        "verification requires a nonempty base snapshot"
    );
    ensure!(
        !request
            .files
            .keys()
            .any(|key| key == ".airgap" || key.starts_with(".airgap/")),
        ".airgap is reserved for verification"
    );
    let input_hash = crate::receipt::fingerprint(&request)?;
    let mut files = request.files;
    files.insert(".airgap/patch.diff".into(), request.diff);
    let command = format!("git init -q && git apply --check .airgap/patch.diff && git apply .airgap/patch.diff && exec sh -c {}", crate::gmi::agentbox::shell_quote(&request.test_command));
    let execution = agentbox
        .run(
            command,
            files,
            request.image,
            request.timeout_secs.unwrap_or(default_timeout),
        )
        .await?;
    let (passed, failed) = test_counts(&format!("{}\n{}", execution.stdout, execution.stderr));
    let verified = execution.passed
        && !execution.stdout_truncated
        && !execution.stderr_truncated
        && passed.is_some_and(|n| n > 0)
        && failed == Some(0);
    let receipt = Receipt::from_input_hash(input_hash, &execution, execution.duration_ms, None)?;
    Ok(VerifyResult {
        verified,
        command_passed: execution.passed,
        tests_passed: passed,
        tests_failed: failed,
        evidence_receipt: receipt,
        execution,
    })
}

/// Exit 0 alone is not a pass: OOM kills and failed test summaries can still exit 0.
pub fn output_passed(exit_code: i64, stdout: &str, stderr: &str) -> bool {
    exit_code == 0
        && !killed(stdout)
        && !killed(stderr)
        && !reports_failed_tests(&format!("{stdout}\n{stderr}"))
}

pub fn killed(output: &str) -> bool {
    static KILLED: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"\b(?:Killed|SIGKILL)\b").expect("static regex"));
    KILLED.is_match(output)
}

pub fn reports_failed_tests(output: &str) -> bool {
    test_counts(output).1.is_some_and(|failed| failed > 0)
}

pub fn test_counts(output: &str) -> (Option<u64>, Option<u64>) {
    static SUMMARY: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"(?m)(?:test result: (?:ok|FAILED)\. )?(\d+) passed[;,] (\d+) failed")
            .expect("static regex")
    });
    let mut passed = 0u64;
    let mut failed = 0u64;
    let mut seen = false;
    for captures in SUMMARY.captures_iter(output) {
        if let (Ok(p), Ok(f)) = (captures[1].parse::<u64>(), captures[2].parse::<u64>()) {
            passed = passed.saturating_add(p);
            failed = failed.saturating_add(f);
            seen = true;
        }
    }
    if seen {
        return (Some(passed), Some(failed));
    }
    // Pytest emits either count independently on its final summary line.
    static PYTEST: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"(\d+) (passed|failed|errors?)\b").expect("static regex"));
    if let Some(line) = output.lines().rev().find(|line| {
        line.contains(" in ")
            && (line.contains(" passed") || line.contains(" failed") || line.contains(" error"))
    }) {
        for captures in PYTEST.captures_iter(line) {
            if let Ok(count) = captures[1].parse::<u64>() {
                if &captures[2] == "passed" {
                    passed = passed.saturating_add(count);
                } else {
                    failed = failed.saturating_add(count);
                }
                seen = true;
            }
        }
    }
    if seen {
        (Some(passed), Some(failed))
    } else {
        (None, None)
    }
}
