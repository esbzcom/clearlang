# Shared Std User Guide

## Status
Shared std is a supported explicit opt-in production mode. It is not the default.

- Default mode remains `embedded`.
- Shared std must be selected in `clg.project.json`.
- There is no silent fallback from `shared` to `embedded`.
- If shared std evidence is missing or mismatched, `clg pkg lock`, `clg release`, and `clg verify-bundle` fail closed.

## When To Use It
Use shared std only when you intentionally want versioned std package identities recorded in:

- `clg.lock.json`
- release bundle evidence
- verify-bundle provenance checks

If you do not need that, stay on the embedded default.

## Opt In
Shared std requires `clg.project.json` schema v2. Embedded-only projects can stay on schema v1.

Minimal shared-std intent example:

```json
{
  "schema_version": 2,
  "project": {
    "name": "app",
    "description": "ClearLang project",
    "version": "0.1.0",
    "clg_version": "^0.1.0",
    "entry": "main.clear",
    "website": "https://example.com",
    "contact": {
      "name": "Project Maintainer",
      "email": "maintainer@example.com"
    }
  },
  "dependencies": [
    { "name": "acme::payments", "requirement": "^1.4.0" }
  ],
  "std": {
    "delivery": "shared",
    "packages": [
      {
        "package_id": "std::text",
        "version_requirement": "^1.2.0",
        "verified_std_abi": {
          "major": 1,
          "minor_min": 3,
          "minor_max": 3
        },
        "registry": "default",
        "signer_policy": "std-publisher-prod",
        "allow_compat_shims": false
      }
    ]
  },
  "release_defaults": {
    "advisory_as_of": "2026-03-26T00:00:00Z",
    "key_id": "release-2026q2",
    "out_dir": "out/release",
    "trust_policy": "trust-policy.json"
  }
}
```

Rules that matter:

- `schema_version` must be `2` for shared std.
- `std.delivery` must be exactly `shared`.
- `std.packages[]` must be non-empty for shared std.
- current bundled shared-std package ids are `std::text`, `std::int`, `std::sequence`, and `std::codec`.
- Shared std package ids must not also appear in normal `dependencies[]`.

## Lock
Generate the lockfile from the shared manifest intent:

```powershell
clg pkg lock --generate --compiler-mode strict --advisory-as-of 2026-03-26T00:00:00Z --root <project-root>
```

What to expect:

- `clg.lock.json` uses schema v2.
- `clg.lock.json` records `std.delivery = shared`.
- `clg.lock.json` records locked shared std package identities, artifact digests, signature metadata, and provenance metadata.

What fails closed:

- schema v1 plus shared std intent
- empty `std.packages[]`
- duplicate shared std package ids
- ABI requirement mismatch
- manifest intent that conflicts with an existing embedded-only lockfile

## Release
Run the normal release flow:

```powershell
clg release --root <project-root>
```

For shared std, `clg release` requires:

- non-empty shared std lock evidence
- matching shared std intent between manifest and lockfile
- matching shared std provenance between lock evidence and release evidence

If any of those are wrong, release fails. It does not downgrade to embedded mode.

## Verify
Verify the resulting bundle the same way you verify any release bundle:

```powershell
clg verify-bundle --bundle out/<name>.release-bundle.json --keyring keys/release-keyring.json --require-provenance
```

For shared std, verification additionally expects the shared std evidence chain to match the release bundle claim.

## Upgrade
To upgrade shared std:

1. Update the version requirement in `clg.project.json`.
2. Regenerate `clg.lock.json`.
3. Run `clg release`.
4. Run `clg verify-bundle --require-provenance`.

Do not hand-edit the lockfile to force a version change.

## Rollback
To roll back shared std:

1. Restore the prior shared std version requirement in `clg.project.json`.
2. Regenerate `clg.lock.json`.
3. Re-run release and verify-bundle.

Do not keep the bad version in place and hope verification ignores it. It will not.

## Incident Response
Treat these as incidents:

- shared std artifact missing
- shared std signature/trust failure
- shared std provenance mismatch
- shared std ABI mismatch

Immediate action:

1. stop release promotion for the affected project,
2. revert to the last known-good shared std version,
3. re-run release + verify-bundle,
4. escalate to the operator runbooks if signer or registry state is suspect.

## Related Docs
- `docs/release/shared-std-operations.md`
- `docs/security/shared-std-key-rotation.md`
- `docs/security/shared-std-rollback-procedure.md`
- `docs/design/phase-28.1.0-shared-std-manifest-lock-extension.md`
- `docs/design/phase-28.1.2-shared-std-migration-coexistence-diagnostics.md`
