# Phase 42.0.2 - Sepolia Pilot Profile Lock

Date: 2026-10-14
Status: Locked
Owner: target-runtime-owner

## Decision

The sole live-network profile for the Milestone 5 pilot is:

| Field | Locked value |
| --- | --- |
| ClearLang target profile | `clg.evm-sepolia-pilot.v1` |
| Network | Ethereum Sepolia testnet |
| Chain ID | decimal `11155111`; canonical JSON-RPC quantity `0xaa36a7` |
| Contract execution profile | `clg.evm-stateful-scalar.v1` |
| Transaction envelope | locally signed legacy EIP-155 with explicit scalar `gasPrice` |
| Network purpose | Testnet-only caller-authorized configuration-registry pilot |

The former unreleased generic adapter identity, `clg.evm-compatible.v1`, is not accepted by the
pilot profile. Existing artifacts must be regenerated so the artifact, wire ABI, release bundle,
and receipt bind `clg.evm-sepolia-pilot.v1` exactly. This is a pre-release replacement, not a
compatibility migration.

Ethereum identifies Sepolia as its default testnet for contract and application development; that
published network role makes it the appropriate single candidate for this application-contract
pilot. It is not evidence of any particular endpoint's behavior. [Ethereum network
guidance](https://ethereum.org/developers/docs/networks)

## Enforced Chain Binding

Every `clg target deploy`, `clg target call`, and `clg target invoke` invocation must provide both
the locked profile identifier and chain ID `11155111`. The CLI rejects another chain ID during
local preflight, before opening an RPC connection or signing/submitting a transaction. It then
calls `eth_chainId` before any transaction submission and rejects a response other than
`0xaa36a7`.

Thus an operator cannot turn this pilot into a mainnet, Hoodi, local-chain, or arbitrary RPC
operation by changing only `--chain-id`. Artifact and wire-ABI profile equality remains an
independent pre-submission check.

## Permitted RPC Contract

The profile permits only this request surface:

| Method | Exact use | Required outcome |
| --- | --- | --- |
| `eth_chainId` | Empty parameters before `call`, `deploy`, or `invoke`. | Canonical quantity `0xaa36a7`. Any other or malformed quantity rejects. |
| `eth_call` | One canonical call object and literal block tag `"latest"`; read-only target operation only. | `0x`-prefixed bytes accepted by the wire ABI decoder. |
| `eth_sendRawTransaction` | One locally signed legacy EIP-155 byte string for explicit deploy/invoke. | Canonical 32-byte transaction hash exactly equal to the local raw-transaction hash. |
| `eth_getTransactionReceipt` | The returned transaction hash only; at most ten polls, 100 ms apart. | `null` while unavailable, or an object with matching `transactionHash`, `status`, `blockNumber`, and `blockHash`; deployment also requires canonical `contractAddress`. |

The adapter does not use `eth_sendTransaction`, `eth_estimateGas`, `eth_gasPrice`, account or
wallet discovery, EIP-1559 envelopes, transaction replacement, confirmation-depth checks, or
client-specific RPC extensions. Their availability cannot broaden this profile.

An RPC error, an unlisted method requirement, a malformed result, wrong transaction hash,
receipt status `0x0`, unavailable receipt after the bound polls, or a changed chain ID fails
closed and writes no successful `clg.target-receipt.v1` file.

## Receipt Observation Rule

A successful target receipt records one endpoint observation, not finality. It is accepted only
after a matching successful receipt supplies a well-formed block number and block hash under the
bounded polling rule above. No confirmation depth, safe/finalized head, chain-consensus,
reorganization, provider-honesty, or permanence assertion is made.

The retained ClearLang receipt binds the exact signed request, caller/sender, selected profile,
chain ID, artifact/wire-ABI/proof/schema/release identities, observed transaction hash, and block
reference. It does not prove that another endpoint would report the same data.

## Endpoint and Client Observation Boundary

An endpoint is an explicit deploy-operator input, never a default, discovery result, or release
artifact field. It must be an HTTP(S) URL without embedded credentials. Endpoint TLS and a
successful response protect transport selection only; they do not establish endpoint honesty,
client correctness, consensus correctness, or finality.

No client implementation or version is universally approved by this lock. The compatible
observation contract is behavioral: the endpoint must satisfy the four exact method/response
rules above for the captured fixture sequence. In 42.0.3, each sanitized packet will retain any
provider-disclosed client/version string only as untrusted descriptive metadata. ClearLang neither
calls a client-version RPC method nor treats a version banner as conformance proof. This makes a
changed client/version an observation that requires re-evaluation, rather than an ambient
compatibility promise.

## Faucet and Key Policy

1. Use a new, testnet-only secp256k1 signer and obtain only testnet funds through an operator's
   independently reviewed Sepolia faucet process.
2. Keep the signer JSON, private key, faucet credentials, endpoint credentials, and any operator
   account data outside the repository, fixtures, release bundle, receipts, reports, shell
   history, and support logs.
3. The explicit `--sender` must equal the address derived from that signer. A signer is not a
   release-evidence key and has no mainnet or custody authorization.
4. A suspected exposure stops all submissions under the signer; preserve non-secret evidence,
   retire the test key, and use a freshly generated test key only after operator review.
5. Faucet availability, limits, and account balances are operational inputs, not release gates.

## Known Incompatibilities and Non-Claims

This lock does not add generic Sepolia, generic EVM, EIP-1559, wallet, remote signer, value
transfer, external-call, proxy, dynamic ABI, or mainnet support. It does not claim a deployment
is production-ready or that a successful receipt represents consensus finality.

The profile remains limited to the caller-aware scalar artifact and the four RPC methods above.
Any request that needs another chain, client-specific behavior, or transaction envelope requires
a new design lock and a separately versioned target profile.

## Required 42.0.3 Evidence

42.0.3 must capture secret-free fixtures for every permitted success and failure path and prove
locally that the adapter accepts only the locked response contract. It must include wrong-chain,
wrong-hash, malformed receipt, reverted receipt, unavailable receipt, malformed call result, and
unlisted/malformed RPC response cases. No live endpoint may be contacted by that local test.

## Design Principles Check

- **Simple for users:** one profile and one chain eliminate ambiguous network selection.
- **AI-friendly:** an immutable name, fixed quantity, method table, and pre-submission failure
  rule make valid invocations and repairs mechanically unambiguous.
- **Provably correct:** artifact identity, explicit chain preflight, response validation, and
  receipt binding fail closed rather than accepting a near match.
- **Crypto-focused:** key separation, caller/sender binding, signed transaction identity, and
  receipt evidence take priority over broad client or chain features.

## Exit Criteria

1. `clg.evm-sepolia-pilot.v1` and chain ID `11155111` are emitted and accepted consistently by
   build, release, target, and receipt paths.
2. A non-Sepolia chain ID is rejected before RPC contact or transaction submission, with local
   unit and CLI coverage.
3. The exact RPC, receipt, key, faucet, endpoint, client-observation, incompatibility, and
   non-claim policies are published.
4. No endpoint, credential, private key, faucet token, or live-network action is committed.

## References

- `README.md`
- `docs/contract-operations.md`
- `docs/contract-product-contract.md`
- `docs/design/phase-30.3.3.4.0-evm-transaction-signing-policy-lock.md`
- `docs/design/phase-40.0-milestone-5-pilot-product-lock.md`
- `docs/design/phase-42.0.1-testnet-conformance-questionnaire.md`
- `docs/todo/milestone_5_roadmap.md`
