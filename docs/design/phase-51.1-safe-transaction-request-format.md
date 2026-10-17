# Phase 51.1 - Safe Transaction-Request and Policy-Evidence Format

Date: 2026-10-17

Status: Locked offline record contract. It authorizes fixtures, parsing, and
local policy validation only. It authorizes no endpoint access, Safe setup, key
access, signing, transaction construction, broadcast, deployment, invocation,
or mainnet claim.

## Purpose and Scope

This contract makes a production action reviewable before a signer or adapter
can act. It defines two separate documents:

1. 'clg.safe-transaction-request.v1': one immutable proposed action.
2. 'clg.safe-transaction-policy-evidence.v1': non-secret evidence that
   approves that exact request.

The separation avoids a circular dependency: the canonical request is hashed
first; each approval then binds its digest. A request is not an approval, a
signature, a wallet configuration, a command to a target, or proof that a
target accepted anything.

This is a production-only contract for
'clg.evm-ethereum-mainnet-safe-registry.v1' on chain ID '1'. Development and
pilot/staging material must use their own tools and records; it is rejected by
this contract rather than coerced into a production request.

## Canonical Encoding and Shared Rules

Both documents use UTF-8 canonical JSON.

- 'format' and 'schema_version' are required and must respectively equal the
  document name and numeric '1'. Unknown fields and duplicate keys at every
  object depth are rejected.
- A document digest is SHA-256 of its canonical UTF-8 JSON bytes, rendered as
  exactly 64 lowercase hexadecimal characters. A referenced digest must match
  the retained bytes exactly.
- EVM quantities are decimal strings, not JSON numbers: ASCII digits only,
  with no sign, whitespace, or leading zero except '0'.
- An address is '0x' followed by exactly 40 lowercase hexadecimal characters.
  A byte string is '0x' followed by an even number of lowercase hexadecimal
  characters. SHA-256 values omit the '0x' prefix.
- A file reference is a non-empty forward-slash relative path with no drive
  prefix, backslash, absolute prefix, '.' segment, or '..' segment. A
  validator resolves it only beneath its supplied artifact root.
- A timestamp is an RFC 3339 UTC instant using 'Z'. 'expires_at' must be after
  'created_at'; an expired request is rejected rather than extended.
- 'submission_endpoint' is an explicit 'https' URL with no username, password,
  query, or fragment. Validation does not contact it. It is operational
  metadata, not a credential or endpoint-honesty assertion.
- Every address array is strictly increasing lexical order with no duplicate.

The validator rejects aliases, defaults, inferred values, and any record that
fails these rules. Its report must not expose a secret, absolute local path, or
endpoint credential.

## Common Request Envelope

Every request has these fields:

| Field | Required value or rule |
| --- | --- |
| format | 'clg.safe-transaction-request.v1' |
| schema_version | numeric '1' |
| request_id | 'clgreq-' followed by 64 lowercase hexadecimal characters; it is unique and non-secret across all environments |
| environment | exactly 'production' |
| target | profile 'clg.evm-ethereum-mainnet-safe-registry.v1', chain_id '1', and explicit credential-free submission_endpoint |
| created_at / expires_at | UTC timestamps as described above |
| release | path-safe bundle and verifier-result paths, each bound to its SHA-256 digest |
| policy | path-safe operator-policy path and digests for that policy and the Safe-owner amendment |
| action | exactly one closed action object defined below |

The release and policy fields bind exact review inputs, not a branch, tag, URL,
or ambient configuration. The two action forms are exclusive; no generic call,
cancellation, replacement, owner-management, or value-transfer object exists.

## Registry Deployment Action

'action.kind' may be 'registry_deploy' only for the one direct creation. It
contains:

| Field | Rule |
| --- | --- |
| deployment_sender / deployment_nonce | Explicit, separate hardware-backed deployer address and explicit EOA nonce. The sender is neither a Safe owner nor executor. |
| value_wei / gas_limit | Value is exactly '0'; gas limit is a nonzero decimal quantity. |
| fee | Exact object: kind 'legacy-eip155-gas-price' and a nonzero gas_price_wei. |
| creation_calldata / creation_calldata_sha256 | Exact byte string and digest; the validator decodes it to the reviewed registry creation bytecode and constructor. |
| constructor | owner_safe_address and initial_value_u64. The owner address equals the Safe identity address. |
| safe_identity | Exact Safe object described below. |

'safe_identity' contains safe_address, safe_version '1.4.1', proxy runtime-code
digest, singleton address and runtime-code digest, exactly three sorted owner
addresses, threshold '2', explicit safe_nonce, empty modules, and null guard
and fallback_handler. The outer creation is legacy EIP-155 on chain ID '1' and
has zero native value.

## Safe Registry-Revision Action

'action.kind' may be 'safe_registry_set_value' only for the one post-launch
mutation. It contains:

| Field | Rule |
| --- | --- |
| executor_sender / executor_nonce | Explicit executor EOA and nonce. The executor is distinct from every Safe owner and the deployment sender. |
| outer_value_wei / outer_gas_limit / outer_fee | Value is exactly '0', gas limit is nonzero, and fee is the legacy EIP-155 object above. |
| safe_identity | The exact Safe object described for registry deployment. |
| safe_transaction.safe_nonce | Equals safe_identity.safe_nonce; it is never inferred or automatically incremented. |
| safe_transaction.to / value_wei / operation | The recorded registry address, exactly '0', and exactly 'CALL'. |
| safe_transaction.set_value_next_u64 | One U64 decimal quantity. |
| safe_transaction.set_value_calldata / set_value_calldata_sha256 | Exact byte string/digest that decodes to canonical ABI 'set_value(next)' calldata, with selector '0x13765838', and the stated next value. |
| safe_transaction.safe_tx_digest | Explicit 32-byte Safe transaction digest. |
| safe_transaction.signature_bundle_path / signature_bundle_sha256 | Path-safe local input and digest. It binds the Safe digest and at least two distinct members of the frozen owner set in canonical Safe ordering. |

EIP-1559, delegatecall, batching, modules, guards, arbitrary Safe calldata,
automatic nonce management, and nonzero inner or outer value are rejected.

## Policy-Evidence Document

A policy-evidence record has these fields:

| Field | Required rule |
| --- | --- |
| format / schema_version | 'clg.safe-transaction-policy-evidence.v1' and numeric '1' |
| transaction_request_sha256 | Exact digest of one request document |
| environment / target_profile / chain_id | Exactly 'production', the named profile, and '1' |
| release_bundle_sha256 / operator_policy_sha256 | Exact equality with the request bindings |
| approval_kind | 'safe_owner_deployment_consent' for deployment or 'safe_exec_transaction' for a revision |
| required_owner_count / frozen_owner_set_sha256 | Exactly '2' and the digest of the request Safe owner set |
| approvals | At least two distinct, sorted owner records, each with public owner address, path-safe approval-record reference/digest, approved_at, and expires_at |
| review_disposition | Exactly 'approved' |

Every approval binds the request digest, environment, profile, chain, release,
policy, action contents, and expiry. It expires no later than the request and
cannot be copied into another request or action. Approval records and
signature bundles contain no private key, seed, credential, hardware export,
recovery data, or absolute machine path. A raw signature is not a secret, but
is never placed in source fixtures or human-readable reports.

## Stable Rejection Labels

| Label | Rejection condition |
| --- | --- |
| M6B001 | Malformed schema, unsupported version, unknown/duplicate field, or non-canonical encoding. |
| M6B002 | Environment, profile, chain ID, release, or policy binding is missing or mismatched. |
| M6B003 | Unsafe path, endpoint credential/shape, secret marker, or prohibited environment data is present. |
| M6B004 | Request ID, nonce, digest, approval, or signature bundle is stale, repeated, substituted, expired, or inconsistent. |
| M6B005 | Safe identity is not version 1.4.1, has anything other than three sorted owners/threshold 2, or has disallowed configuration. |
| M6B006 | A deploy action violates sender separation, creation/constructor binding, legacy EIP-155 fields, or zero-value. |
| M6B007 | A Safe revision violates executor separation, Safe nonce, registry target, CALL, canonical calldata, signature threshold, or zero-value. |
| M6B008 | The action kind is not one of the two closed forms, or attempts a generic call, governance change, cancellation, replacement, or implicit discovery. |

## Retention and Test Corpus

Only request/evidence records, release/policy digests, public addresses, and
sanitized validator output are retained by default. The local root holding a
signature bundle or endpoint configuration is not a repository artifact. A
secret/path scan occurs before acceptance.

The companion corpus locks two positive shapes and representative negative
cases, including cross-environment approval reuse. It contains no live
endpoint, credential, key, signature, Safe address, or deployable calldata.
Phase 51.2 must implement this contract against the corpus before an adapter
can receive an accepted request.

## References

- 'docs/design/phase-51.0-safe-operator-and-environment-policy.md'
- 'docs/design/phase-50.2-safe-owner-amendment.md'
- 'docs/design/phase-42.0.2-sepolia-pilot-profile-lock.md'
- 'docs/fixtures/phase-51.1/safe-transaction-request-fixtures.json'

