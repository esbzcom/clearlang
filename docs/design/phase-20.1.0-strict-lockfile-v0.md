# Phase 20.1.0 - Strict Lockfile v0 Schema

## Goal
Define the minimal strict-mode lockfile schema required for Phase 20.1 preflight gates.

Scope for this slice is intentionally small:
- direct dependencies only,
- exact pins only (`name`, `version`, `digest`),
- deterministic parsing/validation behavior.

## File and Scope
- Canonical filename: `clg.lock.json`.
- Location: module root for the current build input.
- Applicability: `clg build --compiler-mode strict` preflight only.
- Non-goal: lockfile generation/update workflow (tracked in Phase 22.0.2).

## Schema (v0)

```json
{
  "schema_version": 0,
  "dependencies": [
    {
      "name": "std::core",
      "version": "1.0.0",
      "digest": "sha256:9f86d081884c7d659a2feaa0c55ad015a3bf4f1b2b0b822cd15d6c15b0f00a08"
    }
  ]
}
```

### Required fields
- `schema_version`: integer, must equal `0`.
- `dependencies`: array of dependency pins (may be empty).
- Per dependency pin:
  - `name`: string, package identifier.
  - `version`: string, exact `MAJOR.MINOR.PATCH` semver.
  - `digest`: string, exact `sha256:<64 lowercase hex>`.

### Rejected in v0
- Unknown top-level keys.
- Unknown dependency keys.
- Missing required keys.
- Duplicate dependency names.
- Version ranges (`^1.2.3`, `~1.2.3`, `>=...`) or pre-release selectors.
- Non-`sha256` digests.

## Determinism Rules
1. Dependency uniqueness is keyed by `name`.
2. Runtime/compile preflight behavior must be independent of input order:
   - parser sorts dependencies by `name` for internal evaluation,
   - duplicate detection is deterministic and reports the lexicographically first conflicting name.
3. Canonical serialization for replay checks uses:
   - sorted dependencies by `name`,
   - exact input-preserving `version` and `digest` values after validation.

## Integration With 20.1 Gates
- Source-of-truth gate (`20.1.2.1`): strict mode must read `clg.lock.json`; missing file fails closed.
- Artifact identity gate (`20.1.2.2`): resolved artifact must match lockfile `(name, version, digest)` exactly.
- Determinism gate (`20.1.2.7`): identical lockfile content yields identical direct-dependency import map + diagnostics ordering.

## Diagnostics Mapping (Design-Locked Proposal)
- Missing/unreadable strict lockfile: `C101`.
- Malformed lockfile schema/version: `C104`.
- Invalid digest format or pin mismatch: `C102`.

Final canonical code registration remains tracked under TODO `20.1.4.3`.

## Non-Goals
- Transitive dependency pins.
- Semver solver behavior.
- Signature/trust-policy schemas.
- Host-profile capability schemas.
