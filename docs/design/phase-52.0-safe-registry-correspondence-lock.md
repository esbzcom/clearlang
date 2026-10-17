# Phase 52.0 - Safe Registry Correspondence and Target-Observation Lock

Date: 2026-10-17

Status: Locked design for evidence capture and offline replay. It authorizes no
RPC contact, Safe deployment/configuration, signer access, transaction
construction, signing, broadcast, registry deployment, invocation, or mainnet
claim.

## Scope

This lock defines what a later evidence record must show before it may be
called a qualified target observation for the named Configuration Registry
profile on Ethereum Mainnet. It replaces the authority portions of the retained
direct-owner target lock with the Safe-owner policy in Phase 50.2. It does not
select a Safe address, endpoint, client, owner, deployer, executor, constructor
value, registry address, or transaction.

A response from an endpoint is evidence of that endpoint observation only. It
does not prove Ethereum consensus, client correctness, finality, signer
custody, Safe correctness, source correctness, or production readiness.

## Identity Chain

Every later deployment or revision evidence record must bind this ordered
chain. Every link is an exact value or digest; no ambient checkout, branch,
wallet, endpoint default, or inferred network is accepted.

| Order | Identity | Required correspondence |
| --- | --- | --- |
| 1 | Source/profile/proof | The reviewed v1 ConfigRegistry source, caller-authorization profile, proof artifacts, compiler/solver identities, and proof result. |
| 2 | State schema and ABI | The release-bound schema maps immutable owner and U64 value; the ABI binds init(owner, initial_value), set_value(next), and ValueChanged(next). |
| 3 | EVM artifacts | The release binds exact creation bytecode, expected runtime bytecode, constructor ABI shape, state-storage layout identity, and event ABI identity. |
| 4 | Release verification | The independently verified bundle and verifier result bind all preceding identities. |
| 5 | Operator request/evidence | The accepted Phase 51 request, policy evidence, and validator report bind profile, chain ID 1, legacy envelope, zero value, action shape, expiry, and Safe policy. |
| 6 | Safe identity | Safe proxy and singleton addresses/runtime-code digests, version 1.4.1, exactly three ordered public owners, threshold 2, explicit Safe nonce, no modules, no guard, and no fallback handler. |
| 7 | Signed action identity | Deployment: explicit deployer/nonce and constructor owner equal to the Safe. Revision: Safe typed transaction digest, two-or-more frozen-owner signature identity, Safe execTransaction calldata, inner registry target, value zero, CALL, and canonical set_value calldata. |
| 8 | Target transaction identity | Locally calculated transaction hash, sender, nonce, legacy EIP-155 fields, zero value, to/input, and the endpoint-returned transaction hash must match. |
| 9 | Receipt and address | Matching successful receipt, receipt block number/hash, creation contract address derived from sender and nonce, or recorded registry address for an invocation. |
| 10 | Final state/event | Runtime code, schema-defined storage words, Safe configuration reads, and required ValueChanged(next) event bind to the receipt block and later finalized observation. |
| 11 | Independent confirmation | A separately operated observer reports the same chain, transaction, receipt, code/state/event, and finality identities. |

A mismatch at any link is a stop result. Evidence may name the mismatch but
must not repair an input, select a replacement transaction, take a majority
vote, or silently broaden the profile.

## Required Correspondence Checks

### Deployment

A later deployment record must prove, by exact comparison, all of these:

1. The request action is the direct zero-value registry deployment and its
   creation input equals the release creation bytecode followed by the
   ABI-encoded constructor owner and initial U64 value.
2. Constructor owner equals the reviewed Safe proxy address. The Safe reads
   satisfy the identity in link 6 before the deployment is accepted.
3. The contract address equals the Ethereum CREATE address derived from the
   observed deployer address and explicit deployer nonce: the low 20 bytes of
   Keccak-256(RLP([sender, nonce])).
4. The receipt has the expected transaction hash, status 0x1, contract address,
   block number, and block hash. The observed transaction has the approved
   legacy EIP-155 fields, no recipient, zero value, and exact creation input.
5. Runtime code at the derived address has the release-bound byte digest.
   Empty code, creation code, metadata-only comparison, or a merely similar
   byte sequence is rejected.
6. At the receipt block, every state word named by the release-bound state
   schema has the approved initial encoding: owner equals the Safe and value
   equals initial_value. This lock does not guess numeric storage slots; the
   release-bound schema supplies their locations and encodings.
7. The finality and independent-observer requirements below are met.

### Safe-Executed Registry Revision

A later revision record must prove all of these:

1. The outer legacy transaction invokes the pinned Safe execTransaction ABI
   with the request-bound Safe typed transaction digest and signature-bundle
   identity. Its sender/executor has no registry authority by itself.
2. The inner action has the recorded registry address, value zero, operation
   CALL, and exact canonical set_value(next) calldata. Delegatecall, batching,
   another target, nonzero value, or different calldata rejects.
3. The Safe configuration reads at the observation block still match link 6,
   including proxy/singleton code, owners, threshold, nonce progression, and
   absence of modules, guard, fallback handler, or delegated authority.
4. The successful receipt/log set includes exactly the release-bound
   ValueChanged(next) event at the registry address, with the ABI-decoded U64
   equal to the request next value. Extra or missing matching events reject.
5. State words named by the release schema show the fixed owner unchanged and
   value equal to next. Runtime code remains equal to the release-bound digest.
6. The finality and independent-observer requirements below are met.

An expected unauthorized invocation, where later explicitly authorized for
testnet soak, is never deployment or revision acceptance evidence. It must
show no successful state/event transition and remains a separate negative
exercise record.

## Observation Surface and Independent Observers

The only future observation methods are eth_chainId, eth_getTransactionByHash,
eth_getTransactionReceipt, eth_getTransactionCount, eth_getBlockByHash,
eth_getBlockByNumber, eth_getCode, eth_getStorageAt, and credential-free
eth_call for the canonical registry and Safe reads. Submission is outside this
lock and remains a later explicitly approved action.

Each observation record must include:

- an opaque observer identifier, independent-operator classification, client
  family/version as disclosed metadata, capture timestamp, and exact chain-ID
  result 0x1;
- a canonical request digest and sanitized response digest for each allowed
  method, with endpoint credentials, headers, IP addresses, and absolute paths
  omitted;
- the block number/hash used for each code, storage, event, and Safe read; and
- the named release/request/policy/Safe identities that the response is being
  compared against.

The submission observer and independent verification observer must be
separately operated. Agreement is exact identity equality, not a majority
vote. A missing, malformed, unavailable, substituted, or disagreeing second
observation produces preservation-and-stop. It never permits automatic retry
or a weaker one-observer acceptance.

## Finality and Receipt Status

The observation policy is fixed:

| Condition | Deterministic status |
| --- | --- |
| Expected receipt absent during 30 polls at 60-second intervals | missing_receipt; unaccepted and stop |
| Receipt is pending or lacks required fields | pending_receipt; unaccepted and stop |
| Receipt status is not 0x1 | reverted_receipt; unaccepted and stop |
| Expected hash absent while pending nonce exceeds request nonce | replaced_or_conflicting; unaccepted and stop |
| Transaction/receipt/block/Safe/code/storage/event identities conflict | target_mismatch; unaccepted and stop |
| Receipt block is fewer than 12 blocks behind observed head, or finalized head is below it | awaiting_finality; unaccepted and continue observing only within the fixed window |
| Receipt block/header remains canonical, finalized head is at or above it, and two further 60-second observations agree | qualified_finality_observation; still only target-observation evidence |
| Any independent-observer disagreement or later reorganization | observer_disagreement_or_reorg; unaccepted and stop |

After first qualified finality observation, retain two additional observations
at 60-second intervals. A block/hash/height change, a moved/reverted receipt,
or a finalized-head regression is a stop. Automatic replacement, cancellation,
fee bumping, nonce increment, resubmission, or acceptance of another hash is
prohibited.

## Evidence Boundary and Retention

Capture only the fields required above. Preserve canonical request/response
bytes or their digest-addressed retained forms, public addresses, code/storage
values, ABI-decoded values, receipt/header identities, comparison results, and
sanitized client metadata. Never retain private keys, seeds, recovery data,
endpoint credentials, raw authorization headers, exact device locations, or
absolute paths.

Later Phase 52 schemas must represent missing, pending, reverted, replacement,
conflict, disagreement, reorganization, and unknown schema/profile/client
versions as explicit non-accepted results. Unknown data does not inherit an
earlier acceptance.

## Non-Claims

This lock does not prove a Safe signature cryptographically, establish source
or chain correctness, authorize endpoint contact, choose a finality provider,
or authorize deployment/invocation. It is the evidence vocabulary for later
offline replay and target observation work only.

## References

- docs/design/phase-50.2-safe-owner-amendment.md
- docs/design/phase-51.1-safe-transaction-request-format.md
- docs/design/phase-50.2-ethereum-mainnet-target-profile-lock.md
- docs/design/phase-41.0-caller-authorization-profile-lock.md
- docs/fixtures/phase-52.0/safe-target-observation-lock-fixtures.json

