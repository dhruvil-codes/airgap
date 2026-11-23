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
    assert!(script.contains(
        "if [ -f 'source/cli-args.spec.ts' ]; then set -- \"$@\" 'source/cli-args.spec.ts'; fi"
    ));
    assert!(script.contains("exec \"$PM\" exec vitest run -- \"$@\""));
    assert!(!script.contains("--config=evil.test.js"));
    assert!(script.contains("\"$PM\" run build"));
    assert!(script.contains("https://cache.example.com"));
}

fn pr_script_for(files: &[&str]) -> String {
    use airgap::tools::pr::{build_pr_script, PrTarget};

    let target = PrTarget {
        owner: "o".into(),
        repo: "r".into(),
        number: 7,
    };
    let files: Vec<String> = files.iter().map(|f| (*f).to_owned()).collect();
    build_pr_script(
        &target,
        "0123456789abcdef0123456789abcdef01234567",
        &files,
        None,
        None,
        None,
        false,
        None,
    )
}

#[test]
fn test_targets_pair_sources_with_sibling_tests() {
    use airgap::tools::pr::resolve_test_targets;

    let files: Vec<String> = [
        "source/models/models-dev-client.ts",
        "source/cli-args.spec.ts",
        "src/ui/button.jsx",
        "test/smoke.js",
        "pkg/parser.py",
        "pkg/test_lexer.py",
        "tests/api_test.py",
        "tests/integration_http.rs",
        "README.md",
        "docs/guide.md",
    ]
    .iter()
    .map(|f| (*f).to_owned())
    .collect();
    let targets = resolve_test_targets(&files);

    assert!(!targets.is_full_suite());
    assert_eq!(
        targets.js_tests,
        vec!["source/cli-args.spec.ts", "test/smoke.js"]
    );
    let sources: Vec<&str> = targets
        .js_sources
        .iter()
        .map(|s| s.source.as_str())
        .collect();
    assert_eq!(
        sources,
        vec!["source/models/models-dev-client.ts", "src/ui/button.jsx"]
    );
    assert_eq!(
        targets.js_sources[0].candidates,
        vec![
            "source/models/models-dev-client.spec.ts",
            "source/models/models-dev-client.spec.tsx",
            "source/models/models-dev-client.test.ts",
            "source/models/models-dev-client.test.tsx",
        ]
    );
    assert!(targets.js_sources[1]
        .candidates
        .iter()
        .any(|c| c == "src/ui/button.test.mjs"));
    assert_eq!(
        targets.py_tests,
        vec!["pkg/test_lexer.py", "tests/api_test.py"]
    );
    assert_eq!(
        targets.py_sources[0].candidates,
        vec![
            "pkg/test_parser.py",
            "pkg/parser_test.py",
            "tests/test_parser.py"
        ]
    );
    assert_eq!(targets.cargo_integration_tests, vec!["integration_http"]);
    assert!(!targets.rust_sources_changed);

    // A source whose sibling test is itself in the PR is not run twice, but still counts as covered.
    let both: Vec<String> = vec!["src/a.ts".into(), "src/a.test.ts".into()];
    let targets = resolve_test_targets(&both);
    assert_eq!(targets.js_tests, vec!["src/a.test.ts"]);
    assert!(!targets
        .js_sibling_candidates()
        .iter()
        .any(|c| c.as_str() == "src/a.test.ts"));
    assert!(targets.js_sources[0]
        .candidates
        .iter()
        .any(|c| c == "src/a.test.ts"));

    // Snapshot updates require the test file that owns the snapshot.
    let snap = resolve_test_targets(&["src/__snapshots__/a.spec.ts.snap".to_owned()]);
    assert!(!snap.is_full_suite());
    assert_eq!(snap.js_sources[0].candidates, vec!["src/a.spec.ts"]);
}

#[test]
fn test_targets_fall_back_to_full_suite_for_shared_config() {
    use airgap::tools::pr::resolve_test_targets;

    for trigger in [
        "package.json",
        "packages/core/package.json",
        "pnpm-lock.yaml",
        "yarn.lock",
        "Cargo.lock",
        "Cargo.toml",
        "tsconfig.json",
        "tsconfig.build.json",
        ".github/workflows/ci.yml",
        "vitest.config.ts",
        "jest.config.js",
        "pyproject.toml",
        "requirements-dev.txt",
        "tests/conftest.py",
        "tests/helpers/server.ts",
        "test/_setup.js",
        "tests/fixtures/data.json",
        "tests/common/mod.rs",
        // Files no sibling-pairing rule covers are treated as untested source.
        "source/types.d.ts",
        "pkg/__init__.py",
        "src/App.vue",
        "src/styles.css",
        "source/locales/en.json",
        "scripts/release.sh",
    ] {
        let files = vec!["source/cli-args.spec.ts".to_owned(), trigger.to_owned()];
        let targets = resolve_test_targets(&files);
        assert!(
            targets.is_full_suite(),
            "{trigger} should force the full suite"
        );
        assert!(targets.js_tests.is_empty() && targets.js_sources.is_empty());
        assert!(targets.full_suite_reason.unwrap().contains(trigger));
    }

    let many: Vec<String> = (0..21).map(|i| format!("src/t{i}.test.ts")).collect();
    assert!(resolve_test_targets(&many).is_full_suite());
    let many_sources: Vec<String> = (0..21).map(|i| format!("src/m{i}.ts")).collect();
    assert!(resolve_test_targets(&many_sources).is_full_suite());

    // Docs-only PRs have no targets; the sandbox script then runs the full suite.
    let docs = resolve_test_targets(&["README.md".to_owned(), "docs/guide.md".to_owned()]);
    assert!(!docs.is_full_suite());
    assert!(docs.js_tests.is_empty() && docs.js_sources.is_empty());
    let script = pr_script_for(&["README.md"]);
    assert!(!script.contains("RUNNER="));
    assert!(!script.contains("cargo test \"$@\""));
    assert!(!script.contains("pytest -- \"$@\""));
    assert!(script.contains("exec \"$PM\" run \"$TEST_SCRIPT\""));
    assert!(script.contains("exec cargo test\n"));
    assert!(script.contains("exec pytest\n"));

    let script = pr_script_for(&["source/a.spec.ts", "pnpm-lock.yaml"]);
    assert!(script.contains("AIRGAP_INFO: running full test suite: pnpm-lock.yaml"));
    assert!(!script.contains("'source/a.spec.ts'"));
    assert!(!script.contains("RUNNER="));
}

#[test]
fn unpaired_sources_force_full_suite_in_sandbox() {
    let script = pr_script_for(&["source/models/models-dev-client.ts", "source/other.spec.ts"]);
    // The source is covered only if at least one sibling exists; otherwise every targeted block is skipped.
    assert!(script.contains(
        "if [ ! -f 'source/models/models-dev-client.spec.ts' ] && [ ! -f 'source/models/models-dev-client.spec.tsx' ] && [ ! -f 'source/models/models-dev-client.test.ts' ] && [ ! -f 'source/models/models-dev-client.test.tsx' ]; then echo 'AIRGAP_INFO: running full test suite: no sibling test found for source/models/models-dev-client.ts' >&2; TARGET_FULL=1; fi"
    ));
    let check = script.find("TARGET_FULL=0").unwrap();
    let gate = script.find("if [ \"$TARGET_FULL\" = 0 ]; then").unwrap();
    let branch = check
        + script[check..]
            .find("if [ -f package.json ]; then")
            .unwrap();
    assert!(branch < gate);
    assert!(gate < script.find("RUNNER=").unwrap());
}

#[test]
fn rust_source_changes_run_full_cargo_test() {
    use airgap::tools::pr::resolve_test_targets;

    for source in [
        "src/lib.rs",
        "src/tools/pr.rs",
        "crates/core/src/lib.rs",
        "examples/demo.rs",
    ] {
        let targets =
            resolve_test_targets(&["tests/test_scanner.rs".to_owned(), source.to_owned()]);
        assert!(targets.rust_sources_changed, "{source}");
        assert!(targets.cargo_integration_tests.is_empty(), "{source}");
        let script = pr_script_for(&["tests/test_scanner.rs", source]);
        assert!(!script.contains("--test 'test_scanner'"), "{source}");
        assert!(!script.contains("exec cargo test \"$@\""), "{source}");
        assert!(script.contains("exec cargo test\n"));
    }

    let only_tests = resolve_test_targets(&["tests/test_scanner.rs".to_owned()]);
    assert!(!only_tests.rust_sources_changed);
    assert_eq!(only_tests.cargo_integration_tests, vec!["test_scanner"]);
}

#[test]
fn targeted_test_commands_are_runner_specific_and_quoted() {
    let script = pr_script_for(&[
        "source/models/models-dev-client.ts",
        "source/it's here.test.ts",
        "--reporter=evil.test.js",
        "-rf.spec.ts",
        "../escape.test.ts",
        "pkg/test_mod.py",
        "tests/cli.rs",
    ]);

    // Sibling candidates are existence-checked inside the sandbox before they are run.
    assert!(script.contains(
        "if [ -f 'source/models/models-dev-client.spec.ts' ]; then set -- \"$@\" 'source/models/models-dev-client.spec.ts'; fi"
    ));
    assert!(script.contains("'source/it'\"'\"'s here.test.ts'"));
    assert!(!script.contains("--reporter=evil"));
    assert!(!script.contains("-rf.spec.ts"));
    assert!(!script.contains("escape.test.ts"));

    assert!(script.contains("exec \"$PM\" exec ava -- \"$@\""));
    assert!(script.contains("exec \"$PM\" exec vitest run -- \"$@\""));
    assert!(script.contains("exec \"$PM\" exec jest --runTestsByPath \"$@\""));
    assert!(script.contains("exec node --test \"$@\""));
    assert!(script.contains("set -- \"$@\" \"./$f\""));
    assert!(
        script.contains("if [ -f 'pkg/test_mod.py' ]; then set -- \"$@\" 'pkg/test_mod.py'; fi")
    );
    assert!(script.contains("exec pytest -- \"$@\""));
    assert!(script.contains("if [ -f 'tests/cli.rs' ]; then set -- \"$@\" --test 'cli'; fi"));
    assert!(script.contains("exec cargo test \"$@\""));

    // Targeted runs come before the full-suite fallback in each branch.
    let targeted = script.find("exec \"$PM\" exec ava").unwrap();
    let full = script.find("exec \"$PM\" run \"$TEST_SCRIPT\"").unwrap();
    assert!(targeted < full);
    let cargo_targeted = script.find("exec cargo test \"$@\"").unwrap();
    let cargo_full = script.find("exec cargo test\n").unwrap();
    assert!(cargo_targeted < cargo_full);
}
#[test]
fn pr_script_prefers_prewarmed_stores_and_hydrates_by_lockfile_hash() {
    use airgap::gmi::agentbox::{
        CLEAN_PNPM_STORE, LOCAL_PNPM_STORE, PREWARMED_CARGO_HOME, PREWARMED_PNPM_STORE,
    };
    use airgap::tools::pr::{build_pr_script, PrTarget};

    let target = PrTarget {
        owner: "o".into(),
        repo: "r".into(),
        number: 7,
    };
    let sha = "0123456789abcdef0123456789abcdef01234567";
    let script = build_pr_script(
        &target,
        sha,
        &["source/a.spec.ts".to_owned()],
        None,
        None,
        None,
        false,
        Some("https://cache.example.com"),
    );

    // Pre-warmed stores are used only when present and writable, with a local store fallback.
    assert_eq!(PREWARMED_PNPM_STORE, "/opt/airgap/cache/pnpm");
    assert_eq!(PREWARMED_CARGO_HOME, "/opt/airgap/cache/cargo");
    assert!(script.contains(
        "if [ -d /opt/airgap/cache/pnpm ] && [ -w /opt/airgap/cache/pnpm ]; then\n  export PNPM_STORE_DIR=/opt/airgap/cache/pnpm"
    ));
    assert!(script.contains(&format!("export PNPM_STORE_DIR={LOCAL_PNPM_STORE}")));
    assert!(script.contains(
        "if [ -d /opt/airgap/cache/cargo ] && [ -w /opt/airgap/cache/cargo ]; then\n  export CARGO_HOME=/opt/airgap/cache/cargo"
    ));
    assert!(script.contains("export npm_config_verify_store_integrity=true"));

    // Hydration is keyed on a validated SHA-256 of the lockfile and tests the archive before unpacking.
    let store_pos = script.find("AIRGAP_PNPM_CACHED=0").unwrap();
    let checkout_pos = script.find("git checkout -q --detach").unwrap();
    let hash_pos = script.find("sha256sum pnpm-lock.yaml").unwrap();
    let install_pos = script.find("pnpm install").unwrap();
    assert!(store_pos < checkout_pos && checkout_pos < hash_pos && hash_pos < install_pos);
    assert!(script.contains("*[!0-9a-f]*) LOCK_HASH=\"\" ;;"));
    assert!(script.contains("[ \"${#LOCK_HASH}\" -eq 64 ]"));
    assert!(script.contains("'https://cache.example.com'\"/pnpm-${LOCK_HASH}.tar.zst\""));
    assert!(script.contains("--max-filesize 4294967296"));
    let test_pos = script.find("zstd -tq -T0 \"$CACHE_ARCHIVE\"").unwrap();
    let extract_pos = script
        .find("zstd -dcq -T0 \"$CACHE_ARCHIVE\" | tar --no-same-owner --no-same-permissions -xf - -C \"$PNPM_STORE_DIR\"")
        .unwrap();
    assert!(test_pos < extract_pos);

    // Both install attempts are hardened, and the retry runs only when a cached store was used.
    let hardened = "pnpm install --frozen-lockfile --prefer-offline --ignore-scripts --ignore-pnpmfile --store-dir \"$PNPM_STORE_DIR\" --child-concurrency=4";
    assert_eq!(script.matches(hardened).count(), 2);
    let retry_gate = script
        .find("if [ \"$AIRGAP_PNPM_CACHED\" = 1 ]; then")
        .unwrap();
    let clean = script
        .find(&format!("export PNPM_STORE_DIR={CLEAN_PNPM_STORE}"))
        .unwrap();
    assert!(install_pos < retry_gate && retry_gate < clean);
    assert!(script[retry_gate..clean].contains("rm -rf node_modules"));

    // Without a cache URL there is no network hydration, but pre-warmed stores and the retry remain.
    let offline = build_pr_script(&target, sha, &[], None, None, None, false, None);
    assert!(!offline.contains("sha256sum pnpm-lock.yaml"));
    assert!(!offline.contains("curl"));
    assert!(offline.contains("export PNPM_STORE_DIR=/opt/airgap/cache/pnpm"));
    assert!(offline.contains(&format!("export PNPM_STORE_DIR={CLEAN_PNPM_STORE}")));

    // Opting into install scripts drops the hardening flags from both attempts only.
    let scripts_allowed = build_pr_script(&target, sha, &[], None, None, None, true, None);
    assert_eq!(
        scripts_allowed
            .matches("pnpm install --frozen-lockfile --prefer-offline --store-dir \"$PNPM_STORE_DIR\" --child-concurrency=4")
            .count(),
        2
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
