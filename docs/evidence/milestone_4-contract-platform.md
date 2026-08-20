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

## 30.1.1.3.2 Constructor simulator lifecycle

- Date: 2026-08-09
- Accountable owner: language-and-proof-owner
- Review boundary: target-runtime-owner
- Acceptance commands:
  - `cargo test -p clg-typer --test contract_state`
  - `cargo test -p clg-cli --test cli_it basic::simulate`
- Stable result: checked `init(...)` declarations lower to a private constructor IR entrypoint;
  `clg simulate --init` accepts only `{}`, validates the resulting complete scalar state, and
  writes a canonical initialization trace without a public return value.
- Negative evidence: nested initialization remains `T829`; reusing initialized state with
  `--init` fails before execution and without state output.
- Known exclusions: durable deployment identity, branch/loop constructor execution, target
  receipts, and constructor calldata are deferred. This is local lifecycle evidence only.

## 30.2.3 Adversarial reentrancy and release-fail-closed evidence

- Date: 2026-08-09
- Accountable owner: language-security-owner
- Review boundary: release-assurance-owner
- Acceptance commands:
  - `cargo test -p clg-typer --test contract_state contract_external_calls_enforce_checks_effects_interactions`
  - `cargo test -p clg-cli --test cli_it basic::build_rejects_adversarial_contract_external_call_without_an_adapter`
- Stable result: CEI rejects every state read/write, event emission, and additional outbound call
  after the first external call with `T832`.
- Negative/release evidence: an adversarial contract containing a typed external call fails the
  executable build before it writes a Wasm artifact because the external-call adapter is absent.
- Known exclusions: this confirms fail-closed behavior and source ordering only; it is not a
  target callback or live-network reentrancy conformance result.

## 30.2.4 Reentrancy security claim

- Date: 2026-08-09
- Accountable owner: language-security-owner
- Review boundary: target-runtime-owner
- Stable claim: only accepted source-level `mut` transitions receive the CEI ordering guarantee;
  no executable external-call path is currently supported.
- Exclusions and release rule: callbacks, proxies, delegate calls, raw calldata, dynamic targets,
  target-runtime behavior, and call-boundary invariant proof are excluded. Any outbound-call IR
  or unresolved target/solver boundary remains strict-release blocking.
- Governing lock: `docs/design/phase-30.2.4-reentrancy-security-claim-lock.md`.

## 30.3.4 EVM target conformance fixtures

- Date: 2026-08-20
- Accountable owner: target-runtime-owner
- Review boundary: language-and-proof-owner
- Target profile: `clg.evm-stateful-scalar.v1` over the explicit `clg.evm-compatible.v1` RPC
  adapter.
- Acceptance commands:
  - `cargo test -p clg-cli --lib commands::build::evm_artifact::tests::stateful_scalar_profile_conforms_for_every_supported_storage_word_type`
  - `cargo test -p clg-cli --lib commands::target::tests`
  - `cargo test -p clg-cli --test cli_it target_`
- Stable result: the opcode fixture executes constructor and transition bytecode for Bool, U8,
  U64, U128, and signed Int values, then confirms field-ID-derived slots and ABI return words.
  Target adapter fixtures validate explicit chain selection, EIP-155 raw submission, confirmed
  receipt handling, constructor data, and evidence-bound receipts.
- Negative evidence: unsupported IR remains rejected; receipt revert, malformed target data, and
  compiler/source/proof/bundle drift write no success receipt.
- Compatibility boundary: this validates the locked opcode and JSON-RPC surface only; it is not a
  claim of arbitrary EVM client, transaction-envelope, proxy, or external-call compatibility.
