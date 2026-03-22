# Phase 23.0 - Runtime Loader + Linker Design Lock

## Status
Design lock target for `23.0.0` in `docs/TODO.md`.

## Goal
Lock runtime package loading/linking policy before implementation so host/runtime behavior is deterministic, fail-closed, and production-auditable.

## Scope
- Phase 23 covers runtime artifact loading/linking and runtime trust enforcement.
- Phase 22 remains compile-time/package-resolution source generation.
- Phase 24 remains profile conformance, go-live, and release governance.

## Runtime Source of Truth (Locked)
1. `clg.lock.json` (resolved package pins + digests).
2. Runtime import-binding artifact (`clg.runtime-link.json`) + hash companion (`clg.runtime-link.sha256`) consumed by runtime loader/linker.
3. Trust policy + host profile inputs.
4. Trusted package store index and resolved artifact paths (`clg.package-store-index.json` + optional `clg.runtime-loader.json` policy).

Note:
- `clg.resolved-graph.json` remains compile-time resolver evidence for Phase 22/CI replay gates.
- Runtime loader decisions are derived from `clg.runtime-link.json` + lock/trust/store/profile inputs.

No implicit network fetch by default:
- default runtime loader policy is trusted local store/index only,
- any remote/mirror fetch path must be explicit policy configuration and preserve fail-closed verification gates.

Artifact locator precedence (deterministic, locked):
1. Resolve by `id` + `digest` from trusted local package store/index.
2. If remote/mirror mode is explicitly enabled, consult configured mirrors in declared order.
3. On first verified match (digest/signature/trust gates), stop resolution.
4. If no verified artifact is found, fail with runtime missing-artifact diagnostic (no best-effort fallback).

## Runtime Resolution and Linking Policy (Locked)
1. Loader resolves package artifacts from the trusted source-of-truth set only.
2. Linker binds imports strictly from canonical runtime import-binding artifact + package ABI contract.
3. Resolution/link ordering is deterministic:
   - package selection order by package id lexical ascending,
   - symbol binding order by `(module, symbol)` lexical ascending,
   - diagnostics order by `(code, package id, symbol)` lexical ascending.

## Runtime Import-Binding Artifact Contract (Locked)
Filename:
- `clg.runtime-link.json`

Hash companion:
- `clg.runtime-link.sha256` containing lowercase hex sha256 of canonical JSON bytes.

Schema (v0):
- `schema_version`: `0`
- `resolver_version`: `u32` copied from `clg.lock.json` resolver version
- `packages`: deterministic list of runtime-load candidates with:
  - `id` (`name@version`)
  - `digest`
  - `artifact_path`
  - `abi_id`
- `bindings`: deterministic import-binding entries with:
  - `import_module`
  - `import_name`
  - `provider_package_id`
  - `provider_symbol`

Type lock:
- all string fields are UTF-8 JSON strings with no additional normalization.
- `schema_version`/`resolver_version` are JSON integers.

Serialization/ordering lock:
1. Canonical JSON with stable key ordering.
2. `packages` sorted by `id` lexical ascending.
3. `bindings` sorted by `(import_module, import_name, provider_package_id, provider_symbol)` lexical ascending.
4. Hash computed from canonical JSON bytes, newline-free.

## Runtime Trust Gates (Fail-Closed)
1. Artifact identity gate: runtime artifact digest must match locked digest.
2. Signature/trust gate: signer/signature/trust policy checks must pass for every loaded package artifact.
3. Contract gate: runtime-link bindings must remain deterministic and internally consistent.
4. Profile gate: required host capabilities derived from active app-module imports and runtime-linked provider-package imports must be present in active host profile.
5. Drift gate: runtime import-binding artifact/hash must match compile-time emitted artifacts.

Any gate failure aborts loading/linking before user entrypoint execution.

## Runtime Signature Time Semantics (Locked)
To prevent host-clock nondeterminism at runtime:
1. Runtime trust validation MUST use signature envelope `signed_at` as the signer-window anchor.
2. Validity rule: `not_before <= signed_at < not_after`.
3. Runtime loader/linker MUST NOT use ambient wall-clock time for signer-window acceptance.
4. Runtime trust policy revocation/compromise lists are treated as explicit input state (deterministic by provided files), not time-derived at runtime.

## Diagnostics Reservation Plan (Phase 23)
Reserve dedicated runtime-loader diagnostics before implementation (stable JSON contracts):
- `R012`: runtime package artifact missing/unavailable,
- `R013`: runtime package digest mismatch,
- `R014`: runtime package signature/trust policy verification failure,
- `R015`: runtime package ABI/link binding mismatch,
- `R016`: runtime loader host capability/profile mismatch,
- `R017`: runtime loader determinism replay mismatch.

Code IDs are intentionally reserved by plan first, then registered in `docs/diagnostics.md` with tests.

## Determinism Replay Contract
For identical inputs:
- lockfile/resolved graph/import-binding artifact,
- trust policy/host profile,
- package store contents,
- runtime loader configuration,

the loader/linker must produce:
1. identical loaded package set and binding map,
2. identical diagnostics ordering and payloads on failure,
3. identical runtime-link evidence artifact bytes/hash (if emitted).

## CI Acceptance Matrix (Phase 23 completion target)
1. Positive runtime load/link fixture.
2. Missing artifact fail-closed fixture.
3. Digest tamper fail-closed fixture.
4. Signature/trust tamper fail-closed fixture.
5. ABI mismatch fail-closed fixture.
6. Host capability mismatch fail-closed fixture.
7. Replay determinism fixture (`run A == run B` artifacts/diagnostics).

## Rollout and Operations Lock
1. Canary rollout with explicit enable flag and rollback switch.
2. Runbook coverage for cache/mirror/offline modes and outage behavior.
3. No production enablement without green CI acceptance matrix and rollback drill evidence.

Operational runbook implementation for 23.0.4:
- `docs/runtime/runtime-loader-resilience-runbook.md`

## References
- `docs/TODO.md` (`23.0`, `23.1`, `24.2.5`)
- `docs/design/phase-20.0-std-packaging-runtime-linking.md`
- `docs/design/phase-22.0.3-lockfile-v1.md`
- `docs/design/phase-22.1.0-resolver-semver-determinism.md`
