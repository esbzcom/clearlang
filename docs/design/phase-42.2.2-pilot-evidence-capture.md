# Phase 42.2.2: Canonical pilot-evidence capture

`clg target capture-pilot-evidence` converts already-recorded, local pilot observations into one
canonical record. It is an offline evidence-binding command: it has no signing option and never
contacts an RPC endpoint. A caller supplies one designated evidence root, a root-relative capture
input, and an output location:

```text
clg target capture-pilot-evidence --root <DIR> --input capture.json --out pilot-evidence.json
```

Every capture input location is a canonical relative path below `--root`. The command rejects
absolute paths, drive prefixes, backslashes, empty or `.` segments, parent traversal, symlinks,
and files resolving outside the root. Referenced evidence must use canonical JSON encoding.
Neither source paths nor endpoint URLs are serialized into the final record.

## Capture input

The input format is `clg.pilot-evidence-capture-input.v1`, schema version `1`:

```json
{
  "format": "clg.pilot-evidence-capture-input.v1",
  "schema_version": 1,
  "submission_intent": "intent.json",
  "release_bundle": "release/contract-release-bundle.json",
  "verifier_result": "release-verifier-result.json",
  "observations": [
    { "kind": "deploy", "caller": "0x...", "rpc_observations": "rpc/deploy.json", "receipt": "receipts/deploy.json" },
    { "kind": "authorized-invoke", "caller": "0x...", "rpc_observations": "rpc/authorized.json", "receipt": "receipts/authorized.json" },
    { "kind": "unauthorized-invoke", "caller": "0x...", "rpc_observations": "rpc/unauthorized.json", "receipt": null }
  ]
}
```

The three observations are mandatory and ordered exactly as shown. `deploy` and
`authorized-invoke` require successful canonical target receipts. Their callers must equal the
reviewed sender and their signed-bundle digest must equal the signature artifact digest declared
by the release bundle. Transaction nonce, value, gas limit, gas price, target address, and
function selector are checked against the preflight intent. `unauthorized-invoke` requires a
different caller, a reverted (`0x0`) recorded receipt response, and no successful target receipt.

`submission_intent` is the canonical `clg.pilot-exercise-preflight.v1` output from 42.2.1.
`verifier_result` is a canonical `clg.pilot-release-verifier-result.v1` object containing only
`schema_version: 1`, `status: "verified"`, and the exact SHA-256 digest of the release-bundle
file. It records the outcome of the existing release verifier without treating an operator claim
as a replacement for release verification.

## RPC observations and output

Each RPC observation file is `clg.pilot-rpc-observations.v1`, schema version `1`, and contains a
chain ID, target profile, and ordered `exchanges`. Exchanges are limited to `eth_chainId`,
`eth_sendRawTransaction`, and one or more `eth_getTransactionReceipt` calls in that order. The
observed chain, returned transaction digest, and final receipt status must match the observation
kind. Endpoint, credential, private-key, password, secret, token, authorization, and signer
reference fields are rejected before capture. A signed raw transaction is retained only as a
published transaction request; it contains no signing secret and is required for offline replay.

The result is canonical `clg.pilot-evidence.v1`. It contains no paths, endpoint, signer reference,
or key material. It records the preflight and release-bundle digests, release signature digest,
verified result binding, ordered caller/transaction/receipt identities, and sanitized RPC
exchanges. Its `evidence_signature.status` is `not_authorized`: the existing signing policy covers
release artifacts, not a new pilot-evidence scope. No signature is created until an explicit policy
authorizes one.

The next task replays these recorded exchanges offline; capture itself does not replay or submit
anything.
