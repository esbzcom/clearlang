# Phase 42.0.3 - Sepolia Offline RPC Fixture Lock

Date: 2026-10-14
Status: Locked
Owner: target-runtime-owner

## Purpose

Provide a secret-free, deterministic offline substitute for the selected Sepolia pilot endpoint.
The fixture validates the existing target adapter's exact accepted/rejected JSON-RPC surface; it
does not contact, emulate, or make a trust claim about a live endpoint.

The fixture set is [sepolia-rpc-fixtures.json](../fixtures/phase-42.0.3/sepolia-rpc-fixtures.json).
It has format `clg.sepolia-rpc-fixture-set.v1`, schema version `1`, target profile
`clg.evm-sepolia-pilot.v1`, and chain ID `11155111`.

## Fixture Semantics

Each case is an ordered list of canonical JSON-RPC method expectations and result or error
envelopes. The local loopback fixture server rejects an unexpected method/order. It derives
`$transaction_hash` only from the raw transaction submitted by the local adapter, or from the
receipt lookup parameter in standalone receipt tests. The placeholder is never a signer key,
endpoint credential, live transaction, or ambient value.

Fixtures contain no endpoint URL, user-info credential, faucet token, private key, authorization
header, or account balance. The fixture server binds only to loopback during tests and is not an
operator-facing network service.

## Required Matrix

| Case | Expected adapter result |
| --- | --- |
| `call-success` | Accept the Sepolia chain response and canonical `eth_call` bytes. |
| `deploy-success` | Accept locally signed transaction hash echo and successful deployment receipt. |
| `invoke-success` | Accept locally signed transaction hash echo and successful invocation receipt. |
| `wrong-chain` | Reject before any target receipt can be created. |
| `substituted-transaction-hash` | Reject the response because it differs from the local signed transaction hash. |
| `reverted-receipt` | Reject `status: "0x0"`; no successful receipt exists. |
| `malformed-receipt` | Reject invalid receipt field encoding. |
| `unavailable-receipt` | Reject after the bounded ten receipt polls. |
| `malformed-call-result` | Reject a non-byte `eth_call` result. |
| `required-method-rpc-error` | Reject an RPC error for a required method. |

The local target tests parse this fixture, enforce its schema/profile/chain/method allowlist and
sanitization invariants, then run the real `call`, `deploy`, `invoke`, chain check, transaction
hash check, and receipt parser against its loopback responses. A fixture that changes ordering,
method, profile, chain ID, or expected failure behavior therefore fails locally.

## Boundaries

The fixture does not establish Sepolia availability, a provider's honesty, client correctness,
faucet availability, actual inclusion, consensus finality, or a completed live testnet exercise.
Those are explicit operator observations belonging to 42.2. The only claim here is that the
adapter's fixed local protocol behavior is covered by reproducible fixtures.

## Exit Criteria

1. Every permitted RPC success path and each locked failure class has an ordered, sanitized
   fixture case.
2. Local tests run the existing adapter against only loopback fixture responses and prove the
   accepted/rejected result for every case.
3. The fixture has no secret, endpoint, live transaction, or network dependency.
4. The roadmap marks 42.0, 42.0.1, 42.0.2, and 42.0.3 complete.

## References

- `docs/design/phase-42.0.1-testnet-conformance-questionnaire.md`
- `docs/design/phase-42.0.2-sepolia-pilot-profile-lock.md`
- `docs/fixtures/phase-42.0.3/sepolia-rpc-fixtures.json`
- `docs/todo/milestone_5_roadmap.md`
