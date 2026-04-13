# Phase 25.2.5 - `clg strict init <root>` Bootstrap Lock

## Status
Design lock + implementation record for `25.2.5` in `docs/TODO.md`.

## Gap Closed
`25.2.5` requires `clg.project.json` defaults before full manifest semantics (`25.4.x`) are finalized.
This lock defines a bootstrap schema that is strict, deterministic, and fail-closed for release defaults.
`25.4.1` later extends this bootstrap into project manifest schema v1.

## `clg strict init <root>` Contract
`clg strict init <root>` must:
1. Create missing strict preflight inputs with deterministic templates:
   - `clg.project.json`
   - `clg.lock.json`
   - `clg.trust-policy.json`
   - `clg.host-profile.json`
   - `clg.package-metadata.json`
   - `clg.package-abi.json`
   - `trust-policy.json`
2. Validate strict build preflight inputs using the same strict loaders as production strict build.
3. Validate `clg.project.json` release-default schema and validate `trust-policy.json` schema v1 trust anchors.
4. Never downgrade strict release policy; failures are fail-closed.

## Bootstrap `clg.project.json` Schema (v0, Historical)
Initial bootstrap schema:

```json
{
  "schema_version": 0,
  "release_defaults": {
    "advisory_as_of": "REQUIRED_RFC3339_UTC",
    "key_id": "REQUIRED_KEY_ID",
    "out_dir": "out/release",
    "trust_policy": "trust-policy.json"
  }
}
```

Rules:
- `schema_version` must be `0`.
- `release_defaults.out_dir` and `release_defaults.trust_policy` must be relative non-traversing paths.
- Placeholder values (`REQUIRED_RFC3339_UTC`, `REQUIRED_KEY_ID`) are allowed at init time.
- `clg release` treats placeholders as missing and fails closed until manifest values are explicitly filled.

## Current Schema Note
Current `clg.project.json` template emitted by `clg strict init` uses schema v1 with:
- project metadata (`name`, `description`, `version`, `clg_version`, `entry`, `website`, `contact`),
- dependency requirements (`dependencies[]`),
- retained `release_defaults`.

See `docs/design/phase-25.4.1-project-manifest-v1.md` for the current schema and validation contract.

## `clg release` Default Resolution
`clg release` now resolves required values in this order:
1. `clg.project.json` schema v1 `project.entry`.
2. `clg.project.json` `release_defaults`.
3. Fail with deterministic release diagnostic (`C130`) if unresolved.

## References
- `crates/cli/src/commands/strict.rs`
- `crates/cli/src/commands/release_defaults.rs`
- `crates/cli/src/commands/release.rs`
- `crates/cli/src/main.rs`
- `docs/release-process.md`
