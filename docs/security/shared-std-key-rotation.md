# Shared Std Key Rotation

Use this policy when rotating the signing key used by `xtask shared-std-publish`.

## Rules
1. New publishes must use a new `key_id`.
2. `signed_at` must stay explicit and UTC RFC3339 (`YYYY-MM-DDTHH:MM:SSZ`).
3. Old registry versions remain immutable.
4. Revoke compromised keys in trust inputs; do not mutate published artifacts in place.

## Rotation Steps
1. Generate a new Ed25519 private key and store it outside the repo.
2. Publish the replacement public key metadata to operators.
3. Set:

```powershell
$env:CLG_SHARED_STD_SIGNING_KEY_HEX = "<new-32-byte-ed25519-private-key-hex>"
```

4. Re-run:

```powershell
cargo run -p xtask -- shared-std-publish --signed-at <UTC-RFC3339Z> --key-id <new-key-id>
```

5. Update trust-policy/key-distribution material so consumers trust the new `key_id`.
6. If the old key is compromised, revoke it before any further shared-std release flow.

## Evidence To Retain
- publish manifest,
- signer public-key metadata,
- provenance statement,
- operator note linking old and new `key_id`.
