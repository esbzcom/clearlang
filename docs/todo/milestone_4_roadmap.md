# Milestone 4 - Production Contract Platform (30-35)

Milestone 4 makes ClearLang usable as a production contract product, starting with one complete
EVM-compatible target workflow. It preserves the Milestone 3 proof, release, provenance, and
delivery contracts.

Execution order: **contract source semantics -> target-profile lock and minimal state adapter ->
state proof/migration closure -> external-call safety -> local simulator -> wire ABI and bounded
EVM artifacts -> stateful EVM backend -> signed deploy/invoke receipts -> cross-artifact identity
binding -> target conformance -> developer and operational readiness -> end-to-end release gate**.

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
7. [x] 30.2.3 adversarial reentrancy fixtures and fail-closed release tests, then 30.2.4 the
    precise security claim and exclusions. This Contract Gate B work ran in parallel with the
    early target work and does not block Contract Gate A.
8. [x] 30.3.3.1 EVM wire ABI with selectors/topics and deterministic static-scalar calldata.
9. [x] 30.3.3.2 bounded static-pure EVM artifact plus explicit read-only target call and receipt.
10. [x] 30.3.3.3.0 stateful scalar EVM profile lock.
11. [x] 30.3.3.3.1 stateful scalar EVM backend.
12. [x] 30.3.3.4.0 EVM transaction signing policy lock.
13. [x] 30.3.3.4.1 explicit EVM transaction signing, deploy/invoke submission, and receipts.
14. [x] Mark 30.3.3 complete.
15. [x] 30.1.1.4.2.2 [Contract Gate A] receipt identity binding and cross-artifact drift
    rejection, after deploy/invoke receipts exist.
16. [x] Mark 30.1 complete.
17. [x] 30.3.4 target conformance and compatibility fixtures, then mark 30.3 complete.
18. [ ] Execute 30.6.4-30.6.7 exit-criterion hardening in order: release-evidence binding,
   end-to-end fixture coverage, required CI enforcement, then final closure review.

The promoted adapter is deliberately narrow: it consumes only `StateRead`, `StateWrite`, and
`EventEmit`; it does not enable network deployment, external calls, or a general EVM target.

- [x] 30.0 Milestone 4 product and architecture lock [Planning Gate A] `Completed: 2026-08-05`.
  - [x] 30.0.0 Publish the governing product boundary, supported target decision, proof claim,
    non-goals, and exit criterion. (`docs/design/phase-30.0.0-milestone-4-production-contract-platform-lock.md`) `Completed: 2026-08-05`.
  - [x] 30.0.1 Define ownership, acceptance evidence, and release-gate dependencies for every
    Milestone 4 parent gate. (`docs/design/phase-30.0.1-milestone-4-gate-governance-lock.md`) `Completed: 2026-08-05`.

- [x] 30.1 Stateful contract semantics [Contract Gate A] `Completed: 2026-08-20`.
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
    - [x] 30.1.1.3 Implement the locked `init(...)` lifecycle: exactly-once constructor identity,
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
    - [x] 30.1.1.4 Complete the locked contract-state artifact identity: normalized invariant
      identifiers/expressions, constructor identity, target-profile identifier, and compiler and
      source digests must be bound into the schema and release evidence.
      - [x] 30.1.1.4.1 Bind normalized invariant and constructor identities plus the minimal
        target-profile identifier into the deterministic state-schema artifact.
        (`crates/cli/src/commands/build/contract_state_schema.rs`,
        `crates/cli/tests/cli_it/basic/contract_state_schema.rs`) `Completed: 2026-08-08`.
      - [x] 30.1.1.4.2 Bind compiler and loaded-source digests into the schema, proof, target,
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

- [x] 30.2 External-call and reentrancy safety [Contract Gate B] `Completed: 2026-08-20`.
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

- [x] 30.3 EVM-compatible target and local simulation [Target Gate C] `Completed: 2026-08-20`.
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
- [x] 30.3.3 Implement deploy/call/invoke adapter commands and target receipts with explicit
    RPC/target configuration and no implicit network selection.
    (`docs/design/phase-30.3.3-explicit-target-adapter-receipt-lock.md`)
    - [x] 30.3.3.1 Emit canonical `clg.evm-wire-abi.v1` artifacts with Keccak selectors, event
      topics, and deterministic ABI-v2 calldata encoding for the initial static-scalar surface.
      (`crates/cli/src/commands/build/evm_wire_abi.rs`, `crates/cli/src/commands/target.rs`,
      `crates/cli/tests/cli_it/basic/contract_state_schema.rs`) `Completed: 2026-08-20`.
    - [x] 30.3.3.2 Emit bounded deployable `clg.evm-artifact.v1` bytecode for the
      fail-closed `clg.evm-static-pure.v1` profile, then support explicit read-only `target call`
      with `eth_chainId` verification, `eth_call`, and canonical success receipts. Constructor,
      mutation, and signing remain unsupported in this bootstrap profile.
      (`crates/cli/src/commands/{build/evm_artifact.rs,target.rs}`,
      `crates/cli/tests/cli_it/basic/{contract_state_schema.rs,contract_target.rs}`)
      `Completed: 2026-08-20`.
    - [x] 30.3.3.3.0 Lock the `clg.evm-stateful-scalar.v1` execution profile: supported scalar
      types, field-ID-derived storage slots, constructor calldata, event encoding, EVM revision,
      revert behavior, and the exact correspondence to the VC state model.
      (`docs/design/phase-30.3.3.3.0-stateful-scalar-evm-profile-lock.md`)
      `Completed: 2026-08-20`.
    - [x] 30.3.3.3.1 Implement the fail-closed `clg.evm-stateful-scalar.v1` backend for the first
      stateful contract profile. It must emit constructor and transition bytecode, map only
      supported scalar `StateRead`/`StateWrite` operations to deterministic `SLOAD`/`SSTORE`
      slots, ABI-decode/encode the supported scalar surface, and lower declared events to
      canonical `LOGn` operations. The emitted artifact must bind its storage-slot derivation and
      transition mapping to the VC state model; every IR operation without a verified EVM mapping
      must reject. Initial exclusions: collections, dynamic bytes/strings, mutable loops, and
      external calls. (`crates/cli/src/commands/build/evm_artifact.rs`,
      `crates/cli/tests/cli_it/basic/contract_state_schema.rs`) `Completed: 2026-08-20`.
    - [x] 30.3.3.4.0 Lock the EVM transaction-signing policy: key-file schema, EIP-155 envelope,
      sender derivation, explicit nonce/gas/fee inputs, receipt confirmation, and failure rules.
      (`docs/design/phase-30.3.3.4.0-evm-transaction-signing-policy-lock.md`)
      `Completed: 2026-08-20`.
    - [x] 30.3.3.4.1 Implement explicit signed `target deploy` and `target invoke`. Define one
      validated secp256k1 key-file format; require explicit sender, nonce, gas limit, and fee
      policy; derive and match the sender address; canonically encode/sign EIP-155 transactions;
      submit only with `eth_sendRawTransaction`; and emit a `clg.target-receipt.v1` only after a
      validated successful on-chain receipt. Chain mismatch, RPC failure, malformed receipt, and
      revert must fail closed without a success receipt. Deploy accepts explicit constructor
      arguments where declared by the wire ABI. (`crates/cli/src/commands/{target.rs,target/transaction.rs}`,
      `crates/cli/{src/main.rs,tests/cli_it/basic/contract_target.rs}`) `Completed: 2026-08-20`.
  - [x] 30.1.1.4.2.2 [Contract Gate A ownership] Bind compiler and loaded-source identity into the
    executable target receipt and reject schema/proof/target/signed-bundle drift. Requires the
    30.3.3 deploy/call/invoke receipt. (`docs/design/phase-30.1.1.4.2.2-target-receipt-identity-binding-lock.md`,
    `crates/cli/{src/commands/target.rs,src/{proofs/artifact_hashing.rs,signing.rs},src/main.rs}`,
    `crates/cli/tests/cli_it/basic/contract_target.rs`) `Completed: 2026-08-20`.
  - [x] 30.3.4 Add end-to-end target conformance and compatibility fixtures against the supported
    EVM-compatible environment. The locked scalar fixture executes all supported storage types,
    constructor and transition ABI words, storage slots, and receipt paths; incompatible surface
    remains fail-closed. (`docs/design/phase-30.3.4-evm-target-conformance-lock.md`,
    `docs/evidence/milestone_4-contract-platform.md`,
    `crates/cli/src/commands/{build/evm_artifact.rs,target.rs}`) `Completed: 2026-08-20`.

- [x] 30.4 Crypto assurance for the first contract profile [Proof Gate D] `Completed: 2026-08-20`.
  - [x] 30.4.0 Select and lock the minimal crypto surface required by the first EVM contract
    profile, including exact claim boundaries and threat-model exclusions. No source-level crypto
    primitive is selected; tooling crypto remains outside contract semantics.
    (`docs/design/phase-30.4.0-contract-crypto-surface-lock.md`) `Completed: 2026-08-09`.
  - [x] 30.4.1 Implement theorem-grade semantic models or an independently verified attestation
    boundary for each selected crypto primitive; unlabelled assumptions remain release-blocking.
    The selected contract surface is empty, and the boundary preserves strict fail-closed handling
    for any future crypto-dependent path. (`docs/design/phase-30.4.1-contract-crypto-assurance-boundary.md`)
    `Completed: 2026-08-09`.
  - [x] 30.4.2 Add authorization/signature proof fixtures and negative tests for malformed keys,
    signatures, domains, and replay inputs. No fixture is valid before a contract crypto API is
    selected; the fixture policy prevents tooling signatures from being misrepresented as support.
    (`docs/design/phase-30.4.2-contract-crypto-fixture-policy.md`) `Completed: 2026-08-09`.
  - [x] 30.4.3 Keep cryptographic hardness and timing/side-channel properties explicitly outside
    the claim unless separately verified.
    (`docs/design/phase-30.4.3-contract-crypto-nonclaims.md`) `Completed: 2026-08-09`.

- [x] 30.5 Contract developer experience and runtime operations [Readiness Gate E]
  `Completed: 2026-08-20`.
  - [x] 30.5.0 Define a bounded-memory lifecycle policy for closures, simulator instances, and
    contract host execution; add limits and observable failure modes. The supported scalar
    simulator records fuel and enforces explicit JSON byte limits; unsupported dynamic lifecycles
    remain fail-closed. (`docs/design/phase-30.5.0-contract-runtime-lifecycle-limits.md`,
    `crates/cli/tests/cli_it/basic/contract_simulate.rs`) `Completed: 2026-08-09`.
  - [x] 30.5.1 Ship target-aware contract test commands with property/fuzz campaigns,
    deterministic seed replay, trace capture, and test-artifact schemas. The simulator-only
    command/replay policy is locked; `clg contract test` provides the bounded first generator
    surface and canonical campaign evidence. (`docs/design/phase-30.5.1-contract-test-simulator-policy.md`,
    `crates/cli/{src/commands/contract_test.rs,tests/cli_it/basic/contract_test_campaign.rs}`)
    `Completed: 2026-08-20`.
  - [x] 30.5.2 Add source locations/stack traces for target simulation failures and a supported
    VS Code/LSP integration based on the stable CLI protocol. Simulation execution failures now
    emit trace-backed source/stack evidence and stable `C142` JSON diagnostics consumable by the
    published VS Code CLI profile. (`docs/design/phase-30.5.2-simulation-diagnostics-vscode-lock.md`,
    `crates/cli/{src/commands/simulate.rs,tests/cli_it/basic/contract_simulate.rs}`)
    `Completed: 2026-08-20`.
  - [x] 30.5.3 Publish developer and operator guidance for local simulation, deployment,
    upgrades, rollback, key custody, incident response, and target compatibility. The guide
    preserves the explicit target/evidence boundary and names unsupported upgrade, signing, and
    compatibility paths rather than implying ambient automation.
    (`docs/contract-operations.md`) `Completed: 2026-08-20`.

- [ ] 30.6 Production contract release closure [Release Gate F]
  `Baseline 30.6.0-30.6.3 completed: 2026-08-20; exit-criterion hardening remains.`
  - [x] 30.6.0 Add `clg contract release` orchestration that proves, tests, simulates, packages
    ABI/state-schema/target evidence, signs, and verifies the contract release bundle. The
    command emits `clg.contract-release-bundle.v1`, binds the signed proof package to canonical
    stateful EVM artifact bytes, verifies the generated signature/assurance evidence, and retains
    a deterministic simulator campaign report. (`docs/design/phase-30.6.0-contract-release-orchestration-lock.md`,
    `crates/cli/{src/commands/contract_release.rs,tests/cli_it/basic/contract_test_campaign.rs}`)
    `Completed: 2026-08-20`.
  - [x] 30.6.1 Add independent bundle verification for source/artifact, ABI, target profile,
    state-schema version, proof/attestation evidence, and signatures. `clg contract verify-release`
    resolves only bundle-local artifacts, verifies canonical hashes and target/schema/wire-ABI/proof
    binding, then verifies the target-artifact signature and assurance manifest.
    (`docs/design/phase-30.6.1-contract-release-verification-lock.md`, `crates/cli/src/commands/contract_release.rs`)
    `Completed: 2026-08-20`.
  - [x] 30.6.2 Add deterministic CI gates covering reference-contract creation through
    deploy/invoke/verification, plus tamper, replay, upgrade, and reentrancy-negative paths.
    The offline contract-release CI step runs the locked artifact, campaign replay, schema,
    target receipt, drift/tamper, and CEI-negative fixtures. (`.github/workflows/ci.yml`,
    `docs/evidence/phase-30.6-contract-release-ci.md`) `Completed: 2026-08-20`.
  - [x] 30.6.3 Publish the final supported product contract, compatibility policy, release
    checklist, and known exclusions. (`docs/contract-product-contract.md`)
    `Completed: 2026-08-20`.
  - [ ] 30.6.4 Bind every release-bundle claim to independently verifiable, bundle-local
    evidence. This closes the gap between a syntactically valid proof/signature and evidence
    that belongs to the packaged contract.
    - [x] 30.6.4.1 Extend `clg contract verify-release` to verify the signed payload's module
      hash, proof-artifact hash, compiler identity, source-graph identity, and signing key ID
      against the packaged EVM artifact, proof, signature, schema, wire ABI, and assurance
      evidence. Reject each mismatch before reporting success.
      (`crates/cli/src/commands/contract_release.rs`) `Completed: 2026-08-30`.
    - [ ] 30.6.4.2 Make simulator campaign evidence self-contained: package a canonical,
      path-safe trace/input index with digests and replay metadata, verify every indexed entry,
      and reject omitted, substituted, or traversal-path campaign evidence.
  - [ ] 30.6.5 Add deterministic release-workflow fixtures that exercise the supported product
    boundary from source through independent release verification.
    - [ ] 30.6.5.1 Create a reference stateful-contract fixture that proves, tests, simulates,
      releases, deploys/invokes on the locked target profile, and independently runs
      `clg contract verify-release` using only the emitted bundle.
    - [ ] 30.6.5.2 Run the fixture with controlled release inputs twice and assert the canonical
      artifact, bundle manifest, campaign index, and verification result are reproducible; make
      any intentionally variable release field explicit and excluded from the canonical
      comparison by specification.
    - [ ] 30.6.5.3 Add a tamper and replay matrix that independently mutates source/artifact,
      schema, wire ABI, target profile, proof, signature payload, assurance evidence, campaign
      trace/input, and bundle paths. Each case must fail closed with a stable diagnostic.
    - [ ] 30.6.5.4 Include supported upgrade/migration and reentrancy-negative scenarios in the
      end-to-end fixture suite, proving the stated state compatibility and CEI boundaries remain
      enforced after packaging.
  - [ ] 30.6.6 Make the complete 30.6.5 fixture suite a required offline CI gate. CI must run
    the actual `contract release` then `verify-release` happy path, deterministic replay checks,
    and the tamper/replay/reentrancy/upgrade-negative matrix; publish the command list and
    retained evidence locations in the CI evidence record.
  - [ ] 30.6.7 Perform final release-closure review. Mark Release Gate F and the Milestone 4
    exit criteria complete only after the required CI gate passes and the product contract,
    compatibility policy, operations guide, and release checklist accurately describe the
    verified boundary and exclusions.
