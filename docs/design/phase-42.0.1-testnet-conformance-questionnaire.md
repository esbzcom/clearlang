# Phase 42.0.1 - Testnet Candidate Conformance Questionnaire

Date: 2026-10-14
Status: Assessment complete; profile selection pending 42.0.2
Owner: target-runtime-owner

## Purpose

Evaluate public EVM testnet candidates for the narrowly supported
`clg.evm-stateful-scalar.v1` pilot without turning an advertised network, a public RPC endpoint,
or an Internet connection into a ClearLang acceptance dependency. This document records the
questionnaire, candidate facts, and the evidence that a candidate must provide before it can be
selected in 42.0.2.

This is not a target-profile selection, an endpoint approval, or a live-network compatibility
claim. 42.0.2 selects exactly one profile only after the locally replayable evidence below exists;
42.0.3 captures and tests that evidence without contacting the candidate endpoint.

## Inherited Compatibility Boundary

The pilot may exercise only the existing explicit target adapter:

| Item | Required behavior |
| --- | --- |
| ClearLang target profile | `clg.evm-stateful-scalar.v1` through `clg.evm-sepolia-pilot.v1` |
| Transaction envelope | Locally signed legacy EIP-155 transaction with explicit nonce, value, gas limit, and scalar `gasPrice`; EIP-1559 is not supported. |
| EVM program | The generated caller-aware scalar artifact, including direct constructor, `CALLER` guard, `SLOAD`/`SSTORE`, and canonical event behavior. It is the concrete London-opcode profile to test, not arbitrary bytecode. |
| JSON-RPC methods | `eth_chainId`, `eth_call`, `eth_sendRawTransaction`, and `eth_getTransactionReceipt` only. |
| Success receipt | Matching transaction hash, `status: "0x1"`, canonical block number/hash, and `contractAddress` for deployment. A reverted (`"0x0"`), malformed, absent, or mismatched receipt is never success evidence. |
| Inputs | Endpoint, chain ID, sender, signing key, nonce, gas limit, scalar gas price, value, artifact/evidence set, and receipt path remain explicit operator inputs. |

The questionnaire must reject a candidate that requires account-managed `eth_sendTransaction`,
EIP-1559 replacement, ambient fee estimation, automatic nonce discovery, or an RPC method outside
this list. A testnet can support these broader features; ClearLang simply does not rely on them.

## Conformance Questionnaire

The deploy operator records one answer and sanitized observation for each row. Published network
metadata can establish a candidate fact, but cannot replace an observed response from the exact
endpoint proposed for the pilot.

| # | Question | Required local evidence | Fail-closed result |
| --- | --- | --- | --- |
| Q1 | Does `eth_chainId` return the proposed chain ID as a canonical JSON-RPC quantity? | Captured request/response and an offline fixture asserting the expected quantity. | Do not submit a transaction or select the endpoint. |
| Q2 | Does the endpoint accept the locally signed legacy EIP-155 raw deployment and invocation bytes? | Sanitized `eth_sendRawTransaction` request/response pairs whose returned hashes equal the locally calculated transaction hashes. | Reject the candidate; do not substitute an account-managed or EIP-1559 path. |
| Q3 | Does the execution environment run the emitted caller-aware scalar artifact? | A deployment plus authorized invocation observation using the release artifact; the receipt and local read/call result must match the intended scalar transition. | Reject the candidate as unproven for the locked opcode profile. |
| Q4 | Does an unauthorized invocation fail on chain? | A separately signed alternate-sender attempt observed as a revert or other non-success result, with no successful ClearLang receipt and no relabeling as authorization success. | Reject any endpoint/result that yields a successful transition for the alternate caller. |
| Q5 | Does `eth_call` return canonical `0x`-prefixed data for the supported read request? | Sanitized request/response fixture validated by the current target call adapter. | Reject the candidate for this pilot profile. |
| Q6 | Does `eth_getTransactionReceipt` provide the fields needed to bind a successful deploy/invoke? | A `null`-before-inclusion fixture where observed, followed by a receipt containing the matching transaction hash, `status`, `blockNumber`, `blockHash`, and deployment `contractAddress` where applicable. | No successful target receipt or profile selection. |
| Q7 | Can every success and failure observation be captured deterministically and replayed locally? | Ordered secret-free request/response fixtures, expected adapter outcome, and a fixture-server test. Endpoint credentials, private keys, faucet tokens, and live URLs with secrets are excluded. | The candidate remains unselected. |
| Q8 | Can the endpoint behavior be recorded without claiming stronger finality or endpoint trust than observed? | A fixed observation rule documented with the selected profile: successful receipt fields plus the number/order of receipt polls. | Reject a claim of consensus finality, provider honesty, or permanent inclusion. |

The candidate passes this phase only when the questionnaire is complete and its remaining evidence
requirements are explicit. It does **not** pass target conformance until 42.0.3 turns the captured
observations into local fixtures accepted by the current adapter and rejected when tampered.

## Candidate Assessment

The assessment considered the two public Ethereum testnets that current Ethereum documentation
identifies as maintained. No endpoint was contacted and no faucet, account, or private key was
used for this assessment.

| Candidate | Published identity / intended use | Questionnaire disposition | Reason |
| --- | --- | --- | --- |
| Ethereum Sepolia | Chain ID `11155111`; Ethereum describes it as the default testnet for contract and application development. | **Preferred candidate, pending local conformance capture.** | It matches the registry pilot's application-contract purpose. It still must satisfy Q1-Q8 at one explicit operator-supplied endpoint. |
| Ethereum Hoodi | Chain ID `560048`; Ethereum describes it for protocol developers and staking/validator testing. | **Not advanced for this application-contract pilot.** | Its published purpose does not match the narrow registry pilot as directly as Sepolia. This is not a general incompatibility claim and does not prevent a future, separately locked protocol-focused profile. |

Candidate facts above were checked on 2026-10-14 against the Ethereum network guidance and the
Hoodi configuration repository. They are discovery inputs, not ClearLang evidence:

- [Ethereum network guidance](https://ethereum.org/developers/docs/networks) distinguishes
  Sepolia for application development from Hoodi for protocol/staking work.
- [Ethereum development-network guidance](https://ethereum.org/developers/docs/development-networks/)
  lists Sepolia and Hoodi as the maintained public testnets.
- The [Hoodi chain configuration](https://github.com/eth-clients/hoodi) records chain ID
  `560048`; Sepolia's published chain ID is `11155111`.

## Required Evidence Packet for 42.0.2 and 42.0.3

Before a profile is selected, the operator must prepare a candidate packet containing only:

1. a non-secret endpoint label and its exact expected chain ID;
2. the release bundle digest, artifact digest, wire-ABI digest, and canonical caller identities
   used for the authorized and unauthorized observations;
3. sanitized ordered JSON-RPC request/response pairs for Q1-Q6, including rejection/revert and
   malformed-response paths;
4. the fixed receipt-observation rule, including polling order and bounded retry count;
5. an offline fixture manifest that binds each request to one expected response and contains no
   endpoint credential, signer key, faucet token, or live-network default; and
6. a local adapter test proving the packet is accepted only with its exact chain, request order,
   artifact identities, caller identities, and successful receipt fields.

An endpoint URL is an operator input, never a repository default. A packet must use a safe label
in committed evidence; any URL with credentials, query secrets, or private-network address is
rejected rather than sanitized by guesswork.

## Selection Gate

42.0.2 selects Sepolia as `clg.evm-sepolia-pilot.v1`. 42.0.3 may capture a packet only if it
satisfies every questionnaire row and preserves all of the following: `11155111`, the legacy
EIP-155 envelope, the four RPC
methods, compatible observed client/version data, the bounded receipt-observation rule, operator
faucet/key policy, endpoint trust boundary, and known exclusions. A chain-ID mismatch or any
different method/result fails before transaction submission.

ClearLang has **no approved live endpoint** and no claim that a public endpoint is compatible.
The deterministic local target fixture remains the only executable conformance evidence until
42.0.3 captures and validates the selected profile's offline fixture packet.

## Design Principles Check

- **Simple for users:** one short questionnaire and one candidate reduce a testnet choice to
  explicit facts instead of wallet or provider discovery.
- **AI-friendly:** ordered questions, exact method names, and concrete reject conditions make a
  future fixture and diagnostic contract mechanically checkable.
- **Provably correct:** published metadata is separated from captured behavior, and only a local
  replay of exact observations can satisfy the profile gate.
- **Crypto-focused:** the assessment protects transaction, caller, receipt, and evidence identity
  while excluding key custody, provider honesty, consensus finality, and value custody claims.

## Exit Criteria

1. The roadmap marks 42.0.1 complete and references this questionnaire.
2. The questionnaire covers chain identity, concrete London scalar-artifact execution, legacy
   transaction envelope, required RPC methods, receipt fields, and offline fixture capture.
3. Sepolia is identified only as the preferred candidate; selection, an endpoint, and network
   contact remain deferred to 42.0.2 and 42.0.3.
4. The assessment has no secret, live endpoint, faucet, or private-key material and adds no
   network dependency to local tests.

## References

- `README.md`
- `docs/contract-operations.md`
- `docs/design/phase-30.3.3.4.0-evm-transaction-signing-policy-lock.md`
- `docs/design/phase-30.3.4-evm-target-conformance-lock.md`
- `docs/design/phase-40.0-milestone-5-pilot-product-lock.md`
- `docs/todo/milestone_5_roadmap.md`
