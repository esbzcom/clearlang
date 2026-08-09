# Milestone 4 - Production Contract Platform (30-35)

Milestone 4 makes ClearLang usable as a production contract product, starting with one complete
EVM-compatible target workflow. It preserves the Milestone 3 proof, release, provenance, and
delivery contracts.

Execution order: **contract source semantics -> target-profile lock and minimal state adapter ->
state proof/migration closure -> external-call safety -> full target adapter and simulator ->
crypto claim closure -> developer and operational readiness -> end-to-end release gate**.

## Product Boundary

- First supported target: EVM-compatible chains.
- First supported workflow: create -> prove -> test -> simulate -> release -> deploy/invoke ->
  independently verify.
- Explicitly deferred: additional chain targets, web/mobile frameworks, and general application
  runtime work.

## Dependency-Ordered Execution Plan

The numbered gate sections below remain the ownership index. Execute unfinished work only in this
order; a task's gate label identifies accountability, not its position in the implementation
queue.

1. [x] 30.0 product and architecture lock.
2. [x] 30.1 source semantics, state proof closure, and migration regression coverage through
   30.1.3.2 and 30.1.5.3.
3. [x] 30.2.0-30.2.2 external-call capability and CEI enforcement.
4. [x] 30.3.1 deterministic target ABI generation.
5. [x] 30.3.2 deterministic local simulator.
6. [x] 30.1.1.3.2 [Contract Gate A] exactly-once `init` lifecycle and all-path initialization
   proof, after the simulator exists.
7. [ ] 30.3.3 explicit deploy/call/invoke adapter and target receipts.
8. [ ] 30.1.1.4.2.2 [Contract Gate A] receipt identity binding and cross-artifact drift
    rejection, after target receipts exist.
9. [ ] Mark 30.1 complete.
10. [x] 30.2.3 adversarial reentrancy fixtures and fail-closed release tests, then 30.2.4 the
    precise security claim and exclusions. This Contract Gate B work is parallel to steps 4-8 and
    does not block Contract Gate A.
11. [ ] 30.3.4 target conformance and compatibility fixtures, then mark 30.3 complete.
12. [ ] Continue 30.4-30.6 in their listed dependency order.

The promoted adapter is deliberately narrow: it consumes only `StateRead`, `StateWrite`, and
`EventEmit`; it does not enable network deployment, external calls, or a general EVM target.

- [x] 30.0 Milestone 4 product and architecture lock [Planning Gate A] `Completed: 2026-08-05`.
  - [x] 30.0.0 Publish the governing product boundary, supported target decision, proof claim,
    non-goals, and exit criterion. (`docs/design/phase-30.0.0-milestone-4-production-contract-platform-lock.md`) `Completed: 2026-08-05`.
  - [x] 30.0.1 Define ownership, acceptance evidence, and release-gate dependencies for every
    Milestone 4 parent gate. (`docs/design/phase-30.0.1-milestone-4-gate-governance-lock.md`) `Completed: 2026-08-05`.

- [ ] 30.1 Stateful contract semantics [Contract Gate A]
  - [x] 30.1.0 Lock the source-level contract state declaration, schema versioning, upgrade, and
    migration policy before implementation. (`docs/design/phase-30.1.0-state-schema-and-migration-lock.md`) `Completed: 2026-08-05`.
  - [x] 30.1.1 Implement typed persistent state declarations and deterministic storage layout
    artifacts. (`crates/{ast,parser,typer,cli}`, `crates/parser/tests/parse_smoke.rs`, `crates/typer/tests/contract_state.rs`) `Completed: 2026-08-05`.
  - [x] 30.1.1.1 Bind declared `state.<field>` values into contract-function typing and add
    deterministic read/write lowering hooks. This is required before `old(...)` can represent a
    true pre-state rather than an alias for an immutable parameter.
    (`docs/design/phase-30.1.1.1-contract-state-binding-lock.md`, `crates/{ast,parser,typer}`) `Completed: 2026-08-05`.
  - [x] 30.1.1.2 Add target-neutral pre-state/post-state transition symbols to IR and VC
    artifacts. This must distinguish entry and exit storage before solver encoding of
    `old(...)`; target backends remain fail-closed until an adapter consumes these operations.
    (`docs/design/phase-30.1.1.2-state-transition-vc-ir-lock.md`, `crates/{ir,typer,codegen-wasm}`) `Completed: 2026-08-05`.
    - [x] 30.1.1.2.1 Emit deterministic pre/post state symbols, ordered writes, and scalar
      frame conditions in generated VCs, with an explicit non-release-grade assumption boundary.
      (`crates/typer/src/vc/generate.rs`, `crates/typer/tests/contract_state.rs`) `Completed: 2026-08-05`.
    - [x] 30.1.1.2.2 Lower state reads/writes to target-neutral IR operations and make every
      backend without a matching state adapter reject them deterministically. (`crates/{ir,typer,codegen-wasm}`, `crates/codegen-wasm/tests/contract_state_ops.rs`) `Completed: 2026-08-05`.
    - [ ] 30.1.1.3 Implement the locked `init(...)` lifecycle: exactly-once constructor identity,
      complete state-field initialization on every successful path, no outbound call, and
      constructor invariant proof evidence.
      - [x] 30.1.1.3.1 Parse and type-check `init(...)` declarations with constructor-only state
        write authority, full direct field initialization, stable `T829` diagnostics, and
        `init-invariant:*` VCs. (`crates/{ast,parser,typer}`, `crates/typer/tests/contract_state.rs`)
        `Completed: 2026-08-08`.
      - [x] 30.1.1.3.2 Execute `init` once through the local target adapter: a private lowered
        constructor entrypoint accepts only empty input state and validates complete initialized
        output before commit. Control-flow initialization remains fail-closed under `T829` until a
        later adapter proves it. (`docs/design/phase-30.1.1.3.2-init-simulator-lifecycle-lock.md`,
        `crates/{typer,cli}`, `crates/{typer,cli}/tests`) `Completed: 2026-08-09`.
    - [ ] 30.1.1.4 Complete the locked contract-state artifact identity: normalized invariant
      identifiers/expressions, constructor identity, target-profile identifier, and compiler and
      source digests must be bound into the schema and release evidence.
      - [x] 30.1.1.4.1 Bind normalized invariant and constructor identities plus the minimal
        target-profile identifier into the deterministic state-schema artifact.
        (`crates/cli/src/commands/build/contract_state_schema.rs`,
        `crates/cli/tests/cli_it/basic/contract_state_schema.rs`) `Completed: 2026-08-08`.
      - [ ] 30.1.1.4.2 Bind compiler and loaded-source digests into the schema, proof, target,
        and signed release artifacts, then add cross-artifact drift rejection.
        - [x] 30.1.1.4.2.1 Bind compiler identity and the complete loaded source-graph digest
          into schema, proof, and signed-bundle artifacts.
          (`crates/cli/src/commands/build/{run/core.rs,run/helpers.rs,contract_state_schema.rs}`,
          `crates/{cli/src/proofs/artifact_hashing.rs,cli/src/signing.rs}`,
          `crates/cli/tests/cli_it/basic/contract_state_schema.rs`) `Completed: 2026-08-08`.
        - Target-dependent execution item 30.1.1.4.2.2 is scheduled after 30.3.3 below.
  - [x] 30.1.2 Implement `old(...)` snapshots in `ensure` clauses, including parser, typer, VC,
    solver encoding, diagnostics, and no-snapshot misuse rejection. Entry-state symbols now
    encode `old(state.<field>)`, while ordinary state reads in bodies and postconditions use
    exit-state symbols; the explicit target-adapter assumption remains release-blocking.
    (`crates/{parser,typer}`, `crates/typer/tests/contract_state.rs`) `Completed: 2026-08-05`.
  - [x] 30.1.3 Add state-transition and storage-invariant proof obligations with deterministic
    counterexamples and proof artifacts. `Completed: 2026-08-08`.
    - [x] 30.1.3.1 Parse and type-check contract `invariant` declarations, then emit stable
      entry-to-exit storage-invariant VCs for every mut transition. The existing proof artifact
      pipeline records these target-neutral VCs and their assumption boundary.
      (`crates/{ast,parser,typer}`, `crates/parser/tests/parse_smoke.rs`,
      `crates/typer/tests/contract_state.rs`) `Completed: 2026-08-05`.
    - [x] 30.1.3.2 Connect the minimal target state adapter (30.3.0.1) to the solver model so
      straight-line scalar state-transition and invariant VCs use an explicit entry/exit relation
      and retain deterministic concrete solver models for failures. Unsupported flow and external
      calls remain explicit release-blocking boundaries.
      (`docs/design/phase-30.3.0-target-profile-and-state-solver-lock.md`,
      `crates/{typer,cli}`, `crates/typer/tests/contract_state.rs`,
      `crates/cli/tests/solver_outcomes.rs`) `Completed: 2026-08-08`.
  - [x] 30.1.4 Implement canonical event declarations/emission plus target-neutral ABI evidence.
    - [x] 30.1.4.1 Add canonical contract event declarations and deterministic event ABI evidence
      to the contract-state artifact. Event and event-field identifiers are stable hashes over
      the contract/event identity and declaration order is preserved.
      (`crates/{ast,parser,typer,cli}`, `crates/parser/tests/parse_smoke.rs`,
      `crates/typer/tests/contract_state.rs`, `crates/cli/tests/cli_it/basic/contract_state_schema.rs`)
      `Completed: 2026-08-06`.
    - [x] 30.1.4.2 Add typed event emission, target-neutral event IR, and deterministic
      fail-closed backend behavior until a target adapter consumes emitted events.
      (`crates/{parser,typer,ir,codegen-wasm}`, `crates/typer/tests/contract_state.rs`,
      `crates/codegen-wasm/tests/contract_state_ops.rs`) `Completed: 2026-08-06`.
  - [x] 30.1.5 Add schema compatibility, storage migration, and state-invariant regression gates.
    `Completed: 2026-08-08`.
    - [x] 30.1.5.1 Add a deterministic append-only state-schema compatibility gate. It requires
      a version increase and rejects changed, removed, reordered, or identity-mismatched prior
      fields before code generation. (`clg build --check-contract-state-schema <FILE>`,
      `crates/cli/src/commands/build/contract_state_schema.rs`,
      `crates/cli/tests/cli_it/basic/contract_state_schema.rs`) `Completed: 2026-08-06`.
    - [x] 30.1.5.2 Add explicit migration declarations and migration-proof/evidence artifacts
      for incompatible state changes. Migrations require an exact prior schema digest, have
      exclusive state-write access, contribute deterministic schema evidence, and emit
      invariant-preservation VCs under the existing target-adapter assumption boundary.
      (`crates/{ast,parser,typer,cli}`, `crates/typer/tests/contract_state.rs`,
      `crates/cli/tests/cli_it/basic/contract_state_schema.rs`) `Completed: 2026-08-08`.
    - [x] 30.1.5.3 Add state-invariant regression fixtures once the minimal target state adapter
      (30.3.0.1) can discharge concrete state-model counterexamples. Fixtures cover valid and
      invalid scalar transitions, migration invariants, and explicit unsupported-model boundaries.
      (`crates/typer/tests/contract_state.rs`, `crates/cli/tests/solver_outcomes.rs`)
      `Completed: 2026-08-08`.

- [ ] 30.2 External-call and reentrancy safety [Contract Gate B]
  - [x] 30.2.0 Lock the external-call capability, its relationship to local state mutation, and
    exact unsupported patterns/diagnostics.
    (`docs/design/phase-30.2.0-external-call-reentrancy-lock.md`) `Completed: 2026-08-08`.
  - [x] 30.2.1 Implement explicit outbound-call syntax or a typed standard interface that cannot
    be confused with ordinary local I/O. `external_call Interface.method(args);` resolves only
    against a declared interface method, is available only in a contract mut transition, lowers
    to target-neutral IR, and fails closed without a backend adapter.
    (`crates/{parser,typer,ir,codegen-wasm}`, `crates/parser/tests/parse_smoke.rs`,
    `crates/typer/tests/contract_state.rs`, `crates/codegen-wasm/tests/contract_state_ops.rs`)
    `Completed: 2026-08-08`.
  - [x] 30.2.2 Enforce checks-effects-interactions ordering at the type boundary. A
    contract-owned `mut` transition may make at most one outbound call, and state reads/writes,
    event emission, or another outbound call after it fail deterministically with `T832`.
    (`crates/typer/src/check/function_checks.rs`, `crates/typer/tests/contract_state.rs`,
    `docs/evidence/milestone_4-contract-platform.md`) `Completed: 2026-08-08`.
  - [x] 30.2.3 Add adversarial reentrancy fixtures and fail-closed release tests. CEI negatives
    cover every prohibited interaction after an outbound call, and the build command rejects an
    external-call contract without producing an executable artifact. (`crates/typer/tests/contract_state.rs`,
    `crates/cli/tests/cli_it/basic/build_release.rs`,
    `docs/evidence/milestone_4-contract-platform.md`) `Completed: 2026-08-09`.
  - [x] 30.2.4 Publish the precise security claim and exclusions; do not claim generic
    reentrancy prevention outside the enforced model.
    (`docs/design/phase-30.2.4-reentrancy-security-claim-lock.md`) `Completed: 2026-08-09`.

- [ ] 30.3 EVM-compatible target and local simulation [Target Gate C]
  - [x] 30.3.0 Lock the minimal target profile needed by Contract Gate A: canonical state
    pre/post symbols, scalar SMT sorts, event receipts, and strict exclusions. Full EVM
    compatibility/version, account/value/block-context, ABI mapping, revert semantics, and gas
    policy remain deferred to 30.3.1-30.3.4.
    (`docs/design/phase-30.3.0-target-profile-and-state-solver-lock.md`) `Completed: 2026-08-08`.
  - [x] 30.3.0.1 Implement the narrow state/event storage adapter and solver bridge required to
    execute `StateRead`, `StateWrite`, and `EventEmit`, produce deterministic concrete state
    counterexamples, and preserve fail-closed behavior for `ExternalCall`. This must complete
    before 30.1.3.2 and 30.1.5.3; deployment, RPC, and general EVM execution remain deferred.
    (`crates/{typer,cli}`, `crates/typer/tests/contract_state.rs`,
    `crates/cli/tests/solver_outcomes.rs`) `Completed: 2026-08-08`.
  - [x] 30.3.1 Implement deterministic ABI generation for contract functions, events, errors,
    state schema, and target metadata. `clg build --emit-contract-abi <FILE>` emits the canonical
    descriptor with explicit deferred wire/error boundaries; EVM selector and calldata encoding
    remain later target/crypto work.
    (`docs/design/phase-30.3.1-deterministic-target-abi-lock.md`,
    `crates/cli/src/commands/build/contract_abi.rs`,
    `crates/cli/tests/cli_it/basic/contract_state_schema.rs`,
    `docs/evidence/milestone_4-contract-platform.md`) `Completed: 2026-08-09`.
  - [x] 30.3.2 Implement a deterministic local simulator with caller, storage, value, block
    context, gas/resource limits, and versioned execution traces. `clg simulate` executes the
    locked straight-line scalar state profile using explicit JSON state/arguments and emits
    canonical `clg.contract-simulation-trace.v1` evidence. Unsupported instructions and external
    calls fail closed without committing state. (`crates/cli/src/commands/simulate.rs`,
    `crates/cli/tests/cli_it/basic/contract_simulate.rs`,
    `docs/evidence/milestone_4-contract-platform.md`) `Completed: 2026-08-09`.
  - [x] 30.1.1.3.2 [Contract Gate A ownership] Execute `init` exactly once through the local
    target adapter and prove complete initialization for the accepted direct-write profile.
    Branch/loop initialization remains rejected by `T829` until a later target adapter can prove
    all paths. (`docs/design/phase-30.1.1.3.2-init-simulator-lifecycle-lock.md`,
    `crates/cli/tests/cli_it/basic/contract_simulate.rs`) `Completed: 2026-08-09`.
  - [ ] 30.3.3 Implement deploy/call/invoke adapter commands with explicit RPC/target
    configuration and no implicit network selection. The command/receipt contract is locked, but
    remains blocked on EVM bytecode plus selector/calldata encoding; the existing ABI descriptor
    is deliberately not a wire ABI. (`docs/design/phase-30.3.3-explicit-target-adapter-receipt-lock.md`)
  - [ ] 30.1.1.4.2.2 [Contract Gate A ownership] Bind compiler and loaded-source identity into the
    executable target receipt and reject schema/proof/target/signed-bundle drift. Requires the
    30.3.3 deploy/call/invoke receipt.
  - [ ] 30.3.4 Add end-to-end target conformance and compatibility fixtures against the supported
    EVM-compatible environment.

- [ ] 30.4 Crypto assurance for the first contract profile [Proof Gate D]
  - [x] 30.4.0 Select and lock the minimal crypto surface required by the first EVM contract
    profile, including exact claim boundaries and threat-model exclusions. No source-level crypto
    primitive is selected; tooling crypto remains outside contract semantics.
    (`docs/design/phase-30.4.0-contract-crypto-surface-lock.md`) `Completed: 2026-08-09`.
  - [ ] 30.4.1 Implement theorem-grade semantic models or an independently verified attestation
    boundary for each selected crypto primitive; unlabelled assumptions remain release-blocking.
  - [ ] 30.4.2 Add authorization/signature proof fixtures and negative tests for malformed keys,
    signatures, domains, and replay inputs.
  - [ ] 30.4.3 Keep cryptographic hardness and timing/side-channel properties explicitly outside
    the claim unless separately verified.

- [ ] 30.5 Contract developer experience and runtime operations [Readiness Gate E]
  - [ ] 30.5.0 Define a bounded-memory lifecycle policy for closures, simulator instances, and
    contract host execution; add limits and observable failure modes.
  - [ ] 30.5.1 Ship target-aware contract test commands with property/fuzz campaigns,
    deterministic seed replay, trace capture, and test-artifact schemas.
  - [ ] 30.5.2 Add source locations/stack traces for target simulation failures and a supported
    VS Code/LSP integration based on the stable CLI protocol.
  - [ ] 30.5.3 Publish developer and operator guidance for local simulation, deployment,
    upgrades, rollback, key custody, incident response, and target compatibility.

- [ ] 30.6 Production contract release closure [Release Gate F]
  - [ ] 30.6.0 Add `clg contract release` orchestration that proves, tests, simulates, packages
    ABI/state-schema/target evidence, signs, and verifies the contract release bundle.
  - [ ] 30.6.1 Add independent bundle verification for source/artifact, ABI, target profile,
    state-schema version, proof/attestation evidence, and signatures.
  - [ ] 30.6.2 Add deterministic CI gates covering reference-contract creation through
    deploy/invoke/verification, plus tamper, replay, upgrade, and reentrancy-negative paths.
  - [ ] 30.6.3 Publish the final supported product contract, compatibility policy, release
    checklist, and known exclusions.

## Milestone 4 Exit Criteria

1. A reference stateful contract passes the complete supported workflow on the locked target.
2. Contract state, ABI, target, proof/attestation, and release evidence are deterministic and
   independently verifiable.
3. Unsupported crypto, state, target, and external-call assumptions fail closed.
4. Runtime lifecycle limits and operational failures are bounded, observable, and documented.
5. CI proves the happy path and the defined negative/tamper/replay/reentrancy paths.
