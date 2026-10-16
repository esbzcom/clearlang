# Pilot operator runbook

This runbook is for the authorized deploy operator of the one ClearLang testnet pilot. It is not
the independent-verifier walkthrough: an operator uses a local ClearLang checkout and may perform
explicit testnet submissions. The only supported target is
`clg.evm-sepolia-pilot.v1` on Sepolia chain ID `11155111`.

Do not begin a live action until the release owner has approved the exact release bundle and every
value in the operator plan. This is a **locally validated, testnet-only pilot**. It must not be
described as generic EVM support, remote-CI certification, mainnet readiness, or custody safety.

## Labels and external inputs

Each step is marked as one of:

- **[LOCAL / NO NETWORK]** — reads and writes local files only. It must not contact an endpoint,
  wallet, or signer.
- **[NETWORK / SIGNS / SUBMITS]** — can contact the explicit RPC endpoint and, where stated,
  read a local signer and submit a testnet transaction. It needs an explicit approval for that
  action.
- **[EXTERNAL INPUT]** — a value supplied or reviewed outside ClearLang. It is never inferred.

Before the run, record and review this complete input set in the operator change record:

| Input | Classification | Rule |
| --- | --- | --- |
| Release directory and `*.contract-release-bundle.json` | [EXTERNAL INPUT] | Complete, unmodified release evidence set; the bundle identifies its sibling artifact files. |
| Release public key and independent verifier result | [EXTERNAL INPUT] | Use the retained canonical `clg.contract-release-verifier-result.v1` file from the [offline verifier walkthrough](contract-verifier-offline-walkthrough.md). Its `bundle_sha256` must bind the exact bundle bytes. |
| RPC URL | [EXTERNAL INPUT] / [NETWORK] | Explicit credential-free HTTP(S) URL. It is not a trust claim about the provider. |
| Target profile and chain ID | [EXTERNAL INPUT] / fixed | Exactly `clg.evm-sepolia-pilot.v1` and `11155111`; no replacement is allowed. |
| Owner sender, deploy nonce, invoke nonce, value, gas limit, gas price, constructor arguments, function, and invocation arguments | [EXTERNAL INPUT] | Review every value before signing. Nonces are explicit and never auto-selected. The invocation contract address is the reviewed expected deployment address. |
| Owner signing-key file | [EXTERNAL INPUT] / secret | Testnet-only local `clg.evm-signing-key.v1` key. Keep the file and its contents outside the checkout, evidence root, shell history, logs, and support tickets. |
| Unauthorized sender, signing-key file, and nonce | [EXTERNAL INPUT] / secret | A distinct, testnet-only signer used only to demonstrate the expected rejected transition. |
| Sanitized RPC exchanges and target receipts | [EXTERNAL INPUT] after live actions | Retain the exact allowed request/response observations. Do not retain endpoint credentials, private-key data, tokens, passwords, or authorization headers. |

The deploy operator must calculate and review the anticipated contract address before running the
preflight record. ClearLang intentionally does not discover a nonce or derive an address from an
ambient wallet/account.

## 1. Local pilot precheck

Run this from the reviewed ClearLang checkout, with all Cargo dependencies already available
locally. The offline setting applies to the command and its child Cargo processes; it prevents an
incidental registry/network fetch from being mistaken for local acceptance.

```powershell
# [LOCAL / NO NETWORK]
$env:CARGO_NET_OFFLINE = 'true'
try {
    cargo run --offline -p xtask -- pilot-precheck --report out/pilot-precheck-full.json
    if ($LASTEXITCODE -ne 0) { throw 'STOP: pilot precheck failed' }
}
finally {
    Remove-Item Env:CARGO_NET_OFFLINE -ErrorAction SilentlyContinue
}
$Precheck = Get-Content out/pilot-precheck-full.json -Raw | ConvertFrom-Json
if ($Precheck.kind -cne 'clg.pilot-precheck-report.v1' -or
    $Precheck.scope -cne 'full' -or
    $Precheck.overall_status -cne 'passed') {
    throw 'STOP: pilot precheck did not produce a passing full report'
}
```

Retain the report unchanged. Its **`full`** scope covers the fixed local release-precheck,
contract-release, verifier-distribution, caller-authorization, and reference-pilot replay gates.
It is evidence for the bounded testnet pilot only; it does not authorize a live action by itself
and does not make a production, mainnet, or custody claim.

## 2. Review the offline release evidence and operator plan

First follow the offline verifier walkthrough and copy its retained
`contract-verification-result.json` into the evidence root as
`release-verifier-result.json`. This copy is **[LOCAL / NO NETWORK]**. Do not hand-author or
reformat the verifier output: pilot evidence capture requires its canonical bytes.

Create `pilot-exercise.json` with the reviewed values below. Every angle-bracket value is an
**[EXTERNAL INPUT]**. `signer_reference` is an opaque reference only; it is not a path to key
material and does not cause `pilot-preflight` to read a key.

```json
{
  "format": "clg.pilot-exercise-input.v1",
  "schema_version": 1,
  "target_profile": "clg.evm-sepolia-pilot.v1",
  "rpc_url": "<EXTERNAL_CREDENTIAL_FREE_HTTPS_SEPOLIA_RPC_URL>",
  "chain_id": 11155111,
  "signer_reference": "operator/<EXTERNAL_OWNER_SIGNER_REFERENCE>",
  "sender": "<EXTERNAL_OWNER_SENDER_ADDRESS>",
  "release_bundle": "release/<EXTERNAL_BUNDLE_FILENAME>",
  "deploy": {
    "nonce": <EXTERNAL_DEPLOY_NONCE>,
    "value": <EXTERNAL_DEPLOY_VALUE>,
    "gas_limit": <EXTERNAL_DEPLOY_GAS_LIMIT>,
    "fee": { "kind": "legacy-eip155-gas-price", "gas_price": <EXTERNAL_DEPLOY_GAS_PRICE> },
    "constructor_arguments": "release/<EXTERNAL_CONSTRUCTOR_ARGUMENTS_FILENAME>"
  },
  "invocation": {
    "contract_address": "<EXTERNAL_EXPECTED_DEPLOYMENT_ADDRESS>",
    "function_selector": "<EXTERNAL_FOUR_BYTE_SELECTOR>",
    "arguments": "release/<EXTERNAL_AUTHORIZED_ARGUMENTS_FILENAME>",
    "nonce": <EXTERNAL_OWNER_INVOKE_NONCE>,
    "value": <EXTERNAL_INVOKE_VALUE>,
    "gas_limit": <EXTERNAL_INVOKE_GAS_LIMIT>,
    "fee": { "kind": "legacy-eip155-gas-price", "gas_price": <EXTERNAL_INVOKE_GAS_PRICE> }
  }
}
```

Validate and retain the review record before any transaction is signed:

```powershell
# [LOCAL / NO NETWORK] — does not resolve the endpoint or read the signer.
clg target pilot-preflight --input pilot-exercise.json --report-out pilot-preflight.json
```

The resulting `clg.pilot-exercise-preflight.v1` record must say all four `safety` flags are
`false`. A malformed plan, non-Sepolia chain ID, credential-bearing endpoint, unsafe path, or
unreviewable transaction value stops here.

## 3. Explicit testnet observations

The following commands are **[NETWORK / SIGNS / SUBMITS]**. They call `eth_chainId`, then submit
a locally signed legacy EIP-155 transaction with `eth_sendRawTransaction`, and poll
`eth_getTransactionReceipt`. They can consume testnet funds. They must not be run by local test
automation or retried with changed evidence after an error.

Set the reviewed local paths and values once. The signing-key paths name secret files but do not
put secret contents in the command; ensure the terminal history and logs do not retain these
commands if local policy treats secret file locations as sensitive.

```powershell
# [EXTERNAL INPUT] reviewed release artifact paths and testnet transaction values.
$Release = '<EXTERNAL_RELEASE_DIRECTORY>'
$Artifact = Join-Path $Release '<EXTERNAL_EVM_ARTIFACT_FILENAME>'
$WireAbi = Join-Path $Release '<EXTERNAL_WIRE_ABI_FILENAME>'
$Schema = Join-Path $Release '<EXTERNAL_SCHEMA_FILENAME>'
$Proof = Join-Path $Release '<EXTERNAL_PROOF_FILENAME>'
$Signature = Join-Path $Release '<EXTERNAL_SIGNATURE_FILENAME>'
$ConstructorArgs = Join-Path $Release '<EXTERNAL_CONSTRUCTOR_ARGUMENTS_FILENAME>'
$AuthorizedArgs = Join-Path $Release '<EXTERNAL_AUTHORIZED_ARGUMENTS_FILENAME>'
$RpcUrl = '<EXTERNAL_CREDENTIAL_FREE_HTTPS_SEPOLIA_RPC_URL>'
$OwnerSender = '<EXTERNAL_OWNER_SENDER_ADDRESS>'
$OwnerSigningKey = '<EXTERNAL_LOCAL_OWNER_SIGNING_KEY_FILE>'
$UnauthorizedSender = '<EXTERNAL_UNAUTHORIZED_SENDER_ADDRESS>'
$UnauthorizedSigningKey = '<EXTERNAL_LOCAL_UNAUTHORIZED_SIGNING_KEY_FILE>'
$ExpectedContractAddress = '<EXTERNAL_EXPECTED_DEPLOYMENT_ADDRESS>'
$Function = '<EXTERNAL_CANONICAL_FUNCTION_IDENTIFIER>'
```

1. Deploy with the owner signer. This is **[NETWORK / SIGNS / SUBMITS]**. The owner sender,
   nonce, value, gas, gas price, constructor arguments, endpoint, and all release-evidence paths
   must match the reviewed plan.

   ```powershell
   clg target deploy --target-profile clg.evm-sepolia-pilot.v1 --rpc-url $RpcUrl --chain-id 11155111 --sender $OwnerSender --nonce <EXTERNAL_DEPLOY_NONCE> --signing-key $OwnerSigningKey --value <EXTERNAL_DEPLOY_VALUE> --gas-limit <EXTERNAL_DEPLOY_GAS_LIMIT> --gas-price <EXTERNAL_DEPLOY_GAS_PRICE> --artifact $Artifact --abi $WireAbi --contract-state-schema $Schema --proof-artifact $Proof --signed-bundle $Signature --args $ConstructorArgs --receipt-out receipts/deploy.json
   ```

   Stop if this does not produce a successful receipt. Confirm its contract address equals the
   reviewed `$ExpectedContractAddress`; do not modify the preflight record to accommodate a
   different result.

2. Invoke the authorized transition with the owner signer. This is **[NETWORK / SIGNS / SUBMITS]**.

   ```powershell
   clg target invoke --target-profile clg.evm-sepolia-pilot.v1 --rpc-url $RpcUrl --chain-id 11155111 --artifact $Artifact --abi $WireAbi --contract-state-schema $Schema --proof-artifact $Proof --signed-bundle $Signature --contract-address $ExpectedContractAddress --function $Function --args $AuthorizedArgs --sender $OwnerSender --nonce <EXTERNAL_OWNER_INVOKE_NONCE> --signing-key $OwnerSigningKey --value <EXTERNAL_INVOKE_VALUE> --gas-limit <EXTERNAL_INVOKE_GAS_LIMIT> --gas-price <EXTERNAL_INVOKE_GAS_PRICE> --receipt-out receipts/authorized.json
   ```

3. Invoke the same transition with the distinct unauthorized signer. This is
   **[NETWORK / SIGNS / SUBMITS]** and the expected result is a rejected/reverted transaction.
   A non-zero CLI exit and no successful ClearLang target receipt are expected; they are not a
   reason to change the source, release bundle, endpoint, or authorization evidence.

   ```powershell
   clg target invoke --target-profile clg.evm-sepolia-pilot.v1 --rpc-url $RpcUrl --chain-id 11155111 --artifact $Artifact --abi $WireAbi --contract-state-schema $Schema --proof-artifact $Proof --signed-bundle $Signature --contract-address $ExpectedContractAddress --function $Function --args $AuthorizedArgs --sender $UnauthorizedSender --nonce <EXTERNAL_UNAUTHORIZED_INVOKE_NONCE> --signing-key $UnauthorizedSigningKey --value <EXTERNAL_INVOKE_VALUE> --gas-limit <EXTERNAL_INVOKE_GAS_LIMIT> --gas-price <EXTERNAL_INVOKE_GAS_PRICE> --receipt-out receipts/unauthorized-must-not-exist.json
   ```

For each of the three actions, retain a sanitized canonical
`clg.pilot-rpc-observations.v1` file containing exactly the recorded `eth_chainId`,
`eth_sendRawTransaction`, and `eth_getTransactionReceipt` exchanges. Capturing those exchanges
is itself **[NETWORK]** because it observes the live endpoint. A recorder must not insert
credentials, headers, endpoint URLs, tokens, signer references, or private-key material. The
unauthorized observation must retain its `0x0` receipt response but must not have a ClearLang
success receipt.

## 4. Bind and replay the evidence locally

Arrange the retained, canonical files under one evidence root. This arrangement and both commands
below are **[LOCAL / NO NETWORK]**:

```text
evidence-root/
  capture.json
  pilot-preflight.json
  release/
    <bundle and referenced release artifacts>
  release-verifier-result.json
  receipts/deploy.json
  receipts/authorized.json
  rpc/deploy.json
  rpc/authorized.json
  rpc/unauthorized.json
```

Create `capture.json` with the precise relative paths and recorded callers. All path and caller
values below are **[EXTERNAL INPUT]** derived from the reviewed plan and observations; preserve
the required observation order.

```json
{
  "format": "clg.pilot-evidence-capture-input.v1",
  "schema_version": 1,
  "submission_intent": "pilot-preflight.json",
  "release_bundle": "release/<EXTERNAL_BUNDLE_FILENAME>",
  "verifier_result": "release-verifier-result.json",
  "observations": [
    { "kind": "deploy", "caller": "<EXTERNAL_OWNER_SENDER_ADDRESS>", "rpc_observations": "rpc/deploy.json", "receipt": "receipts/deploy.json" },
    { "kind": "authorized-invoke", "caller": "<EXTERNAL_OWNER_SENDER_ADDRESS>", "rpc_observations": "rpc/authorized.json", "receipt": "receipts/authorized.json" },
    { "kind": "unauthorized-invoke", "caller": "<EXTERNAL_UNAUTHORIZED_SENDER_ADDRESS>", "rpc_observations": "rpc/unauthorized.json", "receipt": null }
  ]
}
```

Bind the evidence and replay it without an endpoint, signer, or network selector:

```powershell
# [LOCAL / NO NETWORK]
clg target capture-pilot-evidence --root evidence-root --input capture.json --out pilot-evidence.json
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

# [LOCAL / NO NETWORK]
clg target replay-pilot-evidence --input pilot-evidence.json --report-out pilot-evidence-replay.json
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
```

The final report must be `clg.pilot-evidence-replay-report.v1` and state that the network was not
contacted. Retain the evidence root, `pilot-evidence.json`, and replay report without alteration.
The replay proves that the retained observation set is internally consistent; it does not establish
endpoint honesty, chain consensus, finality, mainnet suitability, or production readiness.

## Stop conditions and disclosure

Stop the pilot and preserve non-secret evidence if verification, plan review, chain binding,
receipt identity, authorized transition, or offline replay fails; if the unauthorized transition
succeeds; or if a signer might be exposed. Do not overwrite a receipt, change a release artifact,
or reuse a questionable signer to recover a passing result.

Report a target incompatibility or verification mismatch with: the verifier version/key ID,
bundle digest, precheck report digest, preflight digest, sanitized RPC-observation digests,
transaction/receipt digests, the stable diagnostic or failure text, and the specific stop point.
Never include a private key, token, endpoint credential, raw credential-bearing URL, or signer
file contents.

## References

- `docs/release/milestone_5_pilot_product_contract.md`
- `docs/release/milestone_5_pilot_incident_and_rollback.md`
- `docs/release/contract-verifier-offline-walkthrough.md`
- `docs/design/phase-40.1.1-pilot-precheck-cli-report-lock.md`
- `docs/design/phase-42.0.2-sepolia-pilot-profile-lock.md`
- `docs/design/phase-42.2.1-pilot-operator-input.md`
- `docs/design/phase-42.2.2-pilot-evidence-capture.md`
- `docs/design/phase-42.2.3-pilot-offline-replay.md`
