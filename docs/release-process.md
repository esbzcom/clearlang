# ClearLang Release Process (Current CLI)

This is the current production-style flow using existing commands.

## Scope
- Strict preflight and deterministic lock input
- Strict build with VC emission
- Signing + verification gates
- Publishable release artifacts

## Prerequisites
Project module root must include:
- `clg.package-metadata.json`
- `clg.package-abi.json`
- `clg.trust-policy.json`
- `clg.host-profile.json`

## 1) Generate Deterministic Lock Inputs
Use an explicit advisory time for replay-stable policy evaluation.

```powershell
clg pkg lock --generate --compiler-mode strict --advisory-as-of 2026-03-26T00:00:00Z --root examples/projects/generic
```

Outputs:
- `clg.lock.json`
- `clg.resolved-graph.json`
- `clg.resolved-graph.sha256`

## 2) Strict Build + VC Emission
Build Wasm and emit verification conditions.

```powershell
clg build examples/projects/generic/main.clear -o out/generic.wasm --emit-vcs out/generic.vc.json --compiler-mode strict --release-profile production --validate
```

Optional (recommended for Gate B workflows):

```powershell
clg build examples/projects/generic/main.clear -o out/generic.wasm --emit-vcs out/generic.vc.json --emit-proof out/generic.proof.json --compiler-mode strict --release-profile production --validate
```

## 3) Sign Release Artifacts
Sign module and proofs in one pass and emit assurance manifest.

```powershell
clg build examples/projects/generic/main.clear -o out/generic.wasm --emit-vcs out/generic.vc.json --emit-proof out/generic.proof.json --compiler-mode strict --release-profile production --validate --sign --key keys/signing.json --key-id release-2026q1 --scope both --sig-out out/generic.sig.json --assurance-manifest-out out/generic.assurance.json
```

## 4) Verify Before Publish
Run verification gate against the produced artifacts.

```powershell
clg verify --module out/generic.wasm --sig out/generic.sig.json --pubkey keys/public.json --verify-mode compile-time --trust-policy examples/projects/generic/clg.trust-policy.json --assurance-manifest out/generic.assurance.json --explain
```

If proof-artifact claims are present, include:

```powershell
clg verify --module out/generic.wasm --sig out/generic.sig.json --pubkey keys/public.json --assurance-manifest out/generic.assurance.json --proof-artifact out/generic.proof.json
```

Theorem-grade gate for release workflows:

```powershell
clg verify --module out/generic.wasm --sig out/generic.sig.json --pubkey keys/public.json --verify-mode compile-time --trust-policy examples/projects/generic/clg.trust-policy.json --assurance-manifest out/generic.assurance.json --require-assurance proved_all
```

Optional release policy gate:

```powershell
clg verify --module out/generic.wasm --sig out/generic.sig.json --pubkey keys/public.json --assurance-manifest out/generic.assurance.json --release-policy release-policy.json
```

## 5) Publish Bundle
Publish these files together:
- `out/generic.wasm`
- `out/generic.vc.json`
- `out/generic.sig.json`
- `out/generic.assurance.json`
- `out/generic.proof.json` (when `--emit-proof` was used)
- checksums/SBOM/release notes as needed by your distribution process

## Notes
- Milestone 3 policy lock: production release is `release == proved` (`proved_all` required). Non-proved outputs are dev/non-release only.
- `--release-profile production` enforces fail-closed theorem-grade gating at build/sign time.
- `--release-profile production` also enforces release-surface policy (`C122`): used `std::...` symbols must be listed as `proved` in `docs/proofs/proof-coverage-matrix.json`.
- `--release-profile production` enforces crypto proof-boundary policy (`C123`): any remaining `crypto.uninterpreted` boundary blocks release-grade/theorem-grade claims.
- Fail-closed release gate: block production release on any proof outcome `failed|unknown|timeout|assumed`.
- `--require-assurance proved_all` now also requires zero assumption boundaries in signed payloads/manifests (`unsigned.int_model`, `bitwise.uninterpreted`, `crypto.uninterpreted` are prohibited in release bundles).
- Signed assurance payloads include deterministic `proof_status` (`proved_all|not_proved_all`) for release-policy tooling.
- Milestone 3 CI/tag gate is locked by `docs/evidence/milestone_3-proof-gate.lock.json` and enforced by `crates/cli/tests/milestone3_release_gate.rs`.
- Milestone 3 cross-platform parity gate (`linux/windows/macos`) compares emitted proof-parity artifacts before `milestone_3` tag release gating.
- `--emit-proof` emits deterministic proof artifact summaries and binds optional proof/solver hashes into signed payload/manifest claims for verify-time consistency checks.
- A single wrapper command (`clg release`) is planned, but not implemented yet.
