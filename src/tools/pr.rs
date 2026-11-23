use std::{sync::LazyLock, time::Instant};

use anyhow::{bail, ensure, Context, Result};
use regex::Regex;
use reqwest::Client;
use rmcp::schemars;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{
    gmi::{
        agentbox::{shell_quote, AgentBox, ExecutionResult},
        maas::Maas,
        response_json,
    },
    receipt::Receipt,
    tools::{
        injection::{self, InjectionRequest, InjectionResult, ThreatLevel},
        sandbox,
        supply::{self, ManifestType, RiskScore, SupplyRequest, SupplyResult},
    },
};

const GITHUB_API_BASE: &str = "https://api.github.com";
const MAX_MANIFEST_FILES: usize = 10;
const MAX_PR_FILE_PAGES: usize = 3;
const MAX_PR_FILES: usize = MAX_PR_FILE_PAGES * 100;

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Deserialize, Serialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PrVerifyRequest {
    /// Full GitHub pull request URL, e.g. "<https://github.com/Nano-Collective/nanocoder/pull/2313>".
    pub pr_url: Option<String>,
    /// GitHub repository in "owner/repo" format (required with `pr_number` if `pr_url` is omitted).
    pub repo: Option<String>,
    /// Pull request number (required with `repo` if `pr_url` is omitted).
    pub pr_number: Option<u64>,
    /// Optional command to run inside the checked-out PR workspace (e.g. a targeted vitest or node -e check).
    /// When omitted, Airgap auto-detects and runs the repository's native test suite.
    pub test_command: Option<String>,
    /// Optional build command override run before `test_command`.
    pub build_command: Option<String>,
    /// Force running the project's build script before `test_command`. When omitted, auto-builds if
    /// `test_command` references built artifacts (such as `dist/`).
    #[serde(default)]
    pub build_first: Option<bool>,
    /// Optional container image override. Auto-detected from PR files/language when omitted.
    pub image: Option<String>,
    /// Execution timeout in seconds (1..=1800). Defaults to max(runtime_timeout, 300).
    pub timeout_secs: Option<u64>,
    /// Block sandbox execution if supply-chain audit flags high-risk manifest changes (default: true).
    #[serde(default = "default_true")]
    pub fail_on_supply_risk: bool,
    /// Allow package manager install/postinstall lifecycle scripts inside the sandbox (default: false).
    #[serde(default)]
    pub allow_install_scripts: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PrTarget {
    pub owner: String,
    pub repo: String,
    pub number: u64,
}

impl PrTarget {
    pub fn display_id(&self) -> String {
        format!("{}/{}#{}", self.owner, self.repo, self.number)
    }

    pub fn clone_url(&self) -> String {
        format!("https://github.com/{}/{}.git", self.owner, self.repo)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PrStatus {
    Verified,
    Failed,
    Blocked,
}

#[derive(Debug, Serialize)]
pub struct PrVerifyResult {
    pub status: PrStatus,
    pub verified: bool,
    pub pr: PrTarget,
    pub title: String,
    pub head_sha: String,
    pub base_ref: String,
    pub image: String,
    pub blocked_reason: Option<String>,
    pub injection_scan: InjectionResult,
    pub supply_audit: SupplyResult,
    pub audited_manifests: Vec<String>,
    pub command_passed: Option<bool>,
    pub tests_passed: Option<u64>,
    pub tests_failed: Option<u64>,
    pub execution: Option<ExecutionResult>,
    pub evidence_receipt: Receipt,
    pub duration_ms: u64,
}

struct ChangedFile {
    filename: String,
    removed: bool,
}

struct PrMetadata {
    title: String,
    body: String,
    head_sha: String,
    base_ref: String,
    language: Option<String>,
    changed_files: Vec<ChangedFile>,
    files_truncated: bool,
}

pub fn parse_pr_target(request: &PrVerifyRequest) -> Result<PrTarget> {
    let (owner, repo, number) = if let Some(url) = request.pr_url.as_deref() {
        static PR_URL: LazyLock<Regex> = LazyLock::new(|| {
            Regex::new(
                r"^https://github\.com/([A-Za-z0-9][A-Za-z0-9_.-]{0,99})/([A-Za-z0-9][A-Za-z0-9_.-]{0,99})/pull/(\d+)(?:/(?:files|commits|checks)?)?/?$",
            )
            .expect("static regex")
        });
        let captures = PR_URL.captures(url.trim()).context(
            "invalid GitHub PR URL; expected https://github.com/<owner>/<repo>/pull/<number>",
        )?;
        let number = captures[3].parse::<u64>().context("invalid PR number")?;
        (captures[1].to_owned(), captures[2].to_owned(), number)
    } else {
        let slug = request
            .repo
            .as_deref()
            .context("provide either `pr_url` or both `repo` (owner/name) and `pr_number`")?
            .trim();
        let number = request
            .pr_number
            .context("provide `pr_number` alongside `repo`")?;
        let (owner, repo) = slug
            .split_once('/')
            .context("`repo` must be in `owner/name` format")?;
        (owner.to_owned(), repo.to_owned(), number)
    };

    validate_slug_part(&owner, "owner")?;
    validate_slug_part(&repo, "repo")?;
    ensure!(number >= 1, "PR number must be >= 1");

    Ok(PrTarget {
        owner,
        repo,
        number,
    })
}

fn validate_slug_part(part: &str, label: &str) -> Result<()> {
    ensure!(
        !part.is_empty()
            && part.len() <= 100
            && part != "."
            && part != ".."
            && part
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.')),
        "invalid GitHub {label} identifier"
    );
    Ok(())
}

pub async fn verify_pr(
    http: &Client,
    github_token: Option<&str>,
    agentbox: &AgentBox,
    maas: &Maas,
    model: &str,
    default_timeout: u64,
    request: PrVerifyRequest,
) -> Result<PrVerifyResult> {
    verify_pr_with_api_base(
        http,
        GITHUB_API_BASE,
        github_token,
        agentbox,
        maas,
        model,
        default_timeout,
        request,
    )
    .await
}

#[allow(clippy::too_many_arguments)]
pub async fn verify_pr_with_api_base(
    http: &Client,
    api_base: &str,
    github_token: Option<&str>,
    agentbox: &AgentBox,
    maas: &Maas,
    model: &str,
    default_timeout: u64,
    request: PrVerifyRequest,
) -> Result<PrVerifyResult> {
    let started = Instant::now();
    let target = parse_pr_target(&request)?;

    if let Some(cmd) = request.test_command.as_deref() {
        ensure!(
            !cmd.trim().is_empty() && cmd.len() <= 64 * 1024 && !cmd.contains('\0'),
            "invalid test_command"
        );
    }
    if let Some(cmd) = request.build_command.as_deref() {
        ensure!(
            !cmd.trim().is_empty() && cmd.len() <= 64 * 1024 && !cmd.contains('\0'),
            "invalid build_command"
        );
    }

    let input_hash = crate::receipt::fingerprint(&request)?;
    let timeout = request
        .timeout_secs
        .unwrap_or_else(|| default_timeout.max(300));
    crate::config::validate_timeout(timeout)?;

    // Step 1: Fetch PR metadata and paginated changed file list.
    let meta = fetch_pr_metadata(http, api_base, github_token, &target).await?;
    let active_file_names: Vec<String> = meta
        .changed_files
        .iter()
        .filter(|f| !f.removed)
        .map(|f| f.filename.clone())
        .collect();
    let image = request
        .image
        .clone()
        .unwrap_or_else(|| detect_pr_image(&active_file_names, meta.language.as_deref(), agentbox));

    // Step 2: Run prompt-injection classification and manifest supply-chain audit in parallel.
    let pr_text = format!("PR #{}: {}\n\n{}", target.number, meta.title, meta.body);
    let injection_fut = injection::scan(
        maas,
        model,
        timeout.min(120),
        InjectionRequest {
            text_content: pr_text,
            source_label: Some(format!("github-pr:{}", target.display_id())),
        },
    );
    let supply_fut = audit_pr_manifests(
        http,
        api_base,
        github_token,
        &target,
        &meta.head_sha,
        &meta.changed_files,
        meta.files_truncated,
    );
    let (injection_scan, (supply_audit, audited_manifests)) =
        tokio::try_join!(injection_fut, supply_fut)?;

    // Step 3: Fail-closed security gates before provisioning compute.
    let blocked_reason = if injection_scan.threat_level == ThreatLevel::Malicious {
        Some("Prompt injection classifier flagged PR description as malicious.".into())
    } else if request.fail_on_supply_risk && matches!(supply_audit.risk_score, RiskScore::High) {
        Some(format!(
            "Supply-chain audit flagged {} high-risk manifest item(s). Review before running.",
            supply_audit.flagged_scripts.len()
        ))
    } else {
        None
    };

    if let Some(reason) = blocked_reason {
        let duration_ms = started.elapsed().as_millis() as u64;
        let receipt = Receipt::from_input_hash(
            input_hash,
            &serde_json::json!({
                "status": "blocked",
                "reason": reason,
                "head_sha": meta.head_sha,
                "injection": injection_scan,
                "supply": supply_audit,
            }),
            duration_ms,
            None,
        )?;
        return Ok(PrVerifyResult {
            status: PrStatus::Blocked,
            verified: false,
            pr: target,
            title: meta.title,
            head_sha: meta.head_sha,
            base_ref: meta.base_ref,
            image,
            blocked_reason: Some(reason),
            injection_scan,
            supply_audit,
            audited_manifests,
            command_passed: None,
            tests_passed: None,
            tests_failed: None,
            execution: None,
            evidence_receipt: receipt,
            duration_ms,
        });
    }

    // Step 4: Native shallow git checkout + cached install + auto-build + test inside microVM.
    let script = build_pr_script(
        &target,
        &meta.head_sha,
        &active_file_names,
        request.test_command.as_deref(),
        request.build_command.as_deref(),
        request.build_first,
        request.allow_install_scripts,
        agentbox.cache_base_url(),
    );
    let execution = agentbox.run_script(script, image.clone(), timeout).await?;

    let combined = format!("{}\n{}", execution.stdout, execution.stderr);
    let (tests_passed, tests_failed) = sandbox::test_counts(&combined);
    let verified = if request.test_command.is_some() {
        // Custom repro/verification commands (e.g. `node -e ...`) may not emit a test runner summary;
        // still reject failed test counts, OOM kills, or truncated outputs.
        execution.passed
            && !execution.stdout_truncated
            && !execution.stderr_truncated
            && !tests_failed.is_some_and(|n| n > 0)
    } else {
        execution.passed
            && !execution.stdout_truncated
            && !execution.stderr_truncated
            && tests_passed.is_some_and(|n| n > 0)
            && tests_failed == Some(0)
    };
    let status = if verified {
        PrStatus::Verified
    } else {
        PrStatus::Failed
    };
    let duration_ms = started.elapsed().as_millis() as u64;
    let receipt = Receipt::from_input_hash(
        input_hash,
        &serde_json::json!({
            "status": status,
            "head_sha": meta.head_sha,
            "execution": execution,
            "tests_passed": tests_passed,
            "tests_failed": tests_failed,
        }),
        duration_ms,
        None,
    )?;

    Ok(PrVerifyResult {
        status,
        verified,
        pr: target,
        title: meta.title,
        head_sha: meta.head_sha,
        base_ref: meta.base_ref,
        image,
        blocked_reason: None,
        injection_scan,
        supply_audit,
        audited_manifests,
        command_passed: Some(execution.passed),
        tests_passed,
        tests_failed,
        execution: Some(execution),
        evidence_receipt: receipt,
        duration_ms,
    })
}

fn check_github_status(response: &reqwest::Response, context: &str) -> Result<()> {
    let status = response.status();
    if status.is_success() {
        return Ok(());
    }
    let remaining = response
        .headers()
        .get("x-ratelimit-remaining")
        .and_then(|v| v.to_str().ok());
    if status.as_u16() == 429 || (status.as_u16() == 403 && remaining == Some("0")) {
        bail!(
            "GitHub API rate limit exceeded. Set GITHUB_TOKEN or GH_TOKEN to raise limits to 5,000 req/hr."
        );
    }
    bail!("{context} returned HTTP {}", status.as_u16());
}

async fn github_json(http: &Client, url: &str, github_token: Option<&str>) -> Result<Value> {
    let mut req = http
        .get(url)
        .header("Accept", "application/vnd.github+json")
        .header("X-GitHub-Api-Version", "2022-11-28");
    if let Some(token) = github_token {
        req = req.bearer_auth(token);
    }
    let response = req
        .send()
        .await
        .map_err(|_| anyhow::anyhow!("GitHub API network request failed"))?;
    check_github_status(&response, "GitHub API request")?;
    response_json(response)
        .await
        .map_err(|e| anyhow::anyhow!("GitHub API error: {e}"))
}

async fn github_raw_file(http: &Client, url: &str, github_token: Option<&str>) -> Result<String> {
    let mut req = http
        .get(url)
        .header("Accept", "application/vnd.github.raw+json")
        .header("X-GitHub-Api-Version", "2022-11-28");
    if let Some(token) = github_token {
        req = req.bearer_auth(token);
    }
    let mut response = req
        .send()
        .await
        .map_err(|_| anyhow::anyhow!("GitHub manifest fetch failed"))?;
    check_github_status(&response, "GitHub manifest fetch")?;
    let mut bytes = Vec::new();
    while let Some(chunk) = response
        .chunk()
        .await
        .context("GitHub response interrupted")?
    {
        ensure!(
            bytes.len() + chunk.len() <= crate::MAX_TEXT_BYTES,
            "manifest exceeds 2 MiB limit"
        );
        bytes.extend_from_slice(&chunk);
    }
    String::from_utf8(bytes).context("manifest is not valid UTF-8")
}

async fn fetch_pr_metadata(
    http: &Client,
    api_base: &str,
    github_token: Option<&str>,
    target: &PrTarget,
) -> Result<PrMetadata> {
    let pr_url = format!(
        "{api_base}/repos/{}/{}/pulls/{}",
        target.owner, target.repo, target.number
    );
    let first_files_url = format!("{pr_url}/files?per_page=100&page=1");
    let (pr_json, first_page) = tokio::try_join!(
        github_json(http, &pr_url, github_token),
        github_json(http, &first_files_url, github_token)
    )?;

    let title = pr_json["title"].as_str().unwrap_or("").to_owned();
    let body = pr_json["body"].as_str().unwrap_or("").to_owned();
    let head_sha = pr_json
        .pointer("/head/sha")
        .and_then(Value::as_str)
        .context("GitHub PR response missing head.sha")?
        .to_owned();
    ensure!(
        head_sha.len() == 40 && head_sha.bytes().all(|b| b.is_ascii_hexdigit()),
        "GitHub PR response contained invalid head SHA"
    );
    let base_ref = pr_json
        .pointer("/base/ref")
        .and_then(Value::as_str)
        .unwrap_or("main")
        .to_owned();
    let language = pr_json
        .pointer("/base/repo/language")
        .and_then(Value::as_str)
        .map(str::to_owned);
    let reported_changed_files = pr_json["changed_files"].as_u64();

    let mut changed_files = Vec::new();
    let mut files_truncated = reported_changed_files.is_some_and(|n| n > MAX_PR_FILES as u64);

    let parse_page = |page_json: &Value, out: &mut Vec<ChangedFile>| -> Result<usize> {
        let items = page_json
            .as_array()
            .context("GitHub PR files response was not an array")?;
        for item in items {
            if let Some(filename) = item.get("filename").and_then(Value::as_str) {
                let removed = item.get("status").and_then(Value::as_str) == Some("removed");
                out.push(ChangedFile {
                    filename: filename.to_owned(),
                    removed,
                });
            }
        }
        Ok(items.len())
    };

    let mut last_count = parse_page(&first_page, &mut changed_files)?;
    for page in 2..=MAX_PR_FILE_PAGES {
        if last_count < 100 {
            break;
        }
        let files_url = format!("{pr_url}/files?per_page=100&page={page}");
        let page_json = github_json(http, &files_url, github_token).await?;
        last_count = parse_page(&page_json, &mut changed_files)?;
    }
    if last_count == 100
        && changed_files.len() >= MAX_PR_FILES
        && reported_changed_files.is_none_or(|n| n > MAX_PR_FILES as u64)
    {
        files_truncated = true;
    }

    Ok(PrMetadata {
        title,
        body,
        head_sha,
        base_ref,
        language,
        changed_files,
        files_truncated,
    })
}

fn manifest_kind(path: &str) -> Option<ManifestType> {
    let file_name = path.rsplit('/').next().unwrap_or(path);
    match file_name {
        "package.json" => Some(ManifestType::Npm),
        "Cargo.toml" => Some(ManifestType::Cargo),
        name if name == "requirements.txt"
            || (name.starts_with("requirements-") && name.ends_with(".txt")) =>
        {
            Some(ManifestType::Python)
        }
        _ => None,
    }
}

fn is_high_risk_hook_file(path: &str) -> bool {
    let file_name = path.rsplit('/').next().unwrap_or(path);
    matches!(
        file_name,
        ".pnpmfile.cjs"
            | ".pnpmfile.js"
            | "pnpmfile.js"
            | ".yarnrc.yml"
            | ".yarnrc"
            | ".npmrc"
            | "setup.py"
            | "build.rs"
    ) || path == ".cargo/config.toml"
        || path.ends_with("/.cargo/config.toml")
        || path == ".cargo/config"
        || path.ends_with("/.cargo/config")
}

async fn audit_pr_manifests(
    http: &Client,
    api_base: &str,
    github_token: Option<&str>,
    target: &PrTarget,
    head_sha: &str,
    changed_files: &[ChangedFile],
    files_truncated: bool,
) -> Result<(SupplyResult, Vec<String>)> {
    let mut flagged = Vec::new();
    let mut audited = Vec::new();

    if files_truncated {
        flagged.push(format!(
            "PR modifies more than {MAX_PR_FILES} files; supply-chain audit cannot inspect all changed files (fail-closed)."
        ));
    }

    for file in changed_files.iter().filter(|f| !f.removed) {
        if is_high_risk_hook_file(&file.filename) {
            audited.push(file.filename.clone());
            flagged.push(format!(
                "{}: high-risk package-manager or build hook file requires review",
                file.filename
            ));
        }
    }

    let active_manifests: Vec<(&str, ManifestType)> = changed_files
        .iter()
        .filter(|f| !f.removed)
        .filter_map(|f| manifest_kind(&f.filename).map(|kind| (f.filename.as_str(), kind)))
        .collect();

    if active_manifests.len() > MAX_MANIFEST_FILES {
        flagged.push(format!(
            "PR modifies {} manifests (limit {MAX_MANIFEST_FILES}); failing closed because uninspected manifests cannot be verified.",
            active_manifests.len()
        ));
    }

    for (path, kind) in active_manifests.into_iter().take(MAX_MANIFEST_FILES) {
        let mut url = reqwest::Url::parse(&format!(
            "{api_base}/repos/{}/{}/contents",
            target.owner, target.repo
        ))
        .context("invalid GitHub API base URL")?;
        let mut bad_segment = false;
        {
            let mut segments = url
                .path_segments_mut()
                .map_err(|_| anyhow::anyhow!("GitHub API URL cannot be a base"))?;
            for part in path.split('/') {
                if part.is_empty() || part == "." || part == ".." || part.contains('\\') {
                    bad_segment = true;
                    break;
                }
                segments.push(part);
            }
        }
        if bad_segment {
            flagged.push(format!(
                "{path}: suspicious manifest path segment rejected (fail-closed)"
            ));
            continue;
        }
        url.query_pairs_mut().append_pair("ref", head_sha);

        // Fail closed on any fetch/size/encoding/rate-limit error; removed files are already excluded.
        let content = github_raw_file(http, url.as_str(), github_token)
            .await
            .with_context(|| format!("supply-chain audit failed to fetch manifest `{path}`"))?;
        let res = supply::audit(SupplyRequest {
            manifest_type: kind,
            manifest_content: content,
        })?;
        audited.push(path.to_owned());
        for flag in res.flagged_scripts {
            flagged.push(format!("{path}: {flag}"));
        }
    }

    Ok((
        SupplyResult {
            risk_score: if flagged.is_empty() {
                RiskScore::Low
            } else {
                RiskScore::High
            },
            flagged_scripts: flagged,
            limitations: "PR diff manifest heuristics only; low does not mean safe. Transitive dependencies and advisory databases are not scanned.".into(),
        },
        audited,
    ))
}

fn detect_pr_image(
    changed_files: &[String],
    language: Option<&str>,
    agentbox: &AgentBox,
) -> String {
    if changed_files
        .iter()
        .any(|p| p.ends_with("Cargo.toml") || p.ends_with(".rs"))
    {
        return "docker.io/library/rust:1-bookworm".into();
    }
    if changed_files.iter().any(|p| {
        p.ends_with("package.json")
            || p.ends_with("pnpm-lock.yaml")
            || p.ends_with(".ts")
            || p.ends_with(".tsx")
            || p.ends_with(".js")
            || p.ends_with(".jsx")
            || p.ends_with(".mjs")
            || p.ends_with(".cjs")
    }) {
        return agentbox.default_node_image().to_owned();
    }
    if changed_files
        .iter()
        .any(|p| p.ends_with(".py") || p.ends_with("pyproject.toml"))
    {
        return "docker.io/library/python:3.12-bookworm".into();
    }
    match language.map(|l| l.to_ascii_lowercase()).as_deref() {
        Some("rust") => "docker.io/library/rust:1-bookworm".into(),
        Some("typescript" | "javascript") => agentbox.default_node_image().to_owned(),
        _ => "docker.io/library/python:3.12-bookworm".into(),
    }
}

#[allow(clippy::too_many_arguments)]
pub fn build_pr_script(
    target: &PrTarget,
    head_sha: &str,
    changed_files: &[String],
    test_command: Option<&str>,
    build_command: Option<&str>,
    build_first: Option<bool>,
    allow_install_scripts: bool,
    cache_base_url: Option<&str>,
) -> String {
    let clone_url = shell_quote(&target.clone_url());
    let fetch_ref = shell_quote(&format!("pull/{}/head:pr-verify", target.number));
    let expected_sha = shell_quote(head_sha);
    let ignore_scripts_flag = if allow_install_scripts {
        ""
    } else {
        " --ignore-scripts"
    };
    let pnpm_hardening_flags = if allow_install_scripts {
        ""
    } else {
        " --ignore-scripts --ignore-pnpmfile"
    };
    let yarn_scripts_env = if allow_install_scripts {
        ""
    } else {
        "export YARN_ENABLE_SCRIPTS=false\n    "
    };

    let needs_build_for_tests = changed_files
        .iter()
        .any(|p| is_js_test_file(p) && (p.contains("integration") || p.contains("cli")));

    let auto_build_trigger = match build_first {
        Some(true) => "always",
        Some(false) => "never",
        None => match test_command {
            Some(cmd)
                if cmd.contains("dist/")
                    || cmd.contains("./dist")
                    || cmd.contains("build/")
                    || cmd.contains("node -e") =>
            {
                "if_script_exists"
            }
            None if needs_build_for_tests => "if_script_exists",
            _ => "never",
        },
    };

    let cache_hydrate = match cache_base_url {
        Some(base) => format!(
            r#"if [ -f pnpm-lock.yaml ] && command -v curl >/dev/null 2>&1 && command -v zstd >/dev/null 2>&1; then
  LOCK_HASH="$(sha256sum pnpm-lock.yaml | awk '{{print $1}}')"
  if [ -n "$LOCK_HASH" ] && curl -fsSL --max-time 20 {base_q}"/pnpm-${{LOCK_HASH}}.tar.zst" -o /root/tmp/pnpm-cache.tar.zst 2>/dev/null; then
    mkdir -p "$PNPM_STORE_DIR"
    tar --zstd --no-same-owner -xf /root/tmp/pnpm-cache.tar.zst -C "$PNPM_STORE_DIR" 2>/dev/null || true
    rm -f /root/tmp/pnpm-cache.tar.zst
  fi
fi"#,
            base_q = shell_quote(base)
        ),
        None => String::new(),
    };

    let build_step = if let Some(cmd) = build_command {
        format!(
            r#"if ! sh -c {} >/root/tmp/airgap-build.log 2>&1; then
  tail -n 60 /root/tmp/airgap-build.log >&2
  exit 1
fi"#,
            shell_quote(cmd)
        )
    } else if auto_build_trigger != "never" {
        r#"if [ -f package.json ] && node -e 'const p=require("./package.json"); process.exit(p.scripts && p.scripts.build ? 0 : 1)'; then
  if ! "$PM" run build >/root/tmp/airgap-build.log 2>&1; then
    tail -n 60 /root/tmp/airgap-build.log >&2
    exit 1
  fi
fi"#
        .to_owned()
    } else {
        String::new()
    };

    let run_step = if let Some(cmd) = test_command {
        format!("exec sh -c {}", shell_quote(cmd))
    } else {
        let targets = resolve_test_targets(changed_files);
        let full_suite_note = match targets.full_suite_reason.as_deref() {
            Some(reason) => format!(
                "echo {} >&2\n",
                shell_quote(&format!("AIRGAP_INFO: running full test suite: {reason}"))
            ),
            None => String::new(),
        };
        let coverage_check = sibling_coverage_check(&targets);
        let js_block = js_targeted_block(&targets);
        let py_block = py_targeted_block(&targets);
        let cargo_block = cargo_targeted_block(&targets);
        format!(
            r#"{full_suite_note}{coverage_check}if [ -f package.json ]; then
{js_block}  TEST_SCRIPT="$(node -e 'const p=require("./package.json"); const s=p.scripts||{{}}; for (const k of ["test","test:unit","test:vitest","test:jest","test:ava","test:ci","test:all"]) {{ if (s[k]) {{ console.log(k); process.exit(0); }} }}')"
  if [ -n "$TEST_SCRIPT" ]; then
    exec "$PM" run "$TEST_SCRIPT"
  fi
  echo "AIRGAP_ERROR: package.json has no recognized test script; pass test_command explicitly" >&2
  exit 2
elif [ -f Cargo.toml ]; then
{cargo_block}  exec cargo test
elif [ -f pyproject.toml ] || [ -f pytest.ini ] || [ -d tests ]; then
{py_block}  exec pytest
else
  echo "AIRGAP_ERROR: no recognized test suite; pass test_command explicitly" >&2
  exit 2
fi"#
        )
    };
    format!(
        r#"#!/bin/sh
set -eu

export TMPDIR=/root/tmp
export CI=true
export NO_UPDATE_NOTIFIER=1
export COREPACK_ENABLE_DOWNLOAD_PROMPT=0
export PNPM_HOME=/root/.local/share/pnpm
export PATH="$PNPM_HOME:$PATH"
export NODE_OPTIONS="${{NODE_OPTIONS:---max-old-space-size=3072}}"

if [ -d /opt/airgap/pnpm-store ]; then
  export PNPM_STORE_DIR=/opt/airgap/pnpm-store
else
  export PNPM_STORE_DIR=/root/.local/share/pnpm/store
fi

mkdir -p /workspace /root/tmp
cd /workspace

git init -q
git config core.hooksPath /dev/null
git config core.fsmonitor false
git config submodule.recurse false
git config protocol.file.allow never
git remote add origin {clone_url}
git fetch -q --depth 1 --no-tags --filter=blob:none origin {fetch_ref}

EXPECTED_SHA={expected_sha}
ACTUAL_SHA="$(git rev-parse pr-verify)"
if [ "$ACTUAL_SHA" != "$EXPECTED_SHA" ]; then
  echo "AIRGAP_ERROR: PR HEAD changed after security audit ($ACTUAL_SHA != $EXPECTED_SHA)" >&2
  exit 86
fi
git checkout -q --detach "$ACTUAL_SHA"

{cache_hydrate}

PM="npm"
if [ -f package.json ]; then
  if [ -f pnpm-lock.yaml ]; then
    PM="pnpm"
    if ! command -v pnpm >/dev/null 2>&1; then
      corepack enable >/dev/null 2>&1 || true
    fi
    if ! pnpm install --frozen-lockfile --prefer-offline{pnpm_hardening_flags} --store-dir "$PNPM_STORE_DIR" --child-concurrency=4 >/root/tmp/airgap-install.log 2>&1; then
      tail -n 60 /root/tmp/airgap-install.log >&2
      exit 1
    fi
  elif [ -f yarn.lock ]; then
    PM="yarn"
    if ! command -v yarn >/dev/null 2>&1; then
      corepack enable >/dev/null 2>&1 || true
    fi
    {yarn_scripts_env}if ! yarn install --frozen-lockfile --prefer-offline{ignore_scripts_flag} >/root/tmp/airgap-install.log 2>&1; then
      tail -n 60 /root/tmp/airgap-install.log >&2
      exit 1
    fi
  elif [ -f package-lock.json ]; then
    PM="npm"
    if ! npm ci --prefer-offline --no-audit --no-fund{ignore_scripts_flag} >/root/tmp/airgap-install.log 2>&1; then
      tail -n 60 /root/tmp/airgap-install.log >&2
      exit 1
    fi
  else
    if ! npm install --prefer-offline --no-audit --no-fund{ignore_scripts_flag} >/root/tmp/airgap-install.log 2>&1; then
      tail -n 60 /root/tmp/airgap-install.log >&2
      exit 1
    fi
  fi
fi

{build_step}
{run_step}
"#
    )
}

const MAX_TARGETED_TESTS: usize = 20;
const MAX_PAIRED_SOURCES: usize = 20;
const JS_TS_EXTS: [&str; 2] = ["ts", "tsx"];
const JS_EXTS: [&str; 4] = ["js", "jsx", "mjs", "cjs"];

/// A changed source file and the sibling test paths that would cover it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourcePairing {
    pub source: String,
    pub candidates: Vec<String>,
}

/// Tests selected from a PR's changed files. When `full_suite_reason` is set, every list is empty
/// and the repository's full test script runs instead.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct TestTargets {
    pub full_suite_reason: Option<String>,
    /// Changed JS/TS test files.
    pub js_tests: Vec<String>,
    /// Changed JS/TS sources (and snapshot files) with their sibling `*.spec.*` / `*.test.*` paths.
    pub js_sources: Vec<SourcePairing>,
    /// Changed pytest files.
    pub py_tests: Vec<String>,
    /// Changed Python sources with their `test_<name>.py` / `<name>_test.py` paths.
    pub py_sources: Vec<SourcePairing>,
    /// Integration test target names from changed root `tests/<name>.rs` files.
    pub cargo_integration_tests: Vec<String>,
    /// A Rust file outside root `tests/` changed, so `cargo test` must run in full.
    pub rust_sources_changed: bool,
}

impl TestTargets {
    pub fn is_full_suite(&self) -> bool {
        self.full_suite_reason.is_some()
    }

    /// Sibling paths to run if they exist, excluding tests the PR already changed.
    pub fn js_sibling_candidates(&self) -> Vec<&String> {
        sibling_candidates(&self.js_sources, &self.js_tests)
    }

    pub fn py_sibling_candidates(&self) -> Vec<&String> {
        sibling_candidates(&self.py_sources, &self.py_tests)
    }

    fn full_suite(reason: String) -> Self {
        Self {
            full_suite_reason: Some(reason),
            ..Self::default()
        }
    }
}

fn sibling_candidates<'a>(sources: &'a [SourcePairing], direct: &[String]) -> Vec<&'a String> {
    let mut out: Vec<&String> = Vec::new();
    for candidate in sources.iter().flat_map(|s| s.candidates.iter()) {
        if !direct.contains(candidate) && !out.contains(&candidate) {
            out.push(candidate);
        }
    }
    out
}

/// Maps changed files to targeted tests. Changed test files run directly and every changed source
/// must have a sibling test; the sandbox script runs the full suite if any source's siblings are
/// missing. Shared config, lockfile, test-support, or unrecognized file changes force the full suite
/// up front, and any Rust change outside root `tests/` forces a full `cargo test`.
pub fn resolve_test_targets(changed_files: &[String]) -> TestTargets {
    if changed_files.len() >= MAX_PR_FILES {
        return TestTargets::full_suite(format!(
            "PR touches {MAX_PR_FILES}+ files; changed file list may be incomplete"
        ));
    }

    let mut targets = TestTargets::default();
    for path in changed_files {
        if !is_safe_target_path(path) {
            continue;
        }
        if is_suite_config_file(path) {
            return TestTargets::full_suite(format!(
                "{path} changes shared build/test configuration"
            ));
        }
        let (dir, file_name) = match path.rsplit_once('/') {
            Some((dir, name)) => (Some(dir), name),
            None => (None, path.as_str()),
        };

        if let Some(test_file) = snapshot_owner(dir, file_name) {
            targets.js_sources.push(SourcePairing {
                source: path.clone(),
                candidates: vec![test_file],
            });
        } else if is_js_test_file(path) {
            push_unique(&mut targets.js_tests, path);
        } else if is_py_test_file(file_name) {
            push_unique(&mut targets.py_tests, path);
        } else if let Some(name) = cargo_integration_test_name(path) {
            push_unique(&mut targets.cargo_integration_tests, &name);
        } else if in_test_directory(path) {
            if js_ext(file_name).is_some() && !is_test_support_path(path) {
                push_unique(&mut targets.js_tests, path);
            } else {
                // Helpers, fixtures, snapshots, conftest-style modules, or nested Rust test modules
                // can affect any test, so targeting would under-test.
                return TestTargets::full_suite(format!("{path} is shared test support code"));
            }
        } else if let Some((stem, ext)) = js_source_stem(file_name) {
            let family: &[&str] = if JS_TS_EXTS.contains(&ext) {
                &JS_TS_EXTS
            } else {
                &JS_EXTS
            };
            let mut candidates = Vec::new();
            for kind in ["spec", "test"] {
                for candidate_ext in family {
                    candidates.push(join_path(dir, &format!("{stem}.{kind}.{candidate_ext}")));
                }
            }
            targets.js_sources.push(SourcePairing {
                source: path.clone(),
                candidates,
            });
        } else if let Some(stem) = file_name
            .strip_suffix(".py")
            .filter(|s| !s.is_empty() && *s != "__init__")
        {
            let mut candidates = vec![
                join_path(dir, &format!("test_{stem}.py")),
                join_path(dir, &format!("{stem}_test.py")),
            ];
            push_unique(&mut candidates, &format!("tests/test_{stem}.py"));
            targets.py_sources.push(SourcePairing {
                source: path.clone(),
                candidates,
            });
        } else if file_name.ends_with(".rs") {
            targets.rust_sources_changed = true;
        } else if !is_non_code_file(path) {
            return TestTargets::full_suite(format!("{path} has no sibling test pairing rule"));
        }
    }

    let direct =
        targets.js_tests.len() + targets.py_tests.len() + targets.cargo_integration_tests.len();
    if direct > MAX_TARGETED_TESTS {
        return TestTargets::full_suite(format!(
            "PR changes {direct} test files (targeting limit {MAX_TARGETED_TESTS})"
        ));
    }
    let paired_sources = targets.js_sources.len() + targets.py_sources.len();
    if paired_sources > MAX_PAIRED_SOURCES {
        return TestTargets::full_suite(format!(
            "PR changes {paired_sources} source files (pairing limit {MAX_PAIRED_SOURCES})"
        ));
    }
    if targets.rust_sources_changed {
        targets.cargo_integration_tests.clear();
    }
    targets
}

/// Docs and static assets that no test suite executes.
fn is_non_code_file(path: &str) -> bool {
    if path.starts_with("docs/")
        || (path.starts_with(".github/") && !path.ends_with(".yml") && !path.ends_with(".yaml"))
    {
        return true;
    }
    let file_name = path.rsplit('/').next().unwrap_or(path);
    if matches!(
        file_name,
        "LICENSE"
            | "LICENCE"
            | "NOTICE"
            | "AUTHORS"
            | "CODEOWNERS"
            | ".gitignore"
            | ".gitattributes"
            | ".editorconfig"
    ) || file_name.starts_with("LICENSE.")
    {
        return true;
    }
    let ext = file_name
        .rsplit_once('.')
        .map(|(_, e)| e.to_ascii_lowercase());
    matches!(
        ext.as_deref(),
        Some(
            "md" | "mdx"
                | "markdown"
                | "rst"
                | "adoc"
                | "txt"
                | "png"
                | "jpg"
                | "jpeg"
                | "gif"
                | "svg"
                | "webp"
                | "ico"
        )
    )
}
fn push_unique(list: &mut Vec<String>, value: &str) {
    if !list.iter().any(|v| v == value) {
        list.push(value.to_owned());
    }
}

fn join_path(dir: Option<&str>, name: &str) -> String {
    match dir {
        Some(dir) => format!("{dir}/{name}"),
        None => name.to_owned(),
    }
}

/// Maps `dir/__snapshots__/a.spec.ts.snap` to the test that owns it, `dir/a.spec.ts`.
fn snapshot_owner(dir: Option<&str>, file_name: &str) -> Option<String> {
    let test_name = file_name.strip_suffix(".snap")?;
    let parent = match dir? {
        "__snapshots__" => None,
        d => Some(d.strip_suffix("/__snapshots__")?),
    };
    let owner = join_path(parent, test_name);
    is_js_test_file(&owner).then_some(owner)
}

fn is_safe_target_path(path: &str) -> bool {
    !path.is_empty()
        && !path.starts_with('-')
        && !path.starts_with('/')
        && !path.contains("..")
        && !path.contains(['\\', ':', '\n', '\r', '\0'])
}

fn is_suite_config_file(path: &str) -> bool {
    if path.starts_with(".github/workflows/") || path.starts_with(".github/actions/") {
        return true;
    }
    if path == ".cargo/config.toml"
        || path == ".cargo/config"
        || path.ends_with("/.cargo/config.toml")
        || path.ends_with("/.cargo/config")
    {
        return true;
    }
    let file_name = path.rsplit('/').next().unwrap_or(path);
    matches!(
        file_name,
        "package.json"
            | "package-lock.json"
            | "npm-shrinkwrap.json"
            | "pnpm-lock.yaml"
            | "pnpm-workspace.yaml"
            | "yarn.lock"
            | "bun.lockb"
            | "bun.lock"
            | ".nvmrc"
            | ".node-version"
            | ".babelrc"
            | ".swcrc"
            | "jsconfig.json"
            | "Cargo.toml"
            | "Cargo.lock"
            | "build.rs"
            | "rust-toolchain"
            | "rust-toolchain.toml"
            | "pyproject.toml"
            | "setup.py"
            | "setup.cfg"
            | "tox.ini"
            | "pytest.ini"
            | "conftest.py"
    ) || (file_name.starts_with("tsconfig") && file_name.ends_with(".json"))
        || (file_name.starts_with("requirements") && file_name.ends_with(".txt"))
        || [
            "vitest.config.",
            "vitest.workspace.",
            "vitest.setup.",
            "jest.config.",
            "jest.setup.",
            "ava.config.",
            "babel.config.",
        ]
        .iter()
        .any(|prefix| file_name.starts_with(prefix))
}

fn js_ext(file_name: &str) -> Option<&'static str> {
    let (_, ext) = file_name.rsplit_once('.')?;
    JS_TS_EXTS
        .iter()
        .chain(JS_EXTS.iter())
        .copied()
        .find(|e| *e == ext)
}

fn js_source_stem(file_name: &str) -> Option<(&str, &'static str)> {
    // Declaration files have no sibling tests; leaving them unpaired forces the full suite.
    if file_name.contains(".d.") {
        return None;
    }
    let ext = js_ext(file_name)?;
    let stem = &file_name[..file_name.len() - ext.len() - 1];
    (!stem.is_empty()).then_some((stem, ext))
}

fn is_py_test_file(file_name: &str) -> bool {
    file_name.ends_with(".py")
        && ((file_name.starts_with("test_") && file_name.len() > "test_.py".len())
            || (file_name.ends_with("_test.py") && file_name.len() > "_test.py".len()))
}

fn cargo_integration_test_name(path: &str) -> Option<String> {
    let name = path.strip_prefix("tests/")?.strip_suffix(".rs")?;
    (!name.is_empty()
        && !name.starts_with('-')
        && name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'-')))
    .then(|| name.to_owned())
}

fn in_test_directory(path: &str) -> bool {
    let mut segments: Vec<&str> = path.split('/').collect();
    segments.pop();
    segments
        .iter()
        .any(|s| matches!(*s, "tests" | "test" | "__tests__"))
}

fn is_test_support_path(path: &str) -> bool {
    let file_name = path.rsplit('/').next().unwrap_or(path);
    file_name.starts_with('_')
        || path.split('/').any(|s| {
            matches!(
                s,
                "helpers"
                    | "helper"
                    | "fixtures"
                    | "__fixtures__"
                    | "__mocks__"
                    | "__snapshots__"
                    | "support"
                    | "utils"
                    | "setup"
            )
        })
}

/// Emits `set -- ...` lines that collect paths that exist in the checkout into `"$@"`.
fn positional_file_list(paths: &[&String], indent: &str) -> String {
    let mut out = format!("{indent}set --\n");
    for path in paths {
        let q = shell_quote(path);
        out.push_str(&format!(
            "{indent}if [ -f {q} ]; then set -- \"$@\" {q}; fi\n"
        ));
    }
    out
}

/// Sets `TARGET_FULL=1` when any changed source has none of its sibling tests in the checkout,
/// so untested source changes never pass on a targeted run.
fn sibling_coverage_check(targets: &TestTargets) -> String {
    let mut out = String::from("TARGET_FULL=0\n");
    for pairing in targets.js_sources.iter().chain(targets.py_sources.iter()) {
        let missing = pairing
            .candidates
            .iter()
            .map(|c| format!("[ ! -f {} ]", shell_quote(c)))
            .collect::<Vec<_>>()
            .join(" && ");
        let note = shell_quote(&format!(
            "AIRGAP_INFO: running full test suite: no sibling test found for {}",
            pairing.source
        ));
        out.push_str(&format!(
            "if {missing}; then echo {note} >&2; TARGET_FULL=1; fi\n"
        ));
    }
    out
}

fn js_targeted_block(targets: &TestTargets) -> String {
    let paths: Vec<&String> = targets
        .js_tests
        .iter()
        .chain(targets.js_sibling_candidates())
        .collect();
    if paths.is_empty() {
        return String::new();
    }
    let list = positional_file_list(&paths, "    ");
    // Jest and `node --test` take no `--` separator, so paths get a `./` prefix to keep them from
    // ever being parsed as flags.
    format!(
        r#"  if [ "$TARGET_FULL" = 0 ]; then
{list}    if [ "$#" -gt 0 ]; then
      RUNNER="$(node -e 'const p=require("./package.json"); const s=p.scripts||{{}}; const d=Object.assign({{}},p.devDependencies||{{}},p.dependencies||{{}}); const vals=Object.values(s).join(" "); if(d.ava||/\bava\b/.test(vals)) console.log("ava"); else if(d.vitest||/\bvitest\b/.test(vals)) console.log("vitest"); else if(d.jest||/\bjest\b/.test(vals)) console.log("jest"); else if(/\bnode\s+--test\b/.test(vals)) console.log("node");')"
      case "$RUNNER" in
        ava|vitest|jest|node) echo "AIRGAP_INFO: targeted $RUNNER run over $# file(s)" >&2 ;;
      esac
      if [ "$RUNNER" = "ava" ]; then
        exec "$PM" exec ava -- "$@"
      elif [ "$RUNNER" = "vitest" ]; then
        exec "$PM" exec vitest run -- "$@"
      elif [ "$RUNNER" = "jest" ]; then
        for f in "$@"; do shift; set -- "$@" "./$f"; done
        exec "$PM" exec jest --runTestsByPath "$@"
      elif [ "$RUNNER" = "node" ]; then
        for f in "$@"; do shift; set -- "$@" "./$f"; done
        exec node --test "$@"
      fi
    fi
  fi
"#
    )
}

fn py_targeted_block(targets: &TestTargets) -> String {
    let paths: Vec<&String> = targets
        .py_tests
        .iter()
        .chain(targets.py_sibling_candidates())
        .collect();
    if paths.is_empty() {
        return String::new();
    }
    let list = positional_file_list(&paths, "    ");
    format!(
        r#"  if [ "$TARGET_FULL" = 0 ]; then
{list}    if [ "$#" -gt 0 ]; then
      echo "AIRGAP_INFO: targeted pytest run over $# file(s)" >&2
      exec pytest -- "$@"
    fi
  fi
"#
    )
}

fn cargo_targeted_block(targets: &TestTargets) -> String {
    if targets.cargo_integration_tests.is_empty() {
        return String::new();
    }
    let mut out = String::from("  if [ \"$TARGET_FULL\" = 0 ]; then\n    set --\n");
    for name in &targets.cargo_integration_tests {
        out.push_str(&format!(
            "    if [ -f {file} ]; then set -- \"$@\" --test {name}; fi\n",
            file = shell_quote(&format!("tests/{name}.rs")),
            name = shell_quote(name)
        ));
    }
    out.push_str(
        r#"    if [ "$#" -gt 0 ]; then
      echo "AIRGAP_INFO: targeted cargo integration test run" >&2
      exec cargo test "$@"
    fi
  fi
"#,
    );
    out
}
fn is_js_test_file(path: &str) -> bool {
    if !is_safe_target_path(path) {
        return false;
    }
    [
        ".spec.ts",
        ".spec.tsx",
        ".spec.js",
        ".spec.jsx",
        ".spec.mjs",
        ".spec.cjs",
        ".test.ts",
        ".test.tsx",
        ".test.js",
        ".test.jsx",
        ".test.mjs",
        ".test.cjs",
    ]
    .iter()
    .any(|suffix| path.ends_with(suffix))
}
