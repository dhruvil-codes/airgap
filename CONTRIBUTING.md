# Contributing to airgap

Bug reports, focused fixes, documentation improvements and regression tests are welcome. For vulnerabilities or credential exposure, follow [SECURITY.md](SECURITY.md) instead of opening a public issue.

## Set up

You need Rust 1.89 or newer, Cargo and a native linker. The server does not need local Docker or Python. Default tests need no cloud account.

```sh
git clone https://github.com/dhruvil-codes/airgap.git
cd airgap
git switch -c fix/describe-the-change
cargo test --locked --all-targets
```

Read the [architecture](docs/architecture.md) before changing protocol or provider behavior. Use the [tool reference](docs/reference.md) to check limits and public inputs.

## Make a change

- Keep a PR focused on one problem. Discuss larger protocol or provider changes in an issue first.
- Include a regression test for a bug or a new trust-boundary behavior. Use local HTTP mocks for provider behavior.
- Keep stdout reserved for MCP. Diagnostics must never include credentials, raw provider error bodies or submitted snapshots.
- Preserve input limits, deadlines, cleanup and fail-closed behavior. A missing exit code or invalid model response must not become a successful result.
- Keep Cargo.lock in version control. Use Cargo to update it rather than editing it by hand.
- Update public documentation when tool inputs, defaults, configuration or limitations change.

Do not include real credentials, private repositories, customer logs or live sandbox access tokens in issues, commits or fixtures. Use synthetic examples. See [.env.example](.env.example) for configuration names.

## Run checks

```sh
cargo fmt --check
cargo check --locked
cargo test --locked --all-targets
cargo clippy --locked --all-targets -- -D warnings
cargo build --release --locked
cargo package --list --allow-dirty --locked
```

CI is configured to run stable-toolchain checks on Linux, macOS and Windows, plus an MSRV check on Rust 1.89. Workflow success still needs to be observed on GitHub; configuration alone is not evidence that a platform works.

If Windows compilation fails under memory pressure, use one build job and omit development debug symbols for that shell session:

```powershell
$env:CARGO_PROFILE_DEV_DEBUG = '0'
cargo test --locked --all-targets -j 1
```

The packaging script requires PowerShell, which is already available on Windows and is used by the hosted release workflow. It is a maintainer tool, not a server runtime dependency. Validate changes to it with `./scripts/test-package-release.ps1`.

## Live provider testing

Offline tests are the default. Never add account credentials to CI or make default tests create billable resources. Live checks require explicit authorization from the account owner, small synthetic fixtures, bounded lifetimes and confirmed teardown. Record the date, model, image and outcome without tokens or private payloads.

Report provider denials accurately. Do not work around an HTTP 403 by hiding commands or switching transports to evade the rejection. The current live patch-verification limitation is documented in [the validation record](docs/validation.md).

## Open a pull request

Explain the problem, the resulting behavior and the checks you ran. Include a concrete before/after example when useful. State untested behavior or compatibility limits. Screenshots are unnecessary for protocol-only changes; use request/response examples with synthetic data.

Review happens on a best-effort basis. Contributors retain copyright in their work. By submitting a contribution, you agree to license it under the project's MIT OR Apache-2.0 terms. No CLA is required.
