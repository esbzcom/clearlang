# Phase 42.2.1: Pilot operator input and offline review

The testnet pilot begins with an explicit, operator-authored file. It is a review boundary, not a
wallet configuration mechanism and not a submission command. The only supported command for this
step is:

```text
clg target pilot-preflight --input pilot-exercise.json --report-out pilot-preflight.json
```

`pilot-preflight` reads only its input file and writes only the review record. It does not contact
`rpc_url`, resolve or read `signer_reference`, sign a transaction, submit a transaction, or infer
any endpoint, account, nonce, fee, release bundle, or arguments.

## Input schema

The document format is `clg.pilot-exercise-input.v1`, schema version `1`. Unknown fields are
rejected so an operator cannot believe an ignored setting is part of the reviewed plan.

```json
{
  "format": "clg.pilot-exercise-input.v1",
  "schema_version": 1,
  "target_profile": "clg.evm-sepolia-pilot.v1",
  "rpc_url": "https://operator-provided-sepolia-rpc.example",
  "chain_id": 11155111,
  "signer_reference": "operator/sepolia-signer.reference",
  "sender": "0x1111111111111111111111111111111111111111",
  "release_bundle": "release/contract-release-bundle.json",
  "deploy": {
    "nonce": 7,
    "value": 0,
    "gas_limit": 250000,
    "fee": { "kind": "legacy-eip155-gas-price", "gas_price": 3 },
    "constructor_arguments": "release/constructor-args.json"
  },
  "invocation": {
    "contract_address": "0x2222222222222222222222222222222222222222",
    "function_selector": "0x13765838",
    "arguments": "release/authorized-set-value.args.json",
    "nonce": 8,
    "value": 0,
    "gas_limit": 100000,
    "fee": { "kind": "legacy-eip155-gas-price", "gas_price": 3 }
  }
}
```

`signer_reference` identifies operator-managed signing material; it must never contain key data.
All four locations (`signer_reference`, `release_bundle`, `constructor_arguments`, and
`invocation.arguments`) are canonical relative paths: non-empty forward-slash paths with no
absolute prefix, drive prefix, backslash, `.` segment, or `..` segment. The command validates
locations without dereferencing them. This makes the record portable and prevents it from
silently exposing machine-specific paths or secrets.

The target profile is fixed to Sepolia and requires chain ID `11155111`. The endpoint must be an
explicit HTTP(S) URL without embedded credentials. Sender and contract address must be canonical
20-byte hexadecimal addresses. `function_selector` must be a canonical four-byte hexadecimal
selector. Both transaction sections require a nonzero gas limit and a nonzero
`legacy-eip155-gas-price` fee.

## Review record

On success, the command writes and prints byte-identical canonical JSON with format
`clg.pilot-exercise-preflight.v1`. It echoes every accepted input value in normalized form and
adds a `safety` object that states no network was contacted, signer material was not accessed,
and no transaction was signed or submitted. Operators must inspect this record before separately
performing any live action.

This phase deliberately does not execute the pilot. Evidence capture and the recorded-RPC replay
verifier are the following 42.2 tasks.
