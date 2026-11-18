use std::{collections::BTreeMap, fs, path::Path};

use airgap::{
    config::Config,
    setup::{self, Editor, Options},
};
use serde_json::{json, Value};

fn config() -> Config {
    Config::from_values(BTreeMap::from([
        ("GMI_AGENTBOX_API_KEY".into(), "synthetic-compute".into()),
        ("GMI_MAAS_API_KEY".into(), "synthetic-inference".into()),
    ]))
    .unwrap()
}
fn options(editors: &[Editor]) -> Options {
    Options {
        yes: true,
        rules_only: false,
        editors: editors.iter().copied().collect(),
    }
}
fn read(path: impl AsRef<Path>) -> Value {
    serde_json::from_slice(&fs::read(path).unwrap()).unwrap()
}

#[test]
fn test_setup_cursor_merge() {
    let temp = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();
    fs::create_dir(temp.path().join(".cursor")).unwrap();
    let path = temp.path().join(".cursor/mcp.json");
    fs::write(&path, r#"{"preference":42,"mcpServers":{"other":{"command":"keep"},"airgap":{"custom":true,"env":{"KEEP":"value","GMI_MAAS_API_KEY":"old"}}}}"#).unwrap();
    setup::apply(
        temp.path(),
        Some(home.path()),
        &options(&[Editor::Cursor]),
        &config(),
        "airgap",
    )
    .unwrap();
    let value = read(&path);
    assert_eq!(value["preference"], 42);
    assert_eq!(value["mcpServers"]["other"]["command"], "keep");
    assert_eq!(value["mcpServers"]["airgap"]["custom"], true);
    assert_eq!(value["mcpServers"]["airgap"]["env"]["KEEP"], "value");
    assert_eq!(
        value["mcpServers"]["airgap"]["env"]["GMI_MAAS_API_KEY"],
        "synthetic-inference"
    );
    assert!(home.path().join(".cursor/mcp.json").is_file());
    assert!(!temp.path().join(".mcp.json").exists());
}

#[test]
fn test_setup_claude_merge() {
    let temp = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();
    fs::write(
        home.path().join(".claude.json"),
        r#"{"hasCompletedOnboarding":true,"mcpServers":{"existing":{"command":"keep"}}}"#,
    )
    .unwrap();
    setup::apply(
        temp.path(),
        Some(home.path()),
        &options(&[Editor::Claude]),
        &config(),
        "airgap",
    )
    .unwrap();
    let project = read(temp.path().join(".mcp.json"));
    assert_eq!(project["mcpServers"]["airgap"]["command"], "airgap");
    assert_eq!(project["mcpServers"]["airgap"]["args"], json!(["--mcp"]));
    assert_eq!(
        read(home.path().join(".claude.json"))["hasCompletedOnboarding"],
        true
    );
    assert_eq!(
        read(home.path().join(".claude.json"))["mcpServers"]["existing"]["command"],
        "keep"
    );
}

#[test]
fn test_setup_idempotency() {
    let temp = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();
    let options = options(&[Editor::Cursor, Editor::Claude, Editor::VsCode]);
    let paths = setup::apply(
        temp.path(),
        Some(home.path()),
        &options,
        &config(),
        "airgap",
    )
    .unwrap();
    let before: Vec<_> = paths.iter().map(|path| fs::read(path).unwrap()).collect();
    assert!(setup::apply(
        temp.path(),
        Some(home.path()),
        &options,
        &config(),
        "airgap"
    )
    .unwrap()
    .is_empty());
    for (path, bytes) in paths.iter().zip(before) {
        assert_eq!(fs::read(path).unwrap(), bytes);
    }
}

#[test]
fn vscode_preserves_inputs_and_other_servers_and_adds_portable_config() {
    let temp = tempfile::tempdir().unwrap();
    fs::create_dir(temp.path().join(".vscode")).unwrap();
    fs::write(temp.path().join(".vscode/mcp.json"),r#"{"inputs":[{"id":"retain"}],"servers":{"other":{"type":"http","url":"https://example.invalid"}}}"#).unwrap();
    setup::apply(
        temp.path(),
        None,
        &options(&[Editor::VsCode]),
        &config(),
        "airgap",
    )
    .unwrap();
    let value = read(temp.path().join(".vscode/mcp.json"));
    assert_eq!(value["inputs"][0]["id"], "retain");
    assert_eq!(value["servers"]["other"]["type"], "http");
    assert_eq!(value["servers"]["airgap"]["type"], "stdio");
    assert!(temp.path().join(".mcp.json").is_file());
    assert!(temp
        .path()
        .join(".github/copilot-instructions.md")
        .is_file());
}

#[test]
fn missing_keys_do_not_erase_existing_editor_credentials() {
    let config = Config::from_values(BTreeMap::new()).unwrap();
    let old = json!({"mcpServers":{"airgap":{"env":{"GMI_MAAS_API_KEY":"existing"}}}});
    let new = setup::merge_config(old, "mcpServers", &config, "airgap").unwrap();
    assert_eq!(
        new["mcpServers"]["airgap"]["env"]["GMI_MAAS_API_KEY"],
        "existing"
    );
}

#[test]
fn stale_remote_transport_does_not_override_the_installed_stdio_command() {
    let old = json!({"mcpServers":{"airgap":{"url":"https://example.invalid","headers":{"Authorization":"synthetic"},"custom":true}}});
    let new = setup::merge_config(old, "mcpServers", &config(), "airgap").unwrap();
    let server = &new["mcpServers"]["airgap"];
    assert!(server.get("url").is_none() && server.get("headers").is_none());
    assert_eq!(server["command"], "airgap");
    assert_eq!(server["custom"], true);
}

#[test]
fn malformed_config_aborts_before_any_files_are_written() {
    let temp = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();
    let path = home.path().join(".claude.json");
    let original = "{ invalid: 'synthetic-secret' }";
    fs::write(&path, original).unwrap();
    let error = setup::apply(
        temp.path(),
        Some(home.path()),
        &options(&[Editor::Claude, Editor::Cursor]),
        &config(),
        "airgap",
    )
    .unwrap_err();
    assert!(!error.to_string().contains("synthetic-secret"));
    assert_eq!(fs::read_to_string(path).unwrap(), original);
    assert!(!temp.path().join(".cursor").exists());
    assert!(!temp.path().join("CLAUDE.md").exists());
}

#[test]
fn routing_rules_preserve_existing_text_and_replace_only_the_managed_block() {
    let original = "# Team rules\nKeep this text.\n";
    let first = setup::merge_rules(original).unwrap();
    let with_suffix = format!("{first}\n# More team rules\nKeep this too.\n");
    assert!(with_suffix.starts_with(original));
    assert_eq!(setup::merge_rules(&with_suffix).unwrap(), with_suffix);
    assert_eq!(with_suffix.matches("<!-- airgap:begin -->").count(), 1);
    assert!(setup::merge_rules("<!-- airgap:begin -->\nmissing end").is_err());
}

#[test]
fn rules_only_does_not_read_or_change_editor_configs() {
    let temp = tempfile::tempdir().unwrap();
    fs::write(temp.path().join(".mcp.json"), "invalid config").unwrap();
    let options = Options {
        yes: true,
        rules_only: true,
        editors: Default::default(),
    };
    let paths = setup::apply(temp.path(), None, &options, &config(), "airgap").unwrap();
    assert_eq!(paths.len(), 2);
    assert_eq!(
        fs::read_to_string(temp.path().join(".mcp.json")).unwrap(),
        "invalid config"
    );
}

#[test]
fn invalid_server_shapes_are_rejected_without_discarding_values() {
    for value in [
        json!([]),
        json!({"mcpServers":null}),
        json!({"mcpServers":{"airgap":[]}}),
        json!({"mcpServers":{"airgap":{"env":null}}}),
    ] {
        assert!(setup::merge_config(value, "mcpServers", &config(), "airgap").is_err());
    }
    assert!(Options::parse(&["--unknown".into()]).is_err());
}

#[cfg(unix)]
#[test]
fn credential_files_are_private_and_symlinks_are_refused() {
    use std::os::unix::fs::{symlink, PermissionsExt};
    let temp = tempfile::tempdir().unwrap();
    fs::write(
        temp.path().join("CLAUDE.md"),
        "Private existing instructions\n",
    )
    .unwrap();
    fs::set_permissions(
        temp.path().join("CLAUDE.md"),
        fs::Permissions::from_mode(0o600),
    )
    .unwrap();
    setup::apply(
        temp.path(),
        None,
        &options(&[Editor::Claude]),
        &config(),
        "airgap",
    )
    .unwrap();
    assert_eq!(
        fs::metadata(temp.path().join(".mcp.json"))
            .unwrap()
            .permissions()
            .mode()
            & 0o777,
        0o600
    );
    fs::create_dir(temp.path().join(".cursor")).unwrap();
    assert_eq!(
        fs::metadata(temp.path().join("CLAUDE.md"))
            .unwrap()
            .permissions()
            .mode()
            & 0o777,
        0o600
    );
    symlink(
        temp.path().join(".mcp.json"),
        temp.path().join(".cursor/mcp.json"),
    )
    .unwrap();
    assert!(setup::apply(
        temp.path(),
        None,
        &options(&[Editor::Cursor]),
        &config(),
        "airgap"
    )
    .is_err());
}

#[cfg(unix)]
#[test]
fn symlinked_editor_directories_cannot_redirect_credentials() {
    let workspace = tempfile::tempdir().unwrap();
    let elsewhere = tempfile::tempdir().unwrap();
    std::os::unix::fs::symlink(elsewhere.path(), workspace.path().join(".cursor")).unwrap();
    assert!(setup::apply(
        workspace.path(),
        None,
        &options(&[Editor::Cursor]),
        &config(),
        "airgap"
    )
    .is_err());
    assert!(!elsewhere.path().join("mcp.json").exists());
}
