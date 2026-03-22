# Runtime Loader Resilience Runbook (Phase 23.0.4)

## Scope
This runbook defines fail-closed runtime artifact availability behavior for `clg run` in milestone_2.

## Inputs
- `clg.runtime-link.json` + `clg.runtime-link.sha256`
- `clg.lock.json`
- `clg.package-signatures.json`
- `clg.trust-policy.json`
- `clg.package-store-index.json`
- optional: `clg.runtime-loader.json`

## Runtime Loader Policy File
Optional file: `clg.runtime-loader.json`

Schema v0:
```json
{
  "schema_version": 0,
  "offline_mode": true,
  "mirror_paths": ["mirror-a", "mirror-b"],
  "artifact_read_retries": 2
}
```

Rules:
- `offline_mode` must be `true` in Phase 23.0.4 (`offline_mode=false` is rejected).
- `mirror_paths` must be relative paths; no absolute paths and no `..` traversal.
- `artifact_read_retries` must be `1..=8`.
- Mirror search order is deterministic and follows declared array order.

## Availability Resolution Order
For each package artifact path from trusted store index:
1. Primary root: `<project-root>/<artifact_path>`
2. Mirror roots in order:
   - `<project-root>/<mirror_path>/<artifact_path>`
3. For each candidate root, retry up to `artifact_read_retries`.
4. First candidate with matching digest is selected.
5. If all candidates are missing/unreadable: fail `R012`.
6. If candidates exist but all digest checks fail: fail `R013`.

## Determinism Guarantees
- Candidate roots are evaluated in a fixed order (primary, then mirror order from policy).
- Retries are bounded and deterministic.
- No implicit network fetch in this phase.
- Trust/signature/lock/runtime-link checks still run before linking.

## Failure Diagnostics
- `R012`: artifact unavailable across configured roots, or invalid availability policy input.
- `R013`: digest mismatch during artifact verification.
- `R014`: trust/signature policy failure.
- `R015`: runtime linker/binding mismatch.
- `R016`: runtime host profile/capability mismatch (invalid profile input or missing required capability for active imports).
- `R017`: runtime-link determinism/hash mismatch.

## Operational Procedures
### Mirror outage
- Keep `mirror_paths` ordered by preferred source.
- If first mirror is stale/unavailable, runtime automatically falls through to later mirrors.
- If all mirrors fail, runtime exits fail-closed with `R012`.

### Cache corruption
- If digest mismatch occurs, runtime exits with `R013`.
- Replace corrupted artifact from a trusted source and rerun.

### Rollback
- Remove or reorder failing mirror entries in `clg.runtime-loader.json`.
- Keep `offline_mode=true` and rerun deterministic path.
