# Phase 52.1 - Safe Registry Evidence and Offline Replay Schemas

Date: 2026-10-17

Status: Locked schemas for later local capture and offline replay. They authorize
no endpoint contact, Safe operation, signing, transaction construction,
deployment, invocation, or mainnet claim.

## Record Families

Phase 52 uses two closed, versioned JSON record families:

| Record | Format | Purpose |
| --- | --- | --- |
| Target evidence | clg.safe-registry-target-evidence.v1 | A canonical, secret-free binding of already recorded target observations to the Phase 52.0 identity chain. |
| Replay report | clg.safe-registry-evidence-replay-report.v1 | The deterministic result of replaying one complete target-evidence record without a network, endpoint, wallet, or signer. |

Both use schema_version 1, canonical UTF-8 JSON, reject unknown/duplicate fields,
and use lowercase SHA-256 digests. Every file reference accepted during future
capture is root-relative, forward-slash-only, and resolved below an explicit
artifact root. The emitted evidence and report contain no local paths.

## Common Identity Envelope

Every target-evidence record includes one exact identity object:

| Field | Required binding |
| --- | --- |
| target_profile / chain_id | clg.evm-ethereum-mainnet-safe-registry.v1 and decimal string 1 |
| source_profile / proof | Source, compiler, solver, proof-profile, and proof-result digests from the verified release |
| state_schema / ABI / event ABI | Digests for the storage layout, constructor/set_value ABI, and ValueChanged event ABI |
| EVM artifact | Creation-bytecode, runtime-bytecode, and constructor-calldata layout digests |
| release | Release-bundle digest, verifier-result digest, verifier version, and verified bundle identity |
| policy | Phase 50.2 Safe amendment, Phase 51 operator policy, request, policy-evidence, and validator-report digests |
| Safe identity | Proxy/singleton addresses and code digests, version 1.4.1, ordered owner-set digest, threshold 2, observed nonce, and no-module/guard/fallback results |
| observer policy | Observer identifiers, independent-operator classification, client-policy version, and sanitized exchange digests |

The record must not contain an endpoint URL, credential, authorization header,
private key, seed, recovery data, raw wallet export, absolute path, device
location, or ambient checkout identity.

## Observation Kinds

The evidence observations array is ordered and contains only these named kinds:

| Kind | Required evidence | Acceptance boundary |
| --- | --- | --- |
| deployment | One direct zero-value creation request/transaction/receipt/address/code/initial-state sequence. | Requires every deployment correspondence check in Phase 52.0. |
| safe_authorized_revision | One Safe execTransaction outer transaction plus the locked zero-value CALL inner set_value(next), event, and final state. | Requires every revision correspondence check in Phase 52.0. |
| expected_unauthorized_invocation | A separately authorized test-only negative case with no successful transition/event. | Never becomes deployment or revision acceptance evidence. |
| confirmation | Block/header, code, storage, event, Safe configuration, and finalized-head observations for one deployment or revision identity. | Records observation progress only. |
| reorganization_observation | A later comparison that detects or rules out changed receipt/block/finality identity. | A detected reorganization is unaccepted and stop. |

Each observation has id, kind, subject_action_id, captured_at, primary and
independent observer exchange digests, block identities, comparison result, and
a closed status. Deployment and revision observations additionally bind the
transaction hash, sender, nonce, legacy envelope fields, zero value, input
digest, receipt, and target address. A revision binds its Safe typed digest,
signature-bundle digest, inner CALL details, and ABI-decoded event value.

## Sanitized Exchange Form

An exchange is retained as:

- observer_id and observer_class, but not a provider URL;
- client family/version as untrusted disclosed metadata;
- allowed JSON-RPC method name, canonical parameter digest, response digest,
  capture timestamp, and block identity when applicable; and
- normalized result fields required by the Phase 52.0 comparison.

Raw requests/responses may remain in a protected local capture root only if
they pass the same secret/path scan and are referred to by digest. They are not
part of a published evidence record. A capture implementation must remove
credentials, headers, endpoint URLs, absolute paths, and client-private
metadata before hashing its emitted exchange form.

## Required Evidence Shape

The target-evidence top level is:

    format, schema_version, target, identity, capture, observations,
    correspondence_status, evidence_signature

capture declares offline_capture_only false only as a description of its
source; it must state that the final record contains no endpoint or secret.
evidence_signature is always status not_authorized in schema version 1. No
evidence signing capability is introduced by this phase.

A complete deployment evidence record requires exactly one deployment
observation, its confirmation observations, and its reorganization observation.
A complete revision record requires exactly one safe_authorized_revision
observation, its confirmation observations, and its reorganization observation.
Expected unauthorized invocation is optional testnet-only evidence and cannot
replace either production action.

## Replay Contract

Future replay accepts exactly one completed target-evidence file and a report
destination. It accepts no RPC URL, network selector, key, wallet, endpoint,
request override, or action selection. It reconstructs the recorded
comparisons from canonical identities and retained exchange forms, recomputes
all digests, and verifies ordering, Safe/action constraints, receipt/block
links, finality sequence, and two-observer agreement without opening a socket.

A replay report contains:

| Field | Rule |
| --- | --- |
| format / schema_version | clg.safe-registry-evidence-replay-report.v1 and 1 |
| evidence_sha256 | Digest of the exact evidence input |
| target_profile / chain_id | Copied only after exact validation |
| status / diagnostic | One closed pair from the table below |
| observation_results | Ordered id/kind/result list with no endpoint or secret |
| network_contacted | Exactly false |
| production_claim | Exactly not_established |

## Stable Replay Statuses

| Status | Diagnostic | Meaning |
| --- | --- | --- |
| replayed | M6C000 | The retained record is canonical and internally corresponds; this is offline replay only, not target or production acceptance. |
| rejected | M6C001 | Malformed, noncanonical, secret-bearing, path-unsafe, or duplicate/unknown record data. |
| rejected | M6C002 | Unsupported schema, profile, chain, observer policy, or client-policy version. |
| rejected | M6C003 | Source/proof/schema/ABI/artifact/release/request/policy/Safe identity mismatch. |
| rejected | M6C004 | Transaction, constructor, derived address, receipt, code, storage, event, or finality comparison mismatch. |
| unaccepted | M6C005 | Missing, pending, reverted, replaced, conflicting, or late receipt. |
| unaccepted | M6C006 | Independent-observer disagreement or detected reorganization. |
| unaccepted | M6C007 | Expected unauthorized invocation had a successful transition, or an observation kind/action combination is not permitted. |

Unknown status or diagnostic values reject. No status means deployed,
final, verified, proved, safe, or production-ready.

## Fixture Contract

The companion fixture contract locks the required fields, five observation
kinds, allowed replay status/diagnostic pairs, and the non-secret retention
boundary. Phase 52.2 must implement exactly this contract; Phase 52.3 supplies
the corresponding tamper corpus.

## References

- docs/design/phase-52.0-safe-registry-correspondence-lock.md
- docs/design/phase-51.1-safe-transaction-request-format.md
- docs/design/phase-42.2.2-pilot-evidence-capture.md
- docs/design/phase-42.2.3-pilot-offline-replay.md
- docs/fixtures/phase-52.1/safe-registry-evidence-schema-fixtures.json

