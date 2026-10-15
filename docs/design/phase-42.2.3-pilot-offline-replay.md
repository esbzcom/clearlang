# Phase 42.2.3: Pilot offline RPC replay

`clg target replay-pilot-evidence` verifies a canonical `clg.pilot-evidence.v1` record without
opening a socket, constructing an endpoint, accessing a signer, or submitting a transaction.

```text
clg target replay-pilot-evidence --input pilot-evidence.json --report-out replay-report.json
```

The command accepts only the path to a completed capture record. It does not accept an RPC URL,
network selector, wallet, signing key, or request override. The output is canonical
`clg.pilot-evidence-replay-report.v1` and explicitly records that the network was not contacted.

## Fixture reconstruction and serving

For each captured observation, replay reconstructs the original canonical
`clg.pilot-rpc-observations.v1` fixture from the evidence record's target profile, chain ID, and
exchange list. It recomputes that fixture's SHA-256 digest before serving any response.

The in-memory fixture server has one cursor. It serves a response only when the replay's next
request exactly equals the next recorded request; extra, omitted, reordered, or altered requests
fail. The fixed request sequence is:

1. `eth_chainId` with no parameters;
2. `eth_sendRawTransaction` with the recorded published raw transaction; and
3. one or more `eth_getTransactionReceipt` requests for the transaction digest returned by step
   2.

The returned chain ID must be Sepolia (`11155111`). The returned submission hash must equal the
captured transaction digest. `deploy` and `authorized-invoke` must finish `0x1`; the mandatory
`unauthorized-invoke` must finish `0x0` and cannot have a successful target-receipt identity.
The three observations are required in that exact order.

## Failure and trust boundary

Replay rejects noncanonical evidence; unknown profile or chain; a changed fixture digest; omitted,
substituted, or reordered observations; a cross-chain response; receipt/transaction identity drift;
and any secret or endpoint field. It emits no passing report on error.

The operation proves that the retained public fixture set is internally consistent and replayable
with no live RPC dependency. It does not establish that an external RPC endpoint was honest, that
a transaction remains final, or that an unsigned evidence record has an authorization signature.
Those boundaries remain explicit in the release verification and pilot-operator processes.
