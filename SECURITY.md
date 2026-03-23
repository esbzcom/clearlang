# Security Policy

## Branch and Release Model

- `main` is the only long-lived production branch.
- Security fixes are merged to `main` via pull request (no direct pushes).
- Releases are created from signed/versioned tags on `main`.

## Supported Versions

Security fixes are applied to the `main` branch.

## Reporting a Vulnerability

Please report vulnerabilities through GitHub Security Advisories (private reporting), not public issues.

If private reporting is unavailable, open an issue with minimal detail and request a private follow-up channel.

## Response Expectations

- Initial acknowledgment target: within 72 hours.
- Triage and severity assessment target: within 7 days.
- Fix and disclosure timeline depends on severity and exploitability.
- Critical fixes may be delivered as fast-follow `hotfix/*` branches merged back to `main`.

## Security Gate Expectations

For security-sensitive changes, maintainers should require:

- Pull request review approval.
- Passing required CI checks on `main`.
- Up-to-date branch before merge.

## Disclosure Policy

Please avoid public disclosure until a fix or mitigation is available and maintainers confirm coordinated disclosure timing.
