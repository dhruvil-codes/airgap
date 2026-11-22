use std::collections::BTreeMap;

use airgap::{
    config::Config,
    gmi::{
        agentbox::{pack_files, validate_files, Files},
        maas::parse_json,
    },
    receipt::Receipt,
    tools::{
        config_tool::{configure, ConfigureRequest, RuntimeConfig},
        sandbox::test_counts,
        secrets::{entropy, redact},
        supply::{audit, ManifestType, RiskScore, SupplyRequest},
    },
};
use serde_json::{json, Value};

#[test]
fn redacts_credentials_without_exposing_values_or_double_counting() {
    let input = "API_KEY=small-secret\nAWS_ACCESS_KEY_ID=AKIAIOSFODNN7EXAMPLE\nGitHub ghp_abcdefghijklmnopqrstuvwxyz0123456789\nAuthorization: Bearer short-token\nnormal output: 42 ✓";
    let output = redact(input);
    assert!(!output.sanitized_text.contains("small-secret"));
    assert!(!output.sanitized_text.contains("AKIAIOS"));
    assert!(!output.sanitized_text.contains("ghp_"));
    assert!(!output.sanitized_text.contains("short-token"));
    assert!(output.sanitized_text.contains("normal output: 42 ✓"));
    assert_eq!(output.redacted_items_count, 4);
}

#[test]
fn settings_with_sensitive_substrings_are_not_redacted() {
    for input in [
        "max_tokens = 4096",
        "tokenizer: gpt2",
        "MAX_TOKENS=4096",
        "retries_token_bucket: 3",
    ] {
        let result = redact(input);
        assert_eq!(result.sanitized_text, input);
        assert_eq!(result.redacted_items_count, 0);
    }
}

#[test]
fn short_passwords_are_redacted_but_short_settings_are_not() {
    assert_eq!(
        redact(r#"password = "hunter2""#).sanitized_text,
        r#"password = "[REDACTED]""#
    );
    assert_eq!(redact("passwd: admin").sanitized_text, "passwd: [REDACTED]");
    for input in ["max_tokens = 4096", "MAX_TOKENS=4096", "tokenizer: gpt2"] {
        assert_eq!(redact(input).sanitized_text, input);
    }
}

#[test]
fn delimited_and_camel_case_credential_keys_are_redacted() {
    let github = format!("ghp_{}", "a1B2".repeat(9));
    for (input, secret) in [
        (format!("GITHUB_TOKEN = \"{github}\""), github.as_str()),
        ("api_key: secret12345".into(), "secret12345"),
        ("x-api-key=abcdefgh1".into(), "abcdefgh1"),
        ("accessToken: 'opaque-value'".into(), "opaque-value"),
        (
            "AWS_SECRET_ACCESS_KEY=wJalrXUtnFEMI".into(),
            "wJalrXUtnFEMI",
        ),
    ] {
        let result = redact(&input);
        assert!(
            !result.sanitized_text.contains(secret),
            "{input} -> {}",
            result.sanitized_text
        );
    }
}

#[test]
fn private_keys_are_redacted_including_incomplete_keys() {
    for input in ["before\n-----BEGIN OPENSSH PRIVATE KEY-----\nabc123\n-----END OPENSSH PRIVATE KEY-----\nafter", "-----BEGIN RSA PRIVATE KEY-----\ntruncated"] {
        let result = redact(input);
        assert!(!result.sanitized_text.contains("PRIVATE KEY"));
        assert_eq!(result.redacted_items_count, 1);
    }
}

#[test]
fn quoted_json_secrets_and_entropy_are_detected() {
    let result =
        redact(r#"{"password":"has spaces too", "ok":true} abcdefghijklmnopQRSTUV1234567890"#);
    assert!(!result.sanitized_text.contains("has spaces"));
    assert_eq!(result.redacted_items_count, 2);
    assert!(entropy("abcdefghijklmnopQRSTUV1234567890") > 4.0);
    assert_eq!(entropy("aaaaa"), 0.0);
    assert_eq!(entropy(""), 0.0);
}

#[test]
fn escaped_quotes_and_unclosed_values_do_not_leave_secret_suffixes() {
    for input in [
        r#"PASSWORD="abc\"remaining-secret""#,
        "API_KEY=\nnormal output",
        r#"client-secret='has spaces and \' remaining-secret'"#,
    ] {
        let result = redact(input);
        assert!(!result.sanitized_text.contains("remaining-secret"));
        if input == "API_KEY=\nnormal output" {
            assert!(result.sanitized_text.contains("normal output"));
        }
    }
    assert_eq!(
        redact("PASSWORD=\"unterminated value").sanitized_text,
        "PASSWORD=\"[REDACTED]"
    );
}

#[test]
fn malformed_model_output_is_never_accepted_as_clean() {
    for input in ["safe", "<think>not closed", "```json\n{}", "{} extra text"] {
        assert!(parse_json::<Value>(input).is_err());
    }
    let value: Value =
        parse_json("<think>private reasoning</think>\n```json\n{\"safe\":false}\n```").unwrap();
    assert_eq!(value, json!({"safe": false}));
    let value: Value = parse_json(r#"{"text":"<think>literal</think>"}"#).unwrap();
    assert_eq!(value["text"], "<think>literal</think>");
}

#[test]
fn npm_scripts_and_remote_sources_are_flagged() {
    let output = audit(SupplyRequest { manifest_type: ManifestType::Npm, manifest_content: r#"{"scripts":{"postinstall":"node install.js"},"dependencies":{"x":"https://example.invalid/x.tgz"}}"#.into() }).unwrap();
    assert!(matches!(output.risk_score, RiskScore::High));
    assert_eq!(output.flagged_scripts.len(), 2);
    assert!(audit(SupplyRequest {
        manifest_type: ManifestType::Npm,
        manifest_content: "bad json".into()
    })
    .is_err());
}

#[test]
fn cargo_build_hooks_and_python_indexes_require_review() {
    let output = audit(SupplyRequest { manifest_type: ManifestType::Cargo, manifest_content: "[package]\nbuild='build.rs'\n[target.'cfg(unix)'.dependencies]\nx={git='https://example.invalid/x'}\n".into() }).unwrap();
    assert_eq!(output.flagged_scripts.len(), 2);
    let output = audit(SupplyRequest { manifest_type: ManifestType::Python, manifest_content: "# comment\nrequests==2.32.0\n--extra-index-url https://example.invalid/simple\n-r private.txt".into() }).unwrap();
    assert_eq!(output.flagged_scripts.len(), 2);
}

#[test]
fn snapshot_paths_cannot_escape_or_upload_git_metadata() {
    for path in [
        "../secret",
        "/etc/passwd",
        "C:/secret",
        "foo\\bar",
        "a/../b",
        "a//b",
        ".git/config",
        "a/.GIT/hooks/x",
        "x\nfile",
        "",
    ] {
        assert!(
            validate_files(&Files::from([(path.into(), "hello".into())])).is_err(),
            "accepted {path}"
        );
    }
}

#[test]
fn tar_contains_only_explicit_relative_regular_files() {
    let files = Files::from([("src/main.rs".into(), "fn main() {}".into())]);
    let archive = pack_files(&files).unwrap();
    let decoder = flate2::read::GzDecoder::new(archive.as_slice());
    let mut tar = tar::Archive::new(decoder);
    let entries: Vec<_> = tar.entries().unwrap().map(|e| e.unwrap()).collect();
    assert_eq!(entries.len(), 1);
    assert!(entries[0].header().entry_type().is_file());
    assert_eq!(entries[0].path().unwrap().to_str(), Some("src/main.rs"));
}

#[test]
fn unknown_test_output_never_invents_passing_tests() {
    assert_eq!(test_counts("done"), (None, None));
    assert_eq!(test_counts("test result: ok. 7 passed; 0 failed; 0 ignored\ntest result: FAILED. 2 passed; 1 failed;"), (Some(9), Some(1)));
    assert_eq!(test_counts("=== 3 passed in 0.12s ==="), (Some(3), Some(0)));
    assert_eq!(
        test_counts("=== 1 failed, 2 passed, 1 error in 0.12s ==="),
        (Some(2), Some(2))
    );
    // Vitest (including ANSI colors)
    assert_eq!(
        test_counts(" \x1b[2mTest Files\x1b[22m  1 passed (1)\n \x1b[2m     Tests\x1b[22m  \x1b[32m14 passed\x1b[39m (14)\n"),
        (Some(14), Some(0))
    );
    assert_eq!(
        test_counts("      Tests  2 failed | 12 passed (14)\n"),
        (Some(12), Some(2))
    );
    // Jest
    assert_eq!(
        test_counts("Tests:       1 failed, 9 passed, 10 total\n"),
        (Some(9), Some(1))
    );
    // Node built-in test runner
    assert_eq!(
        test_counts("# tests 5\n# pass 5\n# fail 0\n"),
        (Some(5), Some(0))
    );
    // Multi-suite Jest/Vitest aggregation + trailing unrelated `Tests ...` line cannot shadow failures
    assert_eq!(
        test_counts(
            "Tests:       2 failed, 8 passed, 10 total\nTests:       5 passed, 5 total\nTests completed in 3.2s\n"
        ),
        (Some(13), Some(2))
    );
    // Multi-suite Node built-in test runner aggregation
    assert_eq!(
        test_counts("# tests 5\n# pass 4\n# fail 1\n# tests 3\n# pass 3\n# fail 0\n"),
        (Some(7), Some(1))
    );
    // AVA
    assert_eq!(test_counts("  14 tests passed\n"), (Some(14), Some(0)));
    assert_eq!(
        test_counts("  2 tests failed\n  12 tests passed\n"),
        (Some(12), Some(2))
    );
}

#[test]
fn pr_target_parsing_rejects_non_github_and_traversal() {
    use airgap::tools::pr::{build_pr_script, parse_pr_target, PrVerifyRequest};

    let req = PrVerifyRequest {
        pr_url: Some("https://github.com/Nano-Collective/nanocoder/pull/2313/files".into()),
        repo: None,
        pr_number: None,
        test_command: None,
        build_command: None,
        build_first: None,
        image: None,
        timeout_secs: None,
        fail_on_supply_risk: true,
        allow_install_scripts: false,
    };
    let target = parse_pr_target(&req).unwrap();
    assert_eq!(target.owner, "Nano-Collective");
    assert_eq!(target.repo, "nanocoder");
    assert_eq!(target.number, 2313);

    for bad_url in [
        "http://github.com/a/b/pull/1",
        "https://evil.example/a/b/pull/1",
        "https://github.com/../b/pull/1",
        "https://github.com/a/b/pull/0",
        "https://github.com/a/b/pull/1;rm -rf /",
    ] {
        let mut bad = req.clone();
        bad.pr_url = Some(bad_url.into());
        assert!(parse_pr_target(&bad).is_err(), "{bad_url} should fail");
    }

    let script = build_pr_script(
        &target,
        "0123456789abcdef0123456789abcdef01234567",
        &[
            "source/cli-args.spec.ts".into(),
            "--config=evil.test.js".into(),
        ],
        None,
        None,
        None,
        false,
        Some("https://cache.example.com"),
    );
    assert!(script.contains("git config core.hooksPath /dev/null"));
    assert!(script.contains("git config core.fsmonitor false"));
    assert!(script.contains("git config submodule.recurse false"));
    assert!(script.contains("git config protocol.file.allow never"));
    assert!(script.contains(
        "git fetch -q --depth 1 --no-tags --filter=blob:none origin 'pull/2313/head:pr-verify'"
    ));
    let verify_pos = script
        .find("ACTUAL_SHA=\"$(git rev-parse pr-verify)\"")
        .unwrap();
    let checkout_pos = script
        .find("git checkout -q --detach \"$ACTUAL_SHA\"")
        .unwrap();
    assert!(
        verify_pos < checkout_pos,
        "SHA verification must happen before git checkout"
    );
    assert!(script.contains("0123456789abcdef0123456789abcdef01234567"));
    assert!(script.contains("--ignore-scripts --ignore-pnpmfile"));
    assert!(script.contains("export YARN_ENABLE_SCRIPTS=false"));
    assert!(script.contains("vitest run -- 'source/cli-args.spec.ts'"));
    assert!(!script.contains("--config=evil.test.js"));
    assert!(script.contains("\"$PM\" run build"));
    assert!(script.contains("https://cache.example.com"));
}

#[test]
fn config_updates_are_atomic_and_validate_timeouts() {
    let mut state = RuntimeConfig {
        model: "original".into(),
        timeout_seconds: 120,
    };
    assert!(configure(
        &mut state,
        ConfigureRequest {
            model: Some("new".into()),
            timeout_seconds: Some(0)
        }
    )
    .is_err());
    assert_eq!(state.model, "original");
    let output = configure(
        &mut state,
        ConfigureRequest {
            model: Some("provider/model".into()),
            timeout_seconds: Some(90),
        },
    )
    .unwrap();
    assert_eq!(output.active_model, "provider/model");
    assert_eq!(state.timeout_seconds, 90);
}

#[test]
fn config_discovery_precedence_and_credential_fallback() {
    let temp = tempfile::tempdir().unwrap();
    let child = temp.path().join("child");
    std::fs::create_dir(&child).unwrap();
    std::fs::write(
        temp.path().join(".env"),
        "GMI_API_KEY=parent\nGMI_MODEL=parent-model\n",
    )
    .unwrap();
    std::fs::write(
        child.join("cloudvet.env"),
        "GMI_AGENTBOX_API_KEY=child-compute\nGMI_MODEL=child-model\n",
    )
    .unwrap();
    let environment = BTreeMap::from([("GMI_MAAS_API_KEY".into(), "inference".into())]);
    let config = Config::load(&child, environment).unwrap();
    // Workspace .env files must never override GMI_MODEL.
    assert_eq!(config.model, airgap::config::DEFAULT_MODEL);
    assert_eq!(config.agentbox_key.as_deref(), Some("child-compute"));
    assert_eq!(config.maas_key.as_deref(), Some("inference"));

    // Process fallback credentials (GMI_API_KEY / GH_TOKEN) must strictly win over workspace .env.
    std::fs::write(
        child.join(".env"),
        "GMI_AGENTBOX_API_KEY=attacker-compute\nGMI_MAAS_API_KEY=attacker-maas\nGITHUB_TOKEN=attacker-gh\n",
    )
    .unwrap();
    let environment = BTreeMap::from([
        ("GMI_API_KEY".into(), "process-gmi".into()),
        ("GH_TOKEN".into(), "process-gh".into()),
    ]);
    let config = Config::load(&child, environment).unwrap();
    assert_eq!(config.agentbox_key.as_deref(), Some("process-gmi"));
    assert_eq!(config.maas_key.as_deref(), Some("process-gmi"));
    assert_eq!(config.github_token.as_deref(), Some("process-gh"));
}

#[test]
fn dotenv_cannot_override_provider_base_urls_or_runtime_settings() {
    let temp = tempfile::tempdir().unwrap();
    std::fs::write(
        temp.path().join(".env"),
        "GMI_AGENTBOX_BASE_URL=https://attacker.example/agents\n\
         GMI_MAAS_BASE_URL=https://attacker.example/v1\n\
         AIRGAP_CACHE_URL=https://attacker.example/cache\n\
         AIRGAP_NODE_IMAGE=attacker.example/evil-node:latest\n\
         GMI_MODEL=weak-model\n\
         GMI_AGENTBOX_INSTANCE_TYPE=gmi.sandbox.huge\n\
         GMI_DEFAULT_IDC=attacker-region\n",
    )
    .unwrap();
    let environment = BTreeMap::from([("GMI_API_KEY".into(), "process-key".into())]);
    let config = Config::load(temp.path(), environment).unwrap();
    assert_eq!(
        config.agentbox_base_url,
        airgap::config::DEFAULT_AGENTBOX_BASE_URL
    );
    assert_eq!(config.maas_base_url, airgap::config::DEFAULT_MAAS_BASE_URL);
    assert!(config.cache_base_url.is_none());
    assert_eq!(config.node_image, airgap::config::DEFAULT_NODE_IMAGE);
    assert_eq!(config.model, airgap::config::DEFAULT_MODEL);
    assert_eq!(config.instance_type, airgap::config::DEFAULT_INSTANCE_TYPE);
    assert_eq!(config.idc, airgap::config::DEFAULT_IDC);

    let environment = BTreeMap::from([
        (
            "GMI_MAAS_BASE_URL".into(),
            "https://proxy.example/v1".into(),
        ),
        ("GMI_MODEL".into(), "env-model".into()),
    ]);
    let config = Config::load(temp.path(), environment).unwrap();
    assert_eq!(config.maas_base_url, "https://proxy.example/v1");
    assert_eq!(config.model, "env-model");
}

#[test]
fn insecure_or_credential_bearing_provider_urls_are_rejected() {
    for url in [
        "http://localhost",
        "https://user:password@example.com/v1",
        "https://example.com/?key=secret",
    ] {
        assert!(
            Config::from_values(BTreeMap::from([("GMI_MAAS_BASE_URL".into(), url.into())]))
                .is_err()
        );
    }
}

#[test]
fn receipt_detects_changed_evidence() {
    let mut receipt =
        Receipt::new(&json!({"diff":"abc"}), &json!({"exit_code":0}), 42, None).unwrap();
    assert!(receipt.verify().unwrap());
    receipt.duration_ms = 43;
    assert!(!receipt.verify().unwrap());
}

#[test]
fn audit_rejects_invented_or_wrong_line_evidence() {
    use airgap::tools::audit::{evidence_matches, Finding, Severity};
    let files = Files::from([("main.rs".into(), "fn main() {\n unsafe_call();\n}".into())]);
    let mut finding = Finding {
        title: "review".into(),
        severity: Severity::Low,
        path: "main.rs".into(),
        line: 2,
        evidence: " unsafe_call();".into(),
        explanation: "review".into(),
        remediation: "review".into(),
    };
    assert!(evidence_matches(&files, &finding));
    finding.line = 1;
    assert!(!evidence_matches(&files, &finding));
    finding.line = 0;
    assert!(!evidence_matches(&files, &finding));
}
