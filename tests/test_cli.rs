use std::{
    fs,
    process::{Command, Stdio},
};

fn command(cwd: &std::path::Path, home: &std::path::Path) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_airgap"));
    command
        .current_dir(cwd)
        .env_clear()
        .env("HOME", home)
        .env("USERPROFILE", home)
        .env("PATH", "")
        .stdin(Stdio::null());
    if let Ok(root) = std::env::var("SystemRoot") {
        command.env("SystemRoot", root);
    }
    command
}

#[test]
fn help_version_and_invalid_args_do_not_start_mcp() {
    let cwd = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();
    for arg in ["--help", "-h", "help", "--version", "-v"] {
        let output = command(cwd.path(), home.path()).arg(arg).output().unwrap();
        assert!(output.status.success());
        assert!(String::from_utf8(output.stdout).unwrap().contains("airgap"));
    }
    let output = command(cwd.path(), home.path())
        .arg("unknown")
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
}

#[test]
fn setup_clean_directory_writes_all_configs_and_is_byte_identical_on_repeat() {
    let cwd = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();
    fs::write(
        cwd.path().join(".env"),
        "GMI_AGENTBOX_API_KEY=synthetic-compute\nGMI_MAAS_API_KEY=synthetic-inference\n",
    )
    .unwrap();
    let output = command(cwd.path(), home.path())
        .args(["setup", "--yes"])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(!String::from_utf8_lossy(&output.stdout).contains("synthetic-compute"));
    let paths = [
        cwd.path().join(".cursor/mcp.json"),
        cwd.path().join(".mcp.json"),
        cwd.path().join(".vscode/mcp.json"),
        cwd.path().join(".cursorrules"),
        cwd.path().join("CLAUDE.md"),
        home.path().join(".claude.json"),
    ];
    let before: Vec<_> = paths.iter().map(|path| fs::read(path).unwrap()).collect();
    let output = command(cwd.path(), home.path())
        .args(["init", "-y"])
        .output()
        .unwrap();
    assert!(output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains("no files changed"));
    for (path, original) in paths.iter().zip(before) {
        assert_eq!(fs::read(path).unwrap(), original);
    }
}

#[test]
fn setup_without_keys_succeeds_with_warning_and_preserves_rules() {
    let cwd = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();
    fs::write(
        cwd.path().join("CLAUDE.md"),
        "# My team rules\nPreserve this.\n",
    )
    .unwrap();
    let output = command(cwd.path(), home.path())
        .args(["setup", "--yes", "--claude"])
        .output()
        .unwrap();
    assert!(output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("not fully discovered"));
    assert!(fs::read_to_string(cwd.path().join("CLAUDE.md"))
        .unwrap()
        .starts_with("# My team rules\nPreserve this.\n"));
    assert!(!cwd.path().join(".cursor").exists());
}
