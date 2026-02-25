Verified profile regression fixtures for Phase 19.3.3.

These fixtures are expected to remain within the verified std/core subset profile:
- no assumption boundaries in emitted VCs,
- assurance tier at least `L1`,
- strict profile build (`--compiler-mode strict --emit-vcs`) must succeed.

The CI/test manifest for these fixtures is `docs/proofs/verified-profile-fixtures.json`.
