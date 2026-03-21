# Phase 20.1.0 - Strict Host-Profile v0 Schema

## Goal
Define the minimal host-profile schema and capability set consumed by strict
preflight gates in Phase 20.1.

Scope for this bootstrap slice is intentionally narrow:
- direct capability presence checks only,
- deterministic capability identifiers,
- strict fail-closed behavior when required capabilities are absent.

## File and Scope
- Canonical filename: `clg.host-profile.json`.
- Location: module root for the strict build input.
- Applicability: `clg build --compiler-mode strict` preflight only.
- Non-goal: full production profile policy (tracked in `24.0.2`).

## Schema (v0)

```json
{
  "schema_version": 0,
  "profile": "contract_static",
  "capabilities": [
    "std::crypto::hash",
    "std::crypto::hmac",
    "std::crypto::verify",
    "std::env::chain_id",
    "std::wasi::print"
  ]
}
```

### Required fields
- `schema_version`: integer, must equal `0`.
- `profile`: string, one of:
  - `contract_static`
  - `shared_app`
- `capabilities`: array of capability ids (may be empty).

### Allowed capability ids (v0 bootstrap)
- `std::crypto::hash`
- `std::crypto::hmac`
- `std::crypto::verify`
- `std::env::time`
- `std::env::random`
- `std::env::chain_id`
- `std::wasi::print`

### Rejected in v0
- Unknown top-level keys.
- Unknown capability ids.
- Duplicate capability ids.
- Empty capability id strings.

## Capability Mapping (v0)
Strict preflight maps linked host imports to capability ids using this fixed map:

| Host import | Capability id |
|---|---|
| `clearlang_crypto::crypto_hash` | `std::crypto::hash` |
| `clearlang_crypto::crypto_hmac` | `std::crypto::hmac` |
| `clearlang_crypto::crypto_verify` | `std::crypto::verify` |
| `clearlang_env::env_time` | `std::env::time` |
| `clearlang_env::env_random` | `std::env::random` |
| `clearlang_env::env_chain_id` | `std::env::chain_id` |
| `std::wasi::print` | `std::wasi::print` |

Any required capability not present in `capabilities` fails closed.

## Determinism Rules
1. Internal evaluation order is deterministic:
   - sort `capabilities` lexicographically for evaluation.
2. Duplicate detection reports the lexicographically first conflicting id.
3. Capability checks are pure set-membership checks over:
   - strict preflight import requirements,
   - parsed host-profile capability set.

## Integration With 20.1 Gates
- Runtime capability gate (`20.1.2.6`): missing required capability fails with `C106`.
- Determinism gate (`20.1.2.7`): identical inputs produce identical capability-check outcomes and diagnostics ordering.

## Diagnostics Mapping (Design-Locked Proposal)
- Missing/unreadable host profile: `C106`.
- Malformed host-profile schema/version: `C106`.
- Missing required capability in selected profile: `C106`.

Final canonical code registration remains tracked under TODO `20.1.4.3`.

## Remediation (Bootstrap)
When strict preflight fails on host profile input:
1. ensure `<module-root>/clg.host-profile.json` exists and is readable,
2. use schema v0 exactly (`schema_version`, `profile`, `capabilities`),
3. include every required capability id from the v0 allowlist,
4. keep capability ids unique and deterministic (sorted in generated files).

Canonical minimal valid file:

```json
{
  "schema_version": 0,
  "profile": "contract_static",
  "capabilities": []
}
```

## Non-Goals
- Runtime QoS, SLO, or resource budgets.
- Dynamic host negotiation.
- `env_time`/`env_random` capability enablement (Phase 21 lock keeps strict-mode deny policy with `C106` even when capability ids are schema-allowed; expanded production profile policy is captured in `docs/runtime/host-profiles-production-policy.md`).
- Chain-specific capability taxonomies.
