# Shared Std Operations

## Scope
Operational guidance for publishing and distributing shared std artifacts while the mode remains
explicit opt-in.

## Publish
1. Set the shared-std signing key:

```powershell
$env:CLG_SHARED_STD_SIGNING_KEY_HEX = "<32-byte-ed25519-private-key-hex>"
```

2. Publish the package bundle:

```powershell
cargo run -p xtask -- shared-std-publish `
  --version 1.0.0 `
  --signed-at 2026-06-27T00:00:00Z `
  --out-dir dist/shared-std/std-core/1.0.0 `
  --registry-dir dist/shared-std/registry `
  --key-id shared-std-2026q2
```

3. Confirm the publish bundle contains:
   - `std-core-<version>.wasm`
   - `std-core-<version>.sha256`
   - `std-core-<version>.package-signatures.json`
   - `std-core-<version>.provenance.json`
   - `std-core-<version>.shared-std-package.json`
   - `std-core-<version>.publish.json`

## Registry Layout
File-registry publication writes versioned copies under:

```text
<registry-dir>/std__core/<version>/
```

This layout is deterministic:
- package path component is `std__core`,
- version directory is exact semver,
- no silent overwrite or embedded fallback is allowed.

## Release Integration
1. Publish shared std first.
2. Lock the project with explicit shared std intent.
3. Run `clg release`.
4. Run `clg verify-bundle --require-provenance`.

## Upgrade
1. Publish the new shared std version with a new `--signed-at`.
2. Update project lock inputs to the new version.
3. Run release + verify-bundle against the updated lock.
4. Keep the previous version directory intact until rollback window closes.

## Rollback
1. Re-point project lock inputs to the last known-good shared std version.
2. Re-run release + verify-bundle.
3. Mark the bad version as blocked in operator notes and trust inputs if signer compromise is involved.

## References
- `docs/release-process.md`
- `docs/security/shared-std-key-rotation.md`
- `docs/security/shared-std-rollback-procedure.md`
