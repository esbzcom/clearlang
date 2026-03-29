# Phase 25.1.1 - Gate B Design Lock (Proof Engine + Solver Closure)

## Status
Design lock for `25.1.1` in `docs/TODO.md`.

## Goal
Define Gate B policy and deterministic inputs so solver integration can land without changing the `release == proved` contract.

## Design Principles Check
- Simple for users: release behavior stays unchanged (`release == proved`), while internals move from placeholders to solver-backed outcomes.
- AI-friendly: proof/solver artifacts use explicit schemas, versions, and deterministic hashes.
- Provably correct: production release remains fail-closed unless all release-enabled obligations are solver-closed.
- Crypto-focused: unresolved crypto proof boundaries remain release blockers until explicit model closure lands.

## Locked Policy (Gate B)
1. Gate B does not change Gate A semantics:
   - release requires theorem-grade status (`proved_all`),
   - release blocks on any non-closure evidence.
2. Solver-era per-VC status vocabulary is locked to:
   - `proved`, `failed`, `unknown`, `timeout`.
3. Transitional placeholder `generated` may exist in non-release/dev flows during implementation, but is release-invalid.
4. Production release gate remains fail-closed if any VC is not `proved`.

## Deterministic Inputs (Locked)
Gate-B decisions must be deterministic over:
1. source module graph,
2. strict dependency inputs (`clg.lock.json`, package metadata/ABI),
3. trust and host profiles,
4. solver profile lock (version/options/timeouts),
5. VC artifact and proof artifact schema versions.

## Exit Criteria (Gate B)
1. Solver profile lock is machine-readable and hashed into signed assurance claims.
2. `--emit-proof` artifact schema and canonical serialization are locked and implemented.
3. `clg verify` can validate proof-artifact consistency against signed claims.
4. Deterministic replay gates pass for identical inputs (single-platform and cross-platform parity policy where applicable).
5. Release-enabled surfaces have zero unresolved assumption boundaries.

## Non-Goals
1. No release UX wrapper command (`clg release`) in this slice.
2. No std-surface expansion policy changes (tracked in Phase 26).
3. No theorem language keyword additions.

## Verified Current Gap Snapshot
1. VC emission exists (`--emit-vcs`) but dedicated proof artifact emission is not yet wired.
2. Signed payloads currently bind module/proofs hashes but not solver-profile/proof-artifact hashes.
3. Diagnostics table has no dedicated solver-era execution/artifact mismatch codes before this slice.
4. Counterexample envelopes are present, but solver execution path is still placeholder-backed.

## References
- `docs/TODO.md`
- `docs/design/phase-25.0.1-milestone-3-design-lock.md`
- `docs/design/phase-25.0.3-release-equals-proved-policy.md`
- `docs/design/phase-25.1.4-deterministic-solver-profile-lock.md`
- `docs/proofs/vc-schema.md`
