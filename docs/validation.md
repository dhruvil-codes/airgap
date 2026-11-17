# Validation record

Recorded on 2026-11-17 with Rust 1.99.1 on Windows x86_64 GNU. These results describe one development environment and one GMI account. They are not a platform certification or a provider-isolation guarantee.

## Automated checks

- `cargo fmt --check`, `cargo check --locked`, and all-target Clippy with warnings denied passed.
- `cargo test --locked --all-targets` passed 31 tests: 14 provider-mock tests, two real stdio tests, and 15 scanner/configuration/receipt tests.
- `cargo build --release --locked` produced a stripped native Windows executable.
- Default tests use local mocks and a child process. They require no GMI keys and create no billable resources.

The [initial GitHub CI run](https://github.com/dhruvil-codes/airgap/actions/runs/44098293818) passed on commit `74fd005`: native tests and release builds on Linux, macOS and Windows, Rust 1.89 all-target checks, static Linux musl compilation, and formatting/lint/docs/packaging checks. These are offline checks, not additional live provider tests. Consult [GitHub Actions](https://github.com/dhruvil-codes/airgap/actions) for the current commit's results.

## Release measurements

The PowerShell probe launched 10 fresh release processes and measured 100 warmed `tools/list` requests without cloud calls.

- Executable size: 6,033,408 bytes.
- Median working set after protocol requests: 6,932,480 bytes.
- Median startup through the initialize response: 27.22 ms; first launch: 73.64 ms.
- Median warmed `tools/list` roundtrip: 1.06 ms, including PowerShell/.NET I/O and host JSON parsing.

These are idle protocol measurements, not peak memory under uploaded snapshots or concurrent jobs. The size and idle-memory targets were met in this run. The original startup and sub-millisecond roundtrip targets were not met. Do not interpret the host-side probe as isolated server processing time.

Reproduce the probe after a release build:

```powershell
./scripts/measure.ps1
```

On a memory-constrained Windows machine, use `CARGO_PROFILE_DEV_DEBUG=0` and `-j 1` for development tests. Building a Linux static artifact additionally requires the musl target and linker described in [the reference](reference.md).

## Live GMI checks

A maintainer-authorized smoke test used dedicated compute and inference keys with synthetic data. Credentials were not copied into the project or stored in evidence. All inference calls used `deepseek-ai/DeepSeek-V4.1-Flash`.

Sandbox execution passed on `docker.io/library/alpine:3.20`: a fixture uploaded, ran remotely, returned exit 0, and had its synthetic API key redacted from stdout. Deletion succeeded.

Injection classification passed for both benign build instructions and a hidden instruction override. Testing exposed an overly broad use of the findings array. The prompt now requires an empty array for clean content; fail-closed consistency checks remain enforced and covered by a regression test.

The initial repository audit misclassified ordinary hardening advice as a vulnerability. The prompt now requires a source-supported security impact and reachable untrusted entry point leading to a dangerous sink or violated boundary. One recheck failed closed on a model schema mismatch. The final retry completed six calls and returned zero vulnerabilities and 29 rejected hypotheses for the harmless fixture, with provider-reported token usage and a receipt. This small sample does not establish general audit accuracy.

**Live patch verification remains blocked.** `POST /tasks/:id/executions` consistently returned HTTP 403 for the verification request. Separate Python-image diagnostics launched a sandbox, uploaded files and executed ordinary commands. The precise cause of the denial remains unknown. No transport or encoding workaround was attempted. Its client mechanics are covered by mocks; live end-to-end verification is not claimed to work on that account.

The final resource check found zero active sandboxes and no remaining `airgap-*` templates. Existing unrelated resources were left untouched. A scan found no supplied API key values in source, documentation, scripts or evidence.

Raw development probes and logs stay in the ignored `.artifacts/` directory and are not distributed. None of these checks proves provider isolation or a specific CVE mitigation. Client-specific Cursor and Claude integration remains untested live.
