# Phase 50.2 - Ethereum Mainnet Registry Target-Profile Lock

Date: 2026-10-17

Status: Locked for Gate A planning and local fixture validation only. This
document authorizes no endpoint contact, signer use, transaction construction,
deployment, invocation, or mainnet submission.

Owner: target-runtime-owner and product-and-runtime-owner

## Decision

The sole Milestone 6 target profile is
`clg.evm-ethereum-mainnet-registry.v1`:

| Field | Locked value |
| --- | --- |
| Network | Ethereum Mainnet only |
| Chain ID | Decimal `1`; canonical JSON-RPC quantity `0x1` |
| Product profile | Fixed-owner scalar Configuration Registry from Phase 50.0 |
| Source execution profile | `clg.evm-stateful-scalar.v1` with Phase 41.0 caller authorization |
| Future distribution identity | A later Target Profile Pack may distribute this policy as `clg.target.evm.ethereum-mainnet.v1`; that packaging is post-Milestone-6 work and does not change this lock. |
| Transaction envelope | Locally signed legacy EIP-155 transaction only; no EIP-1559 or account-managed transaction. |
| Native value | Exactly `0x0` in every deployment and invocation request. |
| Mainnet authority | None at Gate A. Only Phase 54.4 can submit one limited approved action. |

The selection record in
[`phase-50.2-ethereum-mainnet-selection.md`](phase-50.2-ethereum-mainnet-selection.md)
remains the product rationale. This document locks the target policy that the
selection record left open.

## Explicit Transaction and Invocation Envelope

The profile permits only locally constructed, locally signed legacy EIP-155
raw transactions with all of these fields explicitly present in a later
canonical request record:

| Field | Policy |
| --- | --- |
| `chain_id` | Exactly `1`; the raw signed transaction and prior `eth_chainId` response must agree. |
| `nonce` | Exact canonical quantity supplied in the approved request. Tooling may compare an observation but must not discover, increment, or replace a nonce automatically. |
| `gas_price` | Exact canonical quantity supplied in the approved request. `eth_gasPrice`, fee estimation, EIP-1559 fee fields, and implicit fee defaults are prohibited. |
| `gas_limit` | Exact canonical quantity supplied in the approved request. Estimation and expansion after approval are prohibited. |
| `value` | Exactly `0x0`; any nonzero value rejects before signing. Transaction fees are an approved operator cost, not contract-held value or product asset exposure. |
| `to` and `data` | Direct creation has no `to` and carries only the approved constructor bytes. Invocation has the recorded deployed registry address and only the canonical `set_value(U64)` bytes. Reads use the canonical ABI call and never create a transaction. |
| signer/sender | Supplied and approved explicitly under Phase 51. No wallet discovery, remote signing, account-managed `eth_sendTransaction`, or signer inference is permitted. |

The only source/template is
`examples/projects/reference-config-registry/v1/config_registry.clear` under
the inherited proof/release policy. Construction accepts the reviewed
`init(owner, initial_value)` inputs; `owner` must be one explicit address and
`initial_value` one explicit `U64`. This lock intentionally provides no
default owner or initial value. They become exact values only in the approved
Phase 54 request record.

After deployment, the only permitted mutable calldata is one owner-authorized
`set_value(next)` with one explicit `U64` `next`. The profile does not permit
owner rotation, a second constructor, proxy/delegate behavior, external calls,
value transfer, or an unreviewed ABI selector. The first launch envelope is
one direct deployment only; any later invocation needs its own approved request
and does not inherit launch approval.

## RPC, Endpoint, and Client-Observation Contract

The target client may use only the following Ethereum JSON-RPC methods. A
method not listed here, a malformed result, an RPC error in a required step, or
a changed chain ID is a fail-closed stop condition.

| Method | Permitted use |
| --- | --- |
| `eth_chainId` | Before every read, request submission, and observation sequence; result must be exactly `0x1`. |
| `eth_call` | Read only the registry through canonical ABI calldata at literal block tag `"finalized"`. |
| `eth_sendRawTransaction` | Submit exactly one locally signed, approved legacy EIP-155 raw request after Phase 54.3 approval. The returned hash must equal the locally calculated hash. |
| `eth_getTransactionReceipt` | Observe only the expected transaction hash. A success receipt must have matching hash, `status: 0x1`, canonical block number, and 32-byte block hash; deployment also requires one contract address. |
| `eth_getTransactionByHash` | Confirm that the expected hash, raw transaction fields, sender, nonce, `to`, `value`, and input match the approved request. |
| `eth_getTransactionCount` | Compare the explicit request nonce with the sender's `"pending"` count to detect a possible same-sender conflict; never choose a nonce. |
| `eth_getBlockByHash` | Re-read the receipt block and require its hash and number to match the receipt. |
| `eth_getBlockByNumber` | Obtain the `"finalized"` block header and verify the receipt block is canonical at its recorded height. |
| `eth_getCode` and `eth_getStorageAt` | Phase 52 correspondence checks only, using the final receipt block or `"finalized"` tag. |

An endpoint is an explicit, externally provisioned operator input. It must use
HTTPS, be allowlisted by the approved environment, and contain no credential
in its URL. Authentication material may enter only from the approved secret
boundary (for example, an HTTP authorization header or mTLS credential); it
must never enter source, profile manifests, requests, fixtures, release
bundles, command history, reports, or retained evidence. TLS and a valid
response establish transport selection only, not endpoint honesty, client
correctness, chain consensus, or finality.

Before any candidate can be accepted, Phase 53 must retain secret-free
client-observation records for the submission observer and an independently
operated verification observer. Each record names the provider/operator,
execution-client family and version as disclosed by that operator, timestamp,
chain-ID result, and supported-method result. The records are descriptive
compatibility evidence, not a proof that a client is correct. A client family
or version change requires support-policy re-evaluation and a fresh
production-precheck; no version banner is accepted as conformance by itself.

## Confirmation, Finality, and Failure Rule

A receipt is only an inclusion observation. It is not a successful mainnet
result until all of the following hold:

1. the expected successful receipt is observed at least **12 blocks** after its
   recorded block number;
2. the receipt block hash and number still match `eth_getBlockByHash` and the
   canonical header returned at that block number;
3. `eth_getBlockByNumber("finalized", false)` reports a finalized header at a
   height equal to or greater than the receipt block; and
4. an independently operated verification observer produces the same
   chain-ID, transaction, receipt, receipt-block, and finalized-head identity.

Poll no more often than once per **60 seconds**. The initial observation window
is **30 polls**. After the first qualified finalized observation, retain two
additional observations at 60-second intervals. This is the reorganization
detection window. A missing receipt after the window, a receipt that moves or
reverts, a conflicting block/hash/height, a finalized head below the receipt,
or any observer disagreement fails closed. Preserve secret-free evidence,
declare the request unaccepted, and escalate to the incident owner.

If the expected transaction is absent while the observed pending nonce has
advanced beyond its explicit nonce, treat it as a potential conflicting or
replacement transaction. Automatic replacement, cancellation, fee bumping,
resubmission, or accepting an alternative hash is prohibited. A later action
requires a fresh canonical request and Phase 51 approval; it does not repair
the failed request by implication.

Ethereum's `"finalized"` block tag and proof-of-stake finality model are used
as an endpoint observation rule, not as an assertion that ClearLang proves
consensus or endpoint honesty. Ethereum documentation describes `"finalized"`
as a block parameter tag and describes finalized proof-of-stake blocks as
crypto-economically final absent a consensus failure. [Ethereum JSON-RPC
API](https://ethereum.org/developers/docs/apis/json-rpc/)
[Ethereum finality overview](https://ethereum.org/developers/docs/consensus-mechanisms/pos/faqs)

## Claim Terminology and Evidence Boundary

`docs/fixtures/phase-50.2/ethereum-mainnet-claim-terminology-fixtures.json`
is the versioned positive/negative fixture for this lock. It fixes qualified
claims for source proof, release verification, local replay, target observation,
and the eventual production-profile decision. It rejects bare claims such as
“production-ready”, “mainnet-ready”, “verified”, and “proved safe” when they
are used without their evidence layer and named profile/chain boundary.

The fixture is validated locally by the `xtask` document-consistency test. It
does not convert a documentation fixture, source proof, verifier result,
endpoint observation, or finality observation into a production claim.

## Non-Claims and Exit Criteria

This lock does not implement a mainnet target adapter, contact an endpoint,
select a provider, approve an endpoint, provision a signer, set a fee cap,
approve a concrete owner/initial value, or authorize a transaction. It does not
add EIP-1559, automatic fee/nonce selection, arbitrary Ethereum clients,
generic EVM compatibility, generic multi-chain support, or any language,
standard-library, proof, IR, code-generation, or verifier feature.

Phase 50.2 exits when this lock and its terminology fixture are locally
validated and the roadmap records the single profile. Gate A then permits only
the local policy work of Gate B; it still permits no target action.

## References

- `docs/design/phase-50.0-milestone-6-production-profile-lock.md`
- `docs/design/phase-50.2-ethereum-mainnet-selection.md`
- `docs/fixtures/phase-50.2/ethereum-mainnet-claim-terminology-fixtures.json`
- `docs/design/phase-41.0-caller-authorization-profile-lock.md`
- [EIP-155 replay protection](https://eips.ethereum.org/EIPS/eip-155)
- [Ethereum JSON-RPC API](https://ethereum.org/developers/docs/apis/json-rpc/)
