# Milestone 4 - Production Contract Platform (30-35)

Milestone 4 makes ClearLang usable as a production contract product, starting with one complete
EVM-compatible target workflow. It preserves the Milestone 3 proof, release, provenance, and
delivery contracts.

Execution order: **contract semantics -> external-call safety -> target adapter and simulator ->
crypto claim closure -> developer and operational readiness -> end-to-end release gate**.

## Product Boundary

- First supported target: EVM-compatible chains.
- First supported workflow: create -> prove -> test -> simulate -> release -> deploy/invoke ->
  independently verify.
- Explicitly deferred: additional chain targets, web/mobile frameworks, and general application
  runtime work.

- [ ] 30.0 Milestone 4 product and architecture lock [Planning Gate A]
  - [x] 30.0.0 Publish the governing product boundary, supported target decision, proof claim,
    non-goals, and exit criterion. (`docs/design/phase-30.0.0-milestone-4-production-contract-platform-lock.md`) `Completed: 2026-08-05`.
  - [x] 30.0.1 Define ownership, acceptance evidence, and release-gate dependencies for every
    Milestone 4 parent gate. (`docs/design/phase-30.0.1-milestone-4-gate-governance-lock.md`) `Completed: 2026-08-05`.

- [ ] 30.1 Stateful contract semantics [Contract Gate A]
  - [x] 30.1.0 Lock the source-level contract state declaration, schema versioning, upgrade, and
    migration policy before implementation. (`docs/design/phase-30.1.0-state-schema-and-migration-lock.md`) `Completed: 2026-08-05`.
  - [x] 30.1.1 Implement typed persistent state declarations and deterministic storage layout
    artifacts. (`crates/{ast,parser,typer,cli}`, `crates/parser/tests/parse_smoke.rs`, `crates/typer/tests/contract_state.rs`) `Completed: 2026-08-05`.
  - [ ] 30.1.1.1 Bind declared `state.<field>` values into contract-function typing and add
    deterministic read/write lowering hooks. This is required before `old(...)` can represent a
    true pre-state rather than an alias for an immutable parameter.
    (`docs/design/phase-30.1.1.1-contract-state-binding-lock.md`)
  - [ ] 30.1.2 Implement `old(...)` snapshots in `ensure` clauses, including parser, typer, VC,
    solver encoding, diagnostics, and no-snapshot misuse rejection.
  - [ ] 30.1.3 Add state-transition and storage-invariant proof obligations with deterministic
    counterexamples and proof artifacts.
  - [ ] 30.1.4 Implement canonical event declarations/emission plus target-neutral ABI evidence.
  - [ ] 30.1.5 Add schema compatibility, storage migration, and state-invariant regression gates.

- [ ] 30.2 External-call and reentrancy safety [Contract Gate B]
  - [ ] 30.2.0 Lock the external-call capability, its relationship to local state mutation, and
    exact unsupported patterns/diagnostics.
  - [ ] 30.2.1 Implement explicit outbound-call syntax or a typed standard interface that cannot
    be confused with ordinary local I/O.
  - [ ] 30.2.2 Enforce checks-effects-interactions ordering or an equivalently strong
    state-transition/reentrancy protocol at type and proof boundaries.
  - [ ] 30.2.3 Add adversarial reentrancy fixtures and fail-closed release tests.
  - [ ] 30.2.4 Publish the precise security claim and exclusions; do not claim generic
    reentrancy prevention outside the enforced model.

- [ ] 30.3 EVM-compatible target and local simulation [Target Gate C]
  - [ ] 30.3.0 Lock supported EVM compatibility/version range, account/value/block-context
    model, ABI mapping, error/revert semantics, and gas/resource policy.
  - [ ] 30.3.1 Implement deterministic ABI generation for contract functions, events, errors,
    state schema, and target metadata.
  - [ ] 30.3.2 Implement a deterministic local simulator with caller, storage, value, block
    context, gas/resource limits, and versioned execution traces.
  - [ ] 30.3.3 Implement deploy/call/invoke adapter commands with explicit RPC/target
    configuration and no implicit network selection.
  - [ ] 30.3.4 Add end-to-end target conformance and compatibility fixtures against the supported
    EVM-compatible environment.

- [ ] 30.4 Crypto assurance for the first contract profile [Proof Gate D]
  - [ ] 30.4.0 Select and lock the minimal crypto surface required by the first EVM contract
    profile, including exact claim boundaries and threat-model exclusions.
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
