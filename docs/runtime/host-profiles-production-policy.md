# Host Profile Production Policy (Phase 24.0.2)

## Goal
Expand the Phase 20.1 host-profile v0 bootstrap into an explicit production
policy for the two locked profiles:
- `contract_static`
- `shared_app`

This policy covers both strict build-time gates and runtime loader behavior.

## Source of Truth
1. `clg.host-profile.json` (schema v0 input, loaded by strict build and runtime loader).
2. `docs/design/phase-21.0-host-capability-policy.lock.json` (canonical strict capability policy matrix).
3. `clg.runtime-link.json` + `clg.runtime-link.sha256` (runtime package loading contract in production profiles).

## Profile Identity
- Accepted production profile ids: `contract_static`, `shared_app`.
- Any unsupported profile id is rejected deterministically:
  - strict build preflight: `C106`
  - runtime loader: `R016`

## Strict Build Policy (`clg build --compiler-mode strict`)

### Capability Matrix (locked)
| Capability | `contract_static` | `shared_app` | Failure code |
|---|---|---|---|
| `std::crypto::hash` | allow | allow | `C106` when required but missing |
| `std::crypto::hmac` | allow | allow | `C106` when required but missing |
| `std::crypto::verify` | allow | allow | `C106` when required but missing |
| `std::env::chain_id` | allow | allow | `C106` when required but missing |
| `std::wasi::print` | allow | allow | `C106` when required but missing |
| `std::env::time` | deny | deny | `C106` (strict deterministic deny) |
| `std::env::random` | deny | deny | `C106` (strict deterministic deny) |

### Additional strict host-profile gates
- Missing `clg.host-profile.json` in strict mode: `C106`.
- Malformed schema/unsupported fields/version/capability ids: `C106`.
- Capability absence for required linked import capability: `C106`.

## Runtime Loader Policy (`clg run` and host integrations)
- If `clg.host-profile.json` is absent, runtime uses local/dev path semantics.
- If `clg.host-profile.json` is present and profile is `contract_static` or `shared_app`:
  - runtime-link artifacts are mandatory (`clg.runtime-link.json` + hash),
  - loader is fail-closed on missing/mismatch/untrusted artifacts with `R012`,
  - runtime host capability requirements are derived from active app-module imports and runtime-linked provider package imports; missing required capabilities fail with `R016`,
  - permissive fallback to manual/implicit runtime linking is not allowed.
- Malformed host-profile runtime input also fails with `R016`.

## Determinism Requirements
- Strict build host-profile capability sets are sorted before evaluation.
- Strict gate outcomes for identical inputs are replay-stable with deterministic diagnostics ordering (`C106` under host-capability failures).
- Runtime loader production-profile decisions are replay-stable for identical runtime-link/trust inputs (`R012`/`R016` paths).

## Operational Guidance
- Use `xtask host-capability-policy-artifact` to emit the machine-readable policy artifact and digest that must match the Phase 21 lock.
- Keep `clg.host-profile.json` checked in for production deployments and review it together with lockfile/trust-policy updates.
- Treat profile changes (`contract_static` <-> `shared_app`) as release-gated changes because they alter strict/runtime acceptance behavior.

## Certification Suite (Phase 24.0.3)
The host conformance certification suite is pinned in:
- `crates/cli/tests/host_conformance_certification.rs`

Locked coverage:
- strict profile matrix enforces deterministic deny of `std::env::{time,random}` (`C106`),
- strict profile matrix accepts allowed crypto capability paths,
- runtime fail-closed behavior for production profiles without runtime-link (`R012`),
- runtime rejection of unsupported profile identifiers (`R016`),
- runtime rejection when required host capabilities are missing (`R016`).
