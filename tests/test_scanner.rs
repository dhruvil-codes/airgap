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
    std::fs::write(child.join("cloudvet.env"), "GMI_MODEL=child-model\n").unwrap();
    let environment = BTreeMap::from([("GMI_MAAS_API_KEY".into(), "inference".into())]);
    let config = Config::load(&child, environment).unwrap();
    assert_eq!(config.model, "child-model");
    assert_eq!(config.agentbox_key.as_deref(), Some("parent"));
    assert_eq!(config.maas_key.as_deref(), Some("inference"));
}

#[test]
fn dotenv_cannot_override_provider_base_urls() {
    let temp = tempfile::tempdir().unwrap();
    std::fs::write(
        temp.path().join(".env"),
        "GMI_AGENTBOX_BASE_URL=https://attacker.example/agents\nGMI_MAAS_BASE_URL=https://attacker.example/v1\nGMI_MODEL=dotenv-model\n",
    )
    .unwrap();
    let environment = BTreeMap::from([("GMI_API_KEY".into(), "process-key".into())]);
    let config = Config::load(temp.path(), environment).unwrap();
    assert_eq!(
        config.agentbox_base_url,
        airgap::config::DEFAULT_AGENTBOX_BASE_URL
    );
    assert_eq!(config.maas_base_url, airgap::config::DEFAULT_MAAS_BASE_URL);
    assert_eq!(config.model, "dotenv-model");

    let environment = BTreeMap::from([(
        "GMI_MAAS_BASE_URL".into(),
        "https://proxy.example/v1".into(),
    )]);
    let config = Config::load(temp.path(), environment).unwrap();
    assert_eq!(config.maas_base_url, "https://proxy.example/v1");
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
