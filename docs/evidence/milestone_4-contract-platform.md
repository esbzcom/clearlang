# Milestone 4 Contract Platform Evidence

## 30.2.2 Checks-effects-interactions enforcement

- Date: 2026-08-08
- Accountable owner: language-security-owner
- Review boundary: language-and-proof-owner
- Target profile: `clg.contract-state-solver.v1`
- Acceptance commands:
  - `cargo test -p clg-typer --test contract_state`
  - `cargo test -p clg-cli --test diagnostics_codes`
- Stable result: accepted transitions perform their state writes and event emissions before one
  explicit outbound call; `T832` rejects state reads/writes, event emission, or a second outbound
  call after that boundary.
- Negative evidence: the contract-state test fixture covers each rejected post-call interaction.
- Known exclusions: target execution, callbacks, proxy/delegate calls, and call-boundary invariant
  proof remain unsupported and release-blocking until the target adapter and adversarial fixtures
  are complete.
- Release policy: strict release remains fail-closed for every outbound call because no executable
  target adapter or call-boundary solver model exists.

## 30.3.1 Deterministic target ABI descriptor

- Date: 2026-08-09
- Accountable owner: target-runtime-owner
- Review boundary: language-and-proof-owner
- Target descriptor profile: `clg.evm-compatible.abi.v1`
- Acceptance commands:
  - `cargo test -p clg-cli contract_abi --lib`
  - `cargo test -p clg-cli --test cli_it basic::build_emits_deterministic_target_contract_abi`
- Stable result: `clg build --emit-contract-abi <FILE>` emits byte-stable canonical JSON for one
  contract's functions, events, explicit unsupported error surface, state-schema links, compiler,
  and complete loaded-source graph.
- Negative/provenance evidence: changing an imported loaded source changes the emitted ABI
  source-graph digest.
- Known exclusions: EVM selectors/topics, ABI calldata and tuple encoding, revert decoding,
  deployment, and network compatibility are deferred. The descriptor is not an executable wire
  ABI or a deployment receipt.
- Release policy: release remains fail-closed until the target adapter binds this descriptor to an
  executable receipt and independently verifies cross-artifact identity.

## 30.3.2 Deterministic local simulator

- Date: 2026-08-09
- Accountable owner: target-runtime-owner
- Review boundary: language-and-proof-owner
- Execution profile: `clg.contract-simulation-trace.v1`
- Acceptance command: `cargo test -p clg-cli --test cli_it basic::simulate`
- Stable result: `clg simulate <SOURCE> --function <NAME> --state <FILE> --args <FILE>
  --state-out <FILE> --trace-out <FILE>` deterministically executes the locked straight-line
  scalar state profile. It records explicit caller, value, block number, timestamp, fuel, ordered
  state changes, ordered events, result/failure, and pre/post state in canonical JSON.
- Negative evidence: fuel exhaustion and `ExternalCall` both fail closed. They write a canonical
  failure trace, preserve the input as the only committed state, and never create the requested
  state output, including when the rejected external call follows otherwise valid effects.
- Known exclusions: control flow, collections, memory-backed values, user-function dispatch,
  external execution, target imports, ABI wire encoding, RPC, and deployment remain unsupported.
- Release policy: simulation is local evidence only; it is neither a target-execution attestation
  nor a deployment/release claim.
