# Phase 22.0.6 - Phase 22 Diagnostics Reservation

## Status
Design lock target for `22.0.6` in `docs/TODO.md`.

## Goal
Reserve deterministic diagnostics for Phase 22 package trust/resolution/advisory flows before implementation.

## Reservation Range
- Build-stage diagnostic range reserved for Phase 22: `C109`-`C119`.
- Codes remain 4-character and machine-stable.

## Reserved Code Matrix
| Code | Scope | Trigger (design lock) |
|---|---|---|
| `C109` | metadata model | Conflicting or unsupported package metadata model (including legacy/canonical coexistence conflict). |
| `C110` | metadata trust | Metadata trust-anchor/signature linkage failure against trust policy. |
| `C111` | lockfile workflow | Lockfile generate/update input contract failure (invalid roots/policy constraints). |
| `C112` | transitive resolver | Transitive dependency cycle detected. |
| `C113` | semver solver | No satisfiable version set for dependency constraints. |
| `C114` | semver determinism | Candidate selection cannot be made deterministic under locked tie-break rules. |
| `C115` | advisory deny | Resolution selected a package version denied by advisory policy. |
| `C116` | advisory forced-upgrade | Advisory requires upgrade but no compliant version is resolvable. |
| `C117` | advisory trust | Advisory input is missing/invalid/untrusted in strict mode. |
| `C118` | build/run parity | Build and run package resolution policy mismatch for equivalent inputs. |
| `C119` | replay determinism | Resolver/solver replay mismatch on identical inputs (graph/lockfile/diagnostics drift). |

## Lock Notes
1. Registration to `docs/diagnostics.md` occurs during `22.0.6` implementation.
2. Fixture assertions must pin both code and deterministic ordering.
3. Text can remain concise, but code contracts are stable.

## References
- `docs/TODO.md` (`22.0.6`)
- `docs/diagnostics.md`
- `docs/design/phase-22.1.0-resolver-semver-determinism.md`
- `docs/design/phase-22.1.3-vulnerability-response-policy.md`

