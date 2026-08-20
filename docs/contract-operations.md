# Contract Development and Operations Guide

This guide covers the currently supported ClearLang contract workflow for the locked
EVM-compatible profile. It is intentionally explicit: local simulation, target execution, and
release evidence are distinct operations. A successful simulator trace is never deployment
assurance, and a target receipt is evidence of one request rather than a complete release
attestation.

The first supported executable target profile is `clg.evm-stateful-scalar.v1` through the
explicit `clg.evm-compatible.v1` JSON-RPC adapter. It supports direct `init`, pure reads, and
straight-line mutable transitions over `Bool`, `U8`, `U64`, `U128`, and `Int`. See
`docs/design/phase-30.3.3.3.0-stateful-scalar-evm-profile-lock.md` for the exact boundary.

## 1. Roles and artifact handling

Keep these responsibilities separate:

- Developers author source, run checks, local simulation, and deterministic campaign tests.
- Release owners produce and retain the schema, ABI, EVM artifact, proof, signed bundle, and
  receipts as one evidence set.
- Target operators submit explicit deploy/invoke requests and record the returned receipt.
- Security owners control both signing-key classes and approve rotation, incident, or rollback
  actions.

Never mix artifacts from different source revisions, compiler versions, or build runs. Target
commands validate cross-artifact compiler identity, source-graph identity, contract identity,
schema digest, proof-artifact digest, and wire-ABI binding before target submission. A failure is
actionable evidence of drift; regenerate the complete evidence set rather than patching JSON.

## 2. Local development and simulation

Run the normal preflight before simulation:

```powershell
clg check contracts/counter.clear --root .
```

Emit and retain the state schema for every candidate version. Before an append-only change, check
it against the prior deployed schema:

```powershell
clg build contracts/counter-v2.clear -o out/counter-v2.wasm --check-contract-state-schema releases/counter-v1.schema.json
clg build contracts/counter-v2.clear -o out/counter-v2.wasm --emit-contract-state-schema out/counter-v2.schema.json
```

Changes that remove, reorder, or alter existing fields require an explicit migration declaration
whose `from schema` digest is the exact prior schema. Do not treat a source edit or a fresh
deployment as an upgrade procedure.

Use `clg simulate` for deterministic local transition evidence. Inputs, lifecycle, caller,
value, block context, fuel, and memory limits are always explicit or recorded in the trace:

```powershell
clg --non-interactive --json-errors --json-events simulate contracts/counter.clear --function set_total --state fixtures/counter.state.json --args fixtures/set_total.args.json --state-out out/counter.state.json --trace-out out/counter.trace.json --caller clg:caller:operator --block-number 1 --timestamp 0 --gas-limit 100000 --memory-limit 1048576
```

Use `--init` instead of `--function` for a constructor. It accepts only an empty state object
and writes no transition result. Simulation supports the locked scalar profile only; external
calls, unsupported control flow, and unsupported values fail closed. On an execution failure,
the trace includes `failure_location` and `stack`; `--json-errors` emits C142 with the same
source range for IDE and automation consumers.

For seeded property campaigns, use `clg contract test <SOURCE> --plan <FILE> --report-out <FILE>
--trace-dir <DIR>`. Preserve the plan, report, generated inputs, and trace directory together.
Every case report includes a replay `clg simulate` command; replay the first failing case before
changing source or deployment evidence.

## 3. Target artifact and evidence preparation

Generate the required artifacts from the same source graph. The relevant build outputs are:

- `--emit-contract-state-schema` for versioned storage evidence.
- `--emit-contract-abi` and `--emit-evm-wire-abi` for target-facing ABI evidence.
- `--emit-evm-artifact` for the locked deployable EVM artifact.
- `--emit-vcs` and `--emit-proof` for proof evidence.
- `--sign --key --key-id --scope both --sig-out` for the signed bundle required by target
  commands.

Run `clg build --help` before composing an expert build command: the currently supported EVM
artifact profile is intentionally narrower than the general Wasm build surface. Until
`clg contract release` is delivered, retain the explicit build invocation and every resulting
file in the change record; do not represent this manual preparation as a completed contract
release.

Verify the signed Wasm/proof evidence with the existing release verification flow before target
submission. `docs/release-process.md` documents the required strict build and `clg verify`
inputs. The later contract-release command will compose these steps, not replace their checks.

## 4. Deploy, call, and invoke

All target commands require an explicit target profile, HTTP(S) JSON-RPC endpoint, chain ID,
EVM artifact, wire ABI, state schema, proof artifact, signed bundle, and receipt output path.
They infer no endpoint, network, account, wallet, nonce, fee, or value from the environment.

Before submitting a transaction, execute the corresponding read-only `target call` where the
contract exposes an applicable pure function. `target call` never signs or submits. For deploy
and invoke, require a reviewed change ticket containing the exact sender, nonce, value, gas
limit, gas price, chain ID, artifact/evidence digests, and intended function/arguments.

Illustrative deploy shape (replace every placeholder; do not copy the values as defaults):

```powershell
clg target deploy --target-profile clg.evm-compatible.v1 --rpc-url https://rpc.example --chain-id 11155111 --sender 0x... --nonce 42 --signing-key secrets/evm-signer.json --value 0 --gas-limit 500000 --gas-price 1000000000 --artifact out/counter.evm.json --abi out/counter.evm-wire-abi.json --contract-state-schema out/counter.schema.json --proof-artifact out/counter.proof.json --signed-bundle out/counter.sig.json --args out/counter.constructor-args.json --receipt-out out/counter.deploy.receipt.json
```

`target deploy` and `target invoke` use only legacy EIP-155 envelopes in this profile. They
confirm the requested chain ID, submit only a locally signed raw transaction, and emit a
successful receipt only after a confirmed `0x1` receipt with block reference. RPC failures,
reverts, malformed responses, timeout, identity drift, or sender/key mismatch create no success
receipt. Do not retry a failed submission by changing the evidence files; first determine whether
the transaction was accepted by the target and then use a newly reviewed explicit nonce policy.

## 5. Key custody and signing policy

ClearLang uses two different private-key roles:

- The release/proof signing key passed to `clg build --sign` is an Ed25519 tooling-evidence key.
  It does not authorize an on-chain account.
- The `--signing-key` file for target deploy/invoke is a local
  `clg.evm-signing-key.v1` secp256k1 key. Its declared lowercase EVM address must equal the
  explicit `--sender` value.

Store both classes outside the repository and generated output directories. Use least-privilege
file permissions, an approved secret store or offline signing workstation, separate production
and non-production keys, and an access log for every use. Never place private-key JSON in a
campaign plan, trace, receipt, shell history, CI log, or support ticket. Wallet discovery, remote
signing, environment-variable keys, automatic nonce selection, and fee estimation are not
supported by this profile.

For suspected compromise: immediately stop target submissions, preserve the affected signed
bundle and receipts, identify the potentially exposed sender and release key IDs, rotate or
revoke the affected operational credentials outside ClearLang, and redeploy only under a newly
reviewed evidence set. A tooling-signature rotation does not rotate an EVM account, and an EVM
signer rotation does not repair existing release evidence.

## 6. Upgrade, rollback, and incident response

There is no proxy, delegate-call, automatic migration, transaction replacement, or automatic
rollback mechanism in the supported profile. Treat each deployed contract address as immutable.

For a compatible schema change:

1. Preserve the prior schema, artifact set, signed bundle, and target receipts.
2. Run the append-only schema check against the prior schema.
3. Prove, test, and simulate the new source; retain campaign replay evidence.
4. Prepare a new complete artifact/evidence set and obtain deployment approval.
5. Deploy a new address explicitly, record its receipt, and update the application/configuration
   layer only through its own reviewed change process.

For an incompatible schema change, require the explicit migration declaration and its exact
prior-schema digest. The deployment/activation method for a migration is not yet an automated
target operation; do not claim an in-place upgrade is supported.

For rollback, stop further invokes, preserve all evidence and target identifiers, and redirect
off-chain clients only to a previously approved immutable deployment after confirming its chain,
address, schema version, and retained receipt. Never overwrite a receipt or rewrite a signed
bundle to describe a different deployment.

For any incident (unexpected state, failed/reverted transaction, suspected key exposure, or
evidence mismatch):

1. Stop automated submissions for the affected sender and endpoint.
2. Save the command arguments, trace/campaign inputs, artifact set, signed bundle, and any RPC
   response or receipt without modification.
3. Classify the failure: local simulator, evidence binding, signing/sender, RPC/chain selection,
   or on-chain revert.
4. Reproduce locally only with preserved inputs; do not run a new simulation as evidence of the
   original target result.
5. Escalate key exposure and target-state decisions to the designated security and release
   owners. Resume only with a documented decision and, when artifacts changed, a regenerated
   complete evidence set.

## 7. Compatibility and known exclusions

The target compatibility claim is limited to the deterministic London-opcode harness and strict
JSON-RPC fixture covered by `clg.evm-stateful-scalar.v1`. It is not a claim of compatibility with
arbitrary EVM clients or networks, EIP-1559 transactions, proxy/delegate patterns, fallback
dispatch, dynamic ABI values, collections, `U256`, value transfer, external calls, callbacks, or
general EVM bytecode.

Cryptographic hardness, timing/side-channel resistance, and target-runtime implementation
correctness remain outside the ClearLang proof claim unless a later primitive- and target-specific
assurance boundary is implemented. Unsupported source or target features must fail closed; do
not work around a rejection by substituting raw calldata, manual artifact edits, or a different
target profile.

## References

- `docs/ide/vscode-cli-profile.md`
- `docs/release-process.md`
- `docs/design/phase-30.1.1.4.2.2-target-receipt-identity-binding-lock.md`
- `docs/design/phase-30.3.3.4.0-evm-transaction-signing-policy-lock.md`
- `docs/design/phase-30.3.3.3.0-stateful-scalar-evm-profile-lock.md`
- `docs/design/phase-30.3.4-evm-target-conformance-lock.md`
