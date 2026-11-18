# Release process

Releases are prepared by a maintainer. The workflow builds candidates and checksums; it does not publish a GitHub release or a crates.io package.

## Prepare

1. Check the current [validation limits](validation.md), including the live patch-verification denial. Do not describe an untested platform or isolation property as verified.
2. Update the version in Cargo.toml and regenerate Cargo.lock with Cargo. Move the relevant Unreleased changelog entries into a dated version section.
3. Run the checks in [CONTRIBUTING.md](../CONTRIBUTING.md), the packaging self-check, and `cargo package --allow-dirty --locked --no-verify`. Inspect the package contents before publishing.
4. Merge or push the reviewed change to the default branch and wait for CI to pass.

## Build candidates

Create and push a version tag matching Cargo.toml, for example `v0.1.0`. The release-candidates workflow also supports manual runs from a selected ref.

It builds these targets:

| Target | Runner | Archive |
| --- | --- | --- |
| `x86_64-unknown-linux-musl` | Ubuntu | `.tar.gz` |
| `aarch64-unknown-linux-musl` | Ubuntu ARM64 | `.tar.gz` |
| `x86_64-pc-windows-msvc` | Windows | `.zip` |
| `aarch64-pc-windows-msvc` | Windows ARM64 | `.zip` |
| `aarch64-apple-darwin` | macOS | `.tar.gz` |
| `x86_64-apple-darwin` | macOS Intel | `.tar.gz` |

These are configured build targets, not a claim that every runtime has been tested. Archives contain only the executable and both license texts. Each archive has a separate `.sha256` file. CI and candidate jobs use no GMI keys.

Windows candidates link the C runtime statically to avoid requiring a separate Visual C++ runtime installation. Linux candidates use musl. System libraries supplied by the operating system are still required.

For a local build, install the desired Rust target and linker, then run:

```powershell
cargo build --release --locked --target x86_64-pc-windows-msvc
./scripts/package-release.ps1 -Target x86_64-pc-windows-msvc -ExpectedTag v0.1.0
```

The script reads the version from Cargo metadata. It refuses a mismatched tag or an existing archive. Use an empty output directory for another candidate build. It requires Cargo and PowerShell, plus `tar` and `chmod` for Unix archives on Unix hosts.

## Verify and publish

The one-line installers use the latest published GitHub release. Workflow artifacts and draft releases alone do not enable installation. Check `--version`, `setup --yes`, and a stdio roundtrip on candidate binaries before publishing.

Download the candidate archives and checksum files from the workflow run. Verify checksums before extracting, and run a harmless MCP initialize/list/local-redaction check on each target machine you claim to support. A hash checks integrity; it is not a signature or an attestation.

On a Unix shell, with the checksum and archive in the same directory:

```sh
sha256sum -c airgap-v0.1.0-x86_64-unknown-linux-musl.tar.gz.sha256
```

macOS can use `shasum -a 256 -c`. On Windows, use `Get-FileHash -Algorithm SHA256` and compare it to the corresponding checksum file.

Prepare release notes with changes, upgrade guidance, tested platforms, and known failures. Create a draft release through GitHub or `gh release create --draft`, attach only the checked archives and checksums, review the draft, then publish it. Do not upload `.env` files, private logs or the ignored `.artifacts/` directory.

Publishing to crates.io is a separate maintainer decision. Confirm crate-name ownership, licensing, repository metadata and package contents before running `cargo publish`. No registry credentials are stored in CI.

## Repository settings

After the first push, set `main` as the default branch. Enable private vulnerability reporting and require the CI checks before merging changes. Protect the branch against force pushes and deletion. Keep Actions token permissions read-only by default. Repository settings and publishing permissions are managed separately from these files.
