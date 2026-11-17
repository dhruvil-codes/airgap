# Security policy

## Report privately

Email [dhruvil@bydhruvil.in](mailto:dhruvil@bydhruvil.in) with the subject `airgap security report`. Do not put vulnerabilities, API keys or private logs in public issues. If GitHub private vulnerability reporting is enabled for this repository, you may use that channel instead.

Include the affected version or commit, expected and observed behavior, security impact, and a minimal reproduction using synthetic data. Test only systems and cloud resources you are authorized to use. Never send a working API key or a live sandbox access token.

Reports are reviewed on a best-effort basis. There is no guaranteed response time or paid support contract. Coordinate public disclosure with the maintainer so users can receive a fix and guidance first.

If you accidentally expose a credential, revoke or rotate it immediately. Redacting or deleting the public text does not make an exposed key safe to reuse.

## Supported versions

The project is in early development. Before the first release, fixes target the current default branch. Once releases exist, fixes target the latest release; older versions have no promised backports. Check [CHANGELOG.md](CHANGELOG.md) for release status.

## Scope and limits

Relevant reports include host-side command execution, implicit local file disclosure, credential leakage, path traversal, unbounded resource consumption, false successful verification, and cleanup failures in airgap's own code.

Model misclassification and heuristic scanner misses are known limitations. A concrete case is still useful, but a model verdict is not a security boundary or a certification. Receipts are unsigned hashes, not provider attestations.

GMI infrastructure, sandbox isolation and account permissions are operated by GMI Cloud. Report infrastructure issues to the provider as well, without including credentials. See [the security model](docs/security-model.md) for the boundary this project actually implements.
