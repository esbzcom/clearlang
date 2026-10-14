# Milestone 5 - Pilot-Ready Verified Contract Product (40-44)

## Goal

Make the Milestone 4 contract workflow useful to an external crypto user without widening its
security claim: an operator can build a narrowly supported contract, prove and release it,
independently verify its bundle with a distributed verifier, and run a documented EVM-compatible
testnet pilot using only explicit inputs and locally reproducible evidence.

The first pilot use case is a **proof-carrying, caller-authorized configuration registry**. It is
deliberately not a custody, token, exchange, lending, proxy, or arbitrary-contract product. It
stores a small scalar configuration under a fixed owner and records state transitions as canonical
evidence. That makes the safety and release claims meaningful in a real target environment while
remaining inside a bounded, auditable execution model.

Milestone 5 preserves the README principles:

- **Simple for users:** one documented local preflight, one constrained contract template, and
  explicit deployment inputs.
- **AI-friendly:** versioned schemas, deterministic diagnostics, canonical evidence, and no
  implicit wallet, network, nonce, or fee decisions.
- **Provably correct:** every new source or target feature has a precise proof model, fail-closed
  unsupported boundary, and positive/negative acceptance fixtures.
- **Crypto-focused:** prioritize release integrity, caller authorization, target correspondence,
  and independent verification over language breadth.

## Product Boundary

### Supported on completion

- The Milestone 4 `clg.evm-stateful-scalar.v1` workflow, extended only with the caller identity
  and equality/guard operations required by the registry pilot.
- A distributable, independently invocable release-bundle verifier that needs only a release
  bundle, the approved public key, and documented verifier inputs; it must not trust an ambient
  checkout, RPC endpoint, wallet, or hidden project state.
- One locked EVM-compatible testnet profile, selected by exact chain ID and an explicit endpoint
  supplied by the operator.
- A reference registry project, deterministic local test/simulation/release verification, and a
  reproducible testnet deployment/invocation evidence record.

### Explicit non-goals

- Mainnet or production-funds authorization; the pilot is testnet-only.
- Generic EVM compatibility, Solidity interoperability, proxy upgrades, delegate calls, external
  calls/callbacks, value transfer, dynamic values, collections, `U256`, mutable branches or loops.
- Wallet discovery, remote signing, automatic nonce/fee selection, transaction replacement, or
  implicit network selection.
- General cryptographic APIs, arbitrary signature verification in source code, a token standard,
  custody, DeFi, or a broad language-feature expansion.
- Hosted/remote CI as an acceptance requirement. Remote automation may be informational, but it
  cannot replace or invalidate the local acceptance gate in this milestone.

## Acceptance Rule

Each implementation task must add or update deterministic local tests before it is complete. A
Milestone 5 completion claim requires all of the following on a clean local checkout:

1. `cargo fmt --all -- --check` succeeds.
2. `cargo clippy --workspace --all-targets -- -D warnings` succeeds.
3. the workspace test suite succeeds.
4. `cargo run -p xtask -- release-precheck` succeeds.
5. the new `xtask pilot-precheck` succeeds, runs the complete contract-release fixture suite,
   verifier-distribution tests, caller-authorization matrix, and reference-pilot local replay,
   and writes a canonical machine-readable report.
6. the reference release bundle verifies from a separate local directory using only its packaged
   evidence and approved public key.
7. the optional testnet exercise has a retained evidence record whose artifact identities are
   verified locally. Network availability is not part of the deterministic completion gate.

There are no automatic retries. A failure is investigated and a deliberate rerun is a new local
operator action. The testnet evidence records observation, not theorem-grade proof of an external
chain or RPC implementation.

## Dependency-Ordered Execution Plan

The numbered gate headings establish ownership. Execute unfinished work only in the order below;
the gates are deliberately narrow so a pilot result cannot be mistaken for a general-purpose
contract platform.

1. [ ] 40.0 product, threat-model, and local-validation lock.
2. [ ] 40.1 local pilot-precheck and canonical evidence report.
3. [ ] 40.2 independently distributed verifier contract and fixture matrix.
4. [ ] 41.0 caller-authorization semantic and proof-model lock.
5. [ ] 41.1 minimal caller identity, equality, and guard implementation.
6. [ ] 41.2 simulator/EVM correspondence and authorization-negative matrix.
7. [ ] 42.0 testnet profile and pilot operations lock.
8. [ ] 42.1 reference configuration-registry project and local release evidence.
9. [ ] 42.2 explicit testnet exercise, receipt binding, and evidence replay.
10. [ ] 43.0 external-user distribution and independent-verification walkthrough.
11. [ ] 44.0 final local closure review and pilot-readiness statement.

## 40. Product and Local Assurance Gate [Planning Gate A]

- [ ] 40.0 Publish the Milestone 5 product, threat-model, pilot-user, and non-claim lock before
  implementation. It must distinguish: (a) proven ClearLang source/adapter claims, (b) verified
  release-evidence claims, and (c) observed testnet behavior. It must state that testnet evidence
  does not authorize mainnet or funds custody.
  - [ ] 40.0.1 Inventory the exact Milestone 4 commands, schemas, target profile, diagnostics,
    and exclusions inherited by the pilot; name their source documents and resolve any conflicting
    wording before adding scope.
  - [ ] 40.0.2 Define the pilot actors (release owner, deploy operator, independent verifier,
    and observer), their trusted inputs, attacker capabilities, assets at risk, and response to a
    mismatch. Explicitly exclude secret-key safety, RPC honesty, and chain consensus from the
    proof claim.
  - [ ] 40.0.3 Write measurable acceptance and non-claim statements for source proof, bundle
    verification, local target simulation, and observed testnet actions. Add a document-drift
    test or checklist fixture that ensures all pilot docs use the same terms.

- [ ] 40.1 Add `xtask pilot-precheck`, a local-only, fail-closed aggregate gate. It must execute
  the existing release precheck, all Milestone 4 contract-release fixtures, verifier-distribution
  fixtures, authorization fixtures, and the reference-pilot offline replay; emit a canonical
  versioned JSON report with command versions, input artifact digests, ordered sub-gate outcomes,
  and no absolute host paths. Add tests that reject omitted, reordered, substituted, or malformed
  report evidence.
  - [ ] 40.1.1 Lock the command-line interface, report schema/version, deterministic sub-gate
    order, exit codes, report destination, and redaction policy. The report must identify tool
    versions and digests but never include secret material, endpoint credentials, or host paths.
  - [ ] 40.1.2 Implement the dispatcher as a composition of existing local commands with explicit
    argument forwarding and fail-fast aggregation. It must distinguish a command execution
    failure from a malformed or unverifiable sub-report.
  - [ ] 40.1.3 Add fixture tests for a complete pass, each failing sub-gate, report canonicality,
    report tampering, missing evidence, unsafe artifact paths, and repeat-run deterministic
    output. Integrate the command into the local release/pilot runbook, not as a remote gate.

- [ ] 40.2 Publish the local-toolchain support and reproduction policy for the pilot: exact Rust,
  solver, target-profile, verifier, and report-schema compatibility rules; deterministic failure
  diagnostics; and an explicit policy for supported versus informational remote automation. The
  documentation must name the exact one-command local acceptance path.
  - [ ] 40.2.1 Record the supported local operating-system/architecture matrix, Rust toolchain,
    bundled solver identity, required external tools, and target/verifier/report schema versions.
  - [ ] 40.2.2 Specify upgrade, incompatibility, and rollback behavior: a changed toolchain or
    schema must either reproduce accepted evidence or fail with an actionable deterministic
    diagnostic; it may not silently reinterpret evidence.
  - [ ] 40.2.3 Create a clean-directory reproduction fixture that follows the published commands
    and validates the resulting report against its schema and expected digest projection.

## 41. Minimal Caller Authorization [Contract Gate B]

- [ ] 41.0 Lock a single, target-neutral caller-authorization surface before implementation. The
  source model may expose only an opaque `Address` identity, `context.caller`, equality, and
  `require`-style guards needed by the pilot. Specify typing/effects, VC symbols and assumptions,
  simulator JSON encoding, EVM calldata/context mapping, ABI/schema identity, failure/revert
  behavior, and exact unsupported operations. Arithmetic, ordering, conversion, hashing,
  signature verification, address construction, and arbitrary context fields remain unsupported.
  - [ ] 41.0.1 Define the canonical `Address` representation and literal/input policy, including
    byte length, normalization, display form, JSON encoding, equality semantics, and malformed
    input diagnostics. No implicit conversion from integers, strings, bytes, or keys is allowed.
  - [ ] 41.0.2 Define `context.caller` availability, effect restrictions, `require` failure
    behavior, and the precise VC pre-state/post-state symbols. The model must show why an
    unauthorized transition cannot satisfy the successful transition relation.
  - [ ] 41.0.3 Define simulator trace, state schema, proof artifact, wire ABI, EVM execution, and
    target-receipt identity bindings. Add design-level examples for accepted owner checks and for
    every rejected broader authorization pattern.

- [ ] 41.1 Implement the locked source, parser, typer, IR, VC, simulator, schema, wire-ABI, and
  EVM lowering path. The proof model must bind the caller identity used by a verified transition
  to the simulator trace and EVM target receipt. Any backend or proof configuration unable to
  establish that correspondence rejects the contract/release; no placeholder caller is accepted.
  - [ ] 41.1.1 Add AST/parser/type-checker support and stable diagnostics for `Address`,
    `context.caller`, equality, and the limited guard form. Reject all unlisted operations before
    IR lowering, with parser/type tests for every accepted and rejected expression shape.
  - [ ] 41.1.2 Add target-neutral IR and VC lowering with explicit caller symbols and bind them
    into canonical proof/schema artifacts. Add solver fixtures proving an owner-authorized update
    and rejecting a transition whose guard cannot be established.
  - [ ] 41.1.3 Extend simulator, ABI/wire ABI, EVM artifact generation, deploy/invoke commands,
    and receipts with the canonical caller identity. Add cross-layer golden fixtures and reject
    any target lacking an exact mapping.

- [ ] 41.2 Add exhaustive deterministic positive and negative coverage: owner initialization,
  authorized transition, unauthorized caller, malformed caller input, caller/source/schema/wire
  ABI/proof/target drift, replay under a different caller, and unavailable target mapping. Verify
  that failed authorization does not commit state, cannot produce a successful target receipt,
  and cannot be represented as a successful release bundle. Include property/campaign replay
  cases with the caller identity in canonical evidence.
  - [ ] 41.2.1 Create unit and integration matrices for authorized and unauthorized transitions,
    malformed/normalized addresses, omitted caller context, and state non-commit after every
    authorization failure.
  - [ ] 41.2.2 Create end-to-end release fixtures that mutate caller values independently in the
    source, proof, campaign input/trace, schema, ABI, target artifact, receipt, and bundle index;
    require stable fail-closed diagnostics for each mutation.
  - [ ] 41.2.3 Add deterministic campaign seed/replay tests that demonstrate identical caller
    inputs reproduce identical outcomes and a substituted caller either changes the signed
    evidence identity or fails verification.

## 42. Reference Testnet Pilot [Target Gate C]

- [ ] 42.0 Lock one EVM-compatible **testnet-only** profile. Select it only after local
  conformance tests demonstrate that its explicit RPC behavior matches the supported transaction
  envelope and London scalar profile. Record chain ID, permitted RPC methods, receipt/finality
  observation rule, endpoint trust boundary, faucet/key handling policy, known incompatibilities,
  and a deterministic offline fixture substitute. Do not add ambient network defaults or make
  network reachability a release gate.
  - [ ] 42.0.1 Evaluate candidate testnets against a written conformance questionnaire covering
    chain ID, London opcode availability, transaction envelope support, the required JSON-RPC
    methods, receipt fields, and deterministic fixture capture feasibility.
  - [ ] 42.0.2 Select exactly one profile and publish its immutable identifier, expected RPC
    semantics, compatible client/version observations, faucet/key policy, and endpoint trust
    boundary. A changed chain ID or incompatible method response must fail before submission.
  - [ ] 42.0.3 Capture sanitized request/response fixtures for all supported success and failure
    paths, then prove that the existing target adapter accepts/rejects them exactly as the locked
    profile requires without contacting a live endpoint.

- [ ] 42.1 Add a versioned reference configuration-registry project. It must initialize a fixed
  owner, permit that owner alone to update a bounded scalar configuration, emit a canonical event,
  preserve a stated invariant, and provide a migration-compatible v2 scenario. Its local fixture
  must prove, campaign-test, simulate, release, independently verify, deploy/invoke through the
  mocked locked target, and verify receipts against the exact release bundle.
  - [ ] 42.1.1 Specify v1 state, constructor inputs, owner guard, bounded configuration value,
    event, invariant, and all permitted transitions in the project README and source contracts.
  - [ ] 42.1.2 Implement deterministic source, manifest, test plan, campaign inputs, simulator
    expected traces, release inputs, and signed public test fixtures. Keep all private keys and
    live endpoints outside the repository and fixture format.
  - [ ] 42.1.3 Add v1 and v2 migration fixtures plus source-to-receipt integration tests. Assert
    proof success for valid owner updates, rejection/non-commit for others, release verification
    in an independent directory, and target-receipt identity binding.

- [ ] 42.2 Add an explicit operator command/runbook for the testnet exercise. It must require
  operator-provided endpoint, chain ID, signer key, sender, nonce, gas limit, fee, constructor
  inputs, and release bundle; capture deploy and authorized/unauthorized invocation observations;
  then create a canonical, path-safe pilot-evidence record bound to release-bundle and receipt
  identities. Provide an offline replay verifier using recorded RPC request/response fixtures.
  The live exercise is manually initiated and never silently contacted by a local test command.
  - [ ] 42.2.1 Define an explicit input file/schema for endpoint, chain ID, signer reference,
    sender, nonce, gas limit, fee, constructor arguments, contract invocation, and release-bundle
    location. Validate and display every value before a transaction is signed or submitted.
  - [ ] 42.2.2 Implement canonical pilot-evidence capture for submission intent, transaction
    digest, sanitized RPC observations, receipt, release-bundle digest, caller identity, and
    verifier result. Make it path-safe, secret-free, ordered, and signed only if the existing
    evidence-signing policy authorizes that use.
  - [ ] 42.2.3 Implement offline replay that serves only recorded fixture responses and verifies
    the deploy, authorized invoke, and unauthorized invoke observations. Add tests for omitted,
    substituted, reordered, and cross-chain records.

## 43. Independent User Delivery [Distribution Gate D]

- [ ] 43.0 Lock the external verifier distribution format and compatibility contract. Prefer the
  smallest stable form that can be independently run by a non-Rust user (a dedicated verifier
  executable plus a versioned OCI/container image if that adds no second verification semantics).
  Define artifact naming, platform scope, checksums, signatures, public-key/trust-policy input,
  offline operation, upgrade/deprecation policy, and reproducible invocation examples.
  - [ ] 43.0.1 Decide whether a dedicated subcommand/binary alone is sufficient or whether an OCI
    image is justified; the decision must retain one shared verification core and one canonical
    result schema.
  - [ ] 43.0.2 Define supported platforms, artifact names, semantic version compatibility,
    checksum/signature format, trusted public-key provisioning, offline network policy, and
    stable human/JSON output contracts.
  - [ ] 43.0.3 Define deprecation and emergency rollback rules, including how an unsupported
    verifier/schema/version rejects rather than guessing compatibility, and add a compatibility
    table to the distribution documentation.

- [ ] 43.1 Package the verifier according to the lock without duplicating the verification
  implementation. It must consume only bundle-local paths, reject unsafe paths and unsupported
  schema/profile versions, and preserve stable machine-readable diagnostics. Add black-box tests
  from a temporary independent directory for valid verification and every Milestone 4/5 tamper
  class, including a verifier-binary/version mismatch.
  - [ ] 43.1.1 Extract or stabilize a single verifier entrypoint and make the distributable form
    invoke it without copying parsing, hashing, or policy logic. Add tests that compare its
    canonical output to `clg contract verify-release` for the same inputs.
  - [ ] 43.1.2 Add deterministic local packaging that emits the verifier artifact, version
    metadata, checksum, signature/provenance evidence where supported, and a manifest binding
    those files. Verify the package before declaring packaging success.
  - [ ] 43.1.3 Add black-box clean-directory tests for valid bundles, every inherited and new
    tamper class, unsafe paths, wrong public key, missing evidence, unknown schemas/profiles,
    and incompatible verifier versions. Assert no network attempt occurs.

- [ ] 43.2 Publish a short external-user walkthrough: obtain the verifier, validate its
  checksum/signature, independently verify the reference bundle, inspect the product boundary,
  and reproduce the local pilot precheck. Include a disclosure template for a pilot operator to
  report incompatible target behavior or a verification mismatch with the retained evidence.
  - [ ] 43.2.1 Write a copy/paste offline verification path that begins with checksum/signature
    validation and ends with an explainable pass or stable failure code; show no command that
    requires a source checkout or private key.
  - [ ] 43.2.2 Write a separate operator path for the local pilot precheck and testnet evidence
    replay, marking every externally supplied value and every action that can contact a network.
  - [ ] 43.2.3 Add documentation tests that execute the non-network walkthrough in a temporary
    directory and validate that referenced artifact names, commands, and result schemas remain
    current.

## 44. Pilot Closure [Release Gate E]

- [ ] 44.0 Publish the final Milestone 5 product contract, testnet-pilot operations guide,
  compatibility policy, verifier input schema, incident/rollback guidance, and known exclusions.
  They must consistently say that the product is a locally validated, testnet-only pilot and must
  not claim generic EVM support, remote-CI certification, mainnet readiness, or custody safety.
  - [ ] 44.0.1 Consolidate the supported commands, source/target profile, local gate, testnet
    boundary, verifier trust inputs, operational responsibilities, and known exclusions into one
    product-contract document.
  - [ ] 44.0.2 Write incident and rollback procedures for failed verification, wrong-chain
    submission attempt, unexpected receipt, leaked test key, and a discovered artifact mismatch;
    distinguish evidence preservation from any chain-side remediation.
  - [ ] 44.0.3 Add document consistency checks over the roadmap, product contract, release
    process, verifier guide, and pilot runbook so their version identifiers, commands, and
    supported/non-supported claims cannot drift silently.

- [ ] 44.1 Perform a clean-checkout local closure rehearsal on the reference project. Generate
  the release bundle, run `xtask pilot-precheck`, verify the bundle from an independent directory
  with the distributed verifier, replay every required offline fixture, and record canonical
  evidence plus a human-readable result. A live testnet observation may be attached, but cannot
  substitute for any local acceptance evidence.
  - [ ] 44.1.1 Create a clean local workspace with only documented tools and public fixtures;
    build the verifier and reference project without relying on untracked artifacts or user-level
    project configuration.
  - [ ] 44.1.2 Execute the full local evidence chain in order: release precheck, pilot precheck,
    reference release, independent verification, authorization/tamper matrix, and offline target
    replay. Preserve the canonical report and its digest projection.
  - [ ] 44.1.3 Repeat the closure rehearsal in a second clean directory or by a second operator,
    compare canonical reports, and investigate any difference before proceeding to closure.

- [ ] 44.2 Conduct final pilot-readiness review. Mark Milestone 5 complete only when every
  local acceptance item passes, the external verifier walkthrough is reproducible by a second
  operator or clean local environment, and the published documents accurately match the bounded
  implementation. Any mainnet, token/custody, richer crypto, external-call, or broader language
  request starts a new design lock rather than extending this milestone implicitly.
  - [ ] 44.2.1 Review the local evidence report, independent-verifier results, reference fixture
    coverage, and documentation-drift results against every acceptance item; record any waived
    item as a blocker rather than a completion exception.
  - [ ] 44.2.2 Obtain a second-operator or second-clean-environment reproduction of the public
    verifier walkthrough and local closure rehearsal, then retain only non-secret reproducible
    outputs as pilot evidence.
  - [ ] 44.2.3 Publish the closure decision, exact verified versions/digests, pilot boundary,
    unresolved risks, and follow-on candidate list. Do not mark completion if any statement
    exceeds the locally verified scope.

## Deferred Follow-On Candidates

These are intentionally not Milestone 5 tasks. Reassess them only after pilot feedback identifies
a concrete blocked workflow and the existing local evidence remains green:

The broader, profile-by-profile production path is documented in
[`general_production_readiness_path.md`](general_production_readiness_path.md). It is planning
guidance, not additional Milestone 5 scope.

1. Mainnet readiness, confirmation/finality policy, hardware/remote signing, and operational key
   custody.
2. Richer authorization or cryptographic primitives, after their semantic/proof boundaries are
   independently specified and verified.
3. Dynamic ABI values, collections, `U256`, control flow, external calls, and broader EVM
   compatibility.
4. Token/custody/DeFi reference products, after the prerequisite language and target surfaces
   have their own threat-model and proof locks.
5. Additional chains, package ecosystems, installers, or hosted services.

## First Task

**40.0 — publish the Milestone 5 product, threat-model, pilot-user, and local-validation lock.**
It fixes the security claim and the locally testable completion boundary before any language or
distribution feature is added.

