# Phase 25.1.16: Solver Supply-Chain/Security Gates

## Goal
Lock and enforce supply-chain/security requirements for vendored solver binaries used by release-grade proof flows.

## Lock File
- `docs/design/phase-25.1.16-solver-supply-chain.lock.json`

## Required Gates
1. Pinned solver identity
   - Solver family/version must remain pinned and aligned with `phase-25.1.4-solver-profile.lock.json`.
2. Bundle integrity
   - Each vendored solver bundle must provide SHA-256 checksum metadata and detached signature metadata.
3. License + notice inclusion
   - Distribution must include:
     - `docs/legal/third_party/z3-LICENSE.txt`
     - `docs/legal/third_party/z3-NOTICE.txt`
4. CVE policy
   - Follow `docs/security/solver-cve-update-policy.md` with bounded advisory age.
5. Rollback procedure
   - Follow `docs/security/solver-rollback-procedure.md` for revert-on-incident response.

## CI Enforcement
- CI proof regression gates must include:
  - `cargo test -p clg-cli --test phase25_solver_supply_chain_gate`
- Test validates lock schema, pinned-version alignment, required legal/security docs, and CI wiring.
