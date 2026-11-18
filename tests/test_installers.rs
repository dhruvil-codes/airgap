#[cfg(unix)]
mod unix {
    use flate2::{write::GzEncoder, Compression};
    use sha2::{Digest, Sha256};
    use std::{fs, io::Cursor, os::unix::fs::PermissionsExt, process::Command};

    fn fixture(path: &std::path::Path, symlink: bool) {
        let script=format!("#!/bin/bash\ncase \"$1\" in\n--version) printf 'airgap {}\\n';;\nsetup) [ \"$2\" = --yes ] || exit 4; printf 'setup\\n' >> \"$AIRGAP_TEST_RECORD\";;\n*) exit 3;;\nesac\n",env!("CARGO_PKG_VERSION"));
        let mut tar = tar::Builder::new(GzEncoder::new(Vec::new(), Compression::fast()));
        let mut header = tar::Header::new_gnu();
        header.set_mode(0o755);
        if symlink {
            header.set_entry_type(tar::EntryType::Symlink);
            header.set_size(0);
            header.set_link_name("/not-an-executable").unwrap();
        } else {
            header.set_size(script.len() as u64);
        }
        header.set_cksum();
        tar.append_data(
            &mut header,
            "airgap",
            Cursor::new(if symlink { &b""[..] } else { script.as_bytes() }),
        )
        .unwrap();
        fs::write(path, tar.into_inner().unwrap().finish().unwrap()).unwrap();
    }

    #[test]
    fn shell_installer_verifies_before_replacing_and_calls_setup_noninteractively() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path();
        fs::create_dir(root.join("mock")).unwrap();
        fs::create_dir(root.join("bin")).unwrap();
        fs::create_dir(root.join("home")).unwrap();
        let curl = root.join("mock/curl");
        fs::write(
            &curl,
            r#"#!/bin/bash
out=''; url=''
while [ "$#" -gt 0 ]; do
  case "$1" in
    --output) out=$2; shift 2;;
    https://*) url=$1; shift;;
    *) shift;;
  esac
done
case "$url" in
  */latest)
    [ "$AIRGAP_TEST_MODE" != missing ] || exit 22
    printf 'https://github.com/dhruvil-codes/airgap/releases/tag/%s' "$AIRGAP_TEST_TAG";;
  *.sha256) cp "$AIRGAP_TEST_FIXTURE/checksum" "$out";;
  *.tar.gz) cp "$AIRGAP_TEST_FIXTURE/archive.tar.gz" "$out";;
  *) exit 9;;
esac
"#,
        )
        .unwrap();
        fs::set_permissions(&curl, fs::Permissions::from_mode(0o755)).unwrap();
        let target = if std::env::consts::OS == "macos" {
            "apple-darwin"
        } else {
            "unknown-linux-musl"
        };
        let arch = if std::env::consts::ARCH == "aarch64" {
            "aarch64"
        } else {
            "x86_64"
        };
        let name = format!(
            "airgap-v{}-{arch}-{target}.tar.gz",
            env!("CARGO_PKG_VERSION")
        );
        let run = |mode: &str| {
            Command::new("/bin/bash")
                .arg(format!("{}/scripts/install.sh", env!("CARGO_MANIFEST_DIR")))
                .current_dir(root)
                .env_clear()
                .env(
                    "PATH",
                    format!(
                        "{}:{}",
                        root.join("mock").display(),
                        std::env::var("PATH").unwrap_or_default()
                    ),
                )
                .env("HOME", root.join("home"))
                .env("SHELL", "/bin/bash")
                .env("TMPDIR", root)
                .env("AIRGAP_INSTALL_DIR", root.join("bin"))
                .env("AIRGAP_NO_PATH_UPDATE", "1")
                .env("AIRGAP_TEST_MODE", mode)
                .env("AIRGAP_TEST_TAG", format!("v{}", env!("CARGO_PKG_VERSION")))
                .env("AIRGAP_TEST_RECORD", root.join("record"))
                .env("AIRGAP_TEST_FIXTURE", root)
                .output()
                .unwrap()
        };
        fs::write(root.join("bin/airgap"), "existing-install").unwrap();
        assert!(!run("missing").status.success());
        fixture(&root.join("archive.tar.gz"), false);
        fs::write(
            root.join("checksum"),
            format!("{}  {name}\n", "0".repeat(64)),
        )
        .unwrap();
        let bad = run("bad");
        assert!(!bad.status.success());
        assert!(String::from_utf8_lossy(&bad.stderr).contains("Checksum mismatch"));
        assert_eq!(
            fs::read_to_string(root.join("bin/airgap")).unwrap(),
            "existing-install"
        );
        let hash = Sha256::digest(fs::read(root.join("archive.tar.gz")).unwrap());
        let hash = hash
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>();
        fs::write(root.join("checksum"), format!("{hash}  {name}\n")).unwrap();
        let good = run("good");
        assert!(
            good.status.success(),
            "{}",
            String::from_utf8_lossy(&good.stderr)
        );
        assert_eq!(fs::read_to_string(root.join("record")).unwrap(), "setup\n");
        assert!(run("good").status.success());
        assert_eq!(
            fs::read_to_string(root.join("record")).unwrap(),
            "setup\nsetup\n"
        );
        let installed = fs::read(root.join("bin/airgap")).unwrap();
        fixture(&root.join("archive.tar.gz"), true);
        let hash = Sha256::digest(fs::read(root.join("archive.tar.gz")).unwrap());
        let hash = hash
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>();
        fs::write(root.join("checksum"), format!("{hash}  {name}\n")).unwrap();
        assert!(!run("good").status.success());
        assert_eq!(fs::read(root.join("bin/airgap")).unwrap(), installed);
    }
}

#[cfg(windows)]
#[test]
fn powershell_installer_checks_hash_and_archive_and_runs_real_setup() {
    let temp = tempfile::tempdir().unwrap();
    let shell = std::path::PathBuf::from(std::env::var_os("SystemRoot").unwrap())
        .join("System32/WindowsPowerShell/v1.0/powershell.exe");
    let output = std::process::Command::new(shell)
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-ExecutionPolicy",
            "Bypass",
            "-File",
        ])
        .arg(format!(
            "{}/tests/fixtures/install_windows.ps1",
            env!("CARGO_MANIFEST_DIR")
        ))
        .arg("-Installer")
        .arg(format!(
            "{}/scripts/install.ps1",
            env!("CARGO_MANIFEST_DIR")
        ))
        .arg("-Binary")
        .arg(env!("CARGO_BIN_EXE_airgap"))
        .current_dir(temp.path())
        .env_clear()
        .env("SystemRoot", std::env::var_os("SystemRoot").unwrap())
        .env("PATH", std::env::var_os("PATH").unwrap_or_default())
        .env("PATHEXT", ".COM;.EXE;.BAT;.CMD")
        .env("USERPROFILE", temp.path().join("home"))
        .env("HOME", temp.path().join("home"))
        .env("TEMP", temp.path())
        .env("TMP", temp.path())
        .env("AIRGAP_INSTALL_TEST_ROOT", temp.path())
        .env("AIRGAP_INSTALL_DIR", temp.path().join("bin"))
        .env("AIRGAP_NO_PATH_UPDATE", "1")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}
