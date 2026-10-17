# Verified Profile Packs

Status: proposed post-Milestone-6 architecture. This document fixes the
vocabulary and trust boundary for future work; it does not add a language
feature, a CLI command, or a Milestone 6 acceptance requirement.

## Decision

ClearLang will call a narrowly supported, evidence-bound application contract a
**Verified Profile**. Its signed, installable distribution is a **Profile Pack**.

Use these terms consistently:

| Term | Meaning |
| --- | --- |
| Standard-library package | General reusable API surface, such as `std::eth`; it supplies primitives but makes no application or deployment claim. |
| Capability | A compiler-approved semantic feature with defined typing, proof, IR, target, diagnostic, and compatibility behavior. |
| Verified Profile | A bounded, named combination of existing capabilities, standard-library packages, target profile, verification policy, and operational claim. |
| Profile Pack | The signed/versioned artifact that declares and distributes one Verified Profile and its source template, fixtures, evidence policy, documentation, and metadata. |
| Plugin | Deliberately not the model for Profile Packs. A plugin may imply arbitrary executable extension code or unreviewed compiler behavior. |

In conversation, **Profile Pack** is the preferred short name. Do not use bare
`package` when the distinction from `std` or dependency packages matters.

## Why This Fits ClearLang

Profile Packs make expansion easier for users without making the language or
its security claims open ended.

| Design principle | Profile Pack rule |
| --- | --- |
| Simple for users | A user selects one named profile and receives one template, validation path, and operations guide rather than assembling an unsafe combination of features. |
| AI-friendly | The manifest, compatibility rules, diagnostics, fixtures, and non-goals are versioned and machine-readable. An AI tool can propose a pack only by naming its exact profile and inputs. |
| Provably correct | A pack may select only capabilities with an approved proof model. Its identity is bound into the proof, release, and verifier evidence; unsupported combinations reject. |
| Crypto-focused | Packs center signer authority, target correspondence, replay/tamper resistance, and operational evidence before adding convenience or language breadth. |

## Relationship to `std`, Dependencies, and Plugins

`std` is ClearLang's toolbox. It provides primitives such as types, helpers,
and chain-specific APIs. A Profile Pack is a constrained workflow that selects
some of those primitives and specifies the only supported way to use them for
one product claim.

For example, `std::eth::Address` can be available to many programs. A pack
such as `clg.profile.config-registry.v1` would constrain that capability to a
fixed-owner configuration registry, define its allowed target/profile,
required release evidence, target observation rules, and explicit exclusions.

The closest familiar ecosystem concepts are informative but not exact:

| ClearLang | Approximate analogue | Important difference |
| --- | --- | --- |
| Standard-library package | Rust standard library, Java standard library, Solidity standard types | General primitives, not an application-safety contract. |
| Dependency package | Cargo crate, npm package, Move package | Reusable code may be broad; it does not normally bind deployment and assurance policy. |
| Capability selection | Cargo feature flag | A feature flag enables code; it normally has no proof, release, or operating boundary. |
| Verified Profile / Profile Pack | Helm chart plus policy bundle plus audited reference template | A ClearLang pack must also bind compiler/verifier compatibility and proof/release evidence. |

## Trust Boundary

A Profile Pack is declarative. It must not become a way to introduce unreviewed
semantics or execute code inside compiler or verifier processes.

### A pack may

- select already-approved compiler capabilities and `std` dependencies;
- identify one source/template shape and its allowed parameters;
- select one target and assurance profile already supported by the compiler and
  verifier;
- provide fixtures, documentation, schemas, and deterministic policy metadata;
- declare explicit limits, non-goals, compatibility ranges, and required trust
  material; and
- be signed, versioned, resolved, and bound into build/release/verification
  artifacts.

### A pack may not

- add syntax, types, effects, proof axioms, VC generation, IR operations, or
  target-code generation;
- run arbitrary hooks, macros, installers, build scripts, verifier extensions,
  or hidden network actions;
- weaken diagnostics, release policy, proof status, or verifier checks;
- infer a wallet, signer, network, nonce, fee, endpoint, or secret; or
- silently substitute dependencies, target profiles, trust keys, templates, or
  evidence rules.

An unknown pack schema, missing signature/trust-policy approval, unsupported
capability, conflicting dependency, incompatible compiler/verifier version, or
unrecognized target profile must reject before build or release. This is a
fail-closed configuration boundary, not a sandbox for third-party code.

## Illustrative Manifest Shape

The following is descriptive only; it is not a shipped schema or command
syntax. A future design lock must specify canonical JSON, signing, resolution,
and diagnostic behavior before implementation.

```json
{
  "kind": "clg.verified-profile-pack.v1",
  "id": "clg.profile.config-registry.v1",
  "version": "1.0.0",
  "profile": "clg.config-registry.fixed-owner.v1",
  "requires": {
    "compiler": "^0.1.0",
    "capabilities": ["evm-stateful-scalar", "eth-caller-authorization"],
    "std_packages": ["std::contract", "std::eth"],
    "target_profiles": ["clg.evm-stateful-scalar.v1"],
    "assurance_profiles": ["clg.production-registry.v1"]
  },
  "artifacts": {
    "template": "templates/config_registry.clear",
    "fixtures": "fixtures/",
    "operations_guide": "docs/operator.md"
  },
  "non_goals": ["value-transfer", "external-calls", "dynamic-abi"]
}
```

The resolved pack identity, every selected dependency identity, and approved
trust-policy identity must be included in the project lockfile and in all
applicable proof, release-bundle, deployment-evidence, and verifier reports.
The resolver must choose one canonical result or fail; it must not guess,
float, or merge conflicting pack policies.

## First Profile Pack

The first pack should be:

| Field | Value |
| --- | --- |
| Display name | Configuration Registry Profile |
| Stable identifier | `clg.profile.config-registry.v1` |
| Product | The existing fixed-owner, scalar configuration registry. |
| Capability rule | Uses only capabilities already approved for the Milestone 5/6 registry profile. |
| Target rule | Selects a separately approved one-chain target profile; it does not make all EVM chains supported. |
| Security rule | Binds the reference template, proof/release evidence, verifier policy, operator policy, fixtures, and known exclusions. |

This is intentionally a packaging and assurance exercise, not a language
expansion. It validates discovery, signature/trust validation, deterministic
resolution, project locking, artifact binding, offline verification, and a
simple user workflow with the smallest possible trusted surface.

An eventual user flow may resemble `clg profile add`, `clg profile inspect`,
and normal build/release commands, but these command names are illustrative
until the Profile Pack CLI lock is approved.

## Target Profiles and Future EVM Networks

Adding an EVM network must not create a new language dialect, nor should it
fork the application-level Configuration Registry Profile Pack. The supported
unit of network expansion is a separately signed, declarative **Target
Profile**. When it is distributed independently, call the artifact a **Target
Profile Pack**.

The relationship is intentionally two-layered:

| Layer | Example | Owns | Must not own |
| --- | --- | --- | --- |
| Product Profile Pack | `clg.profile.config-registry.v1` | Source/template shape, proof and release policy, application limits, fixtures, and operator workflow. | Chain selection by inference, an RPC endpoint, signer credentials, or generic EVM compatibility. |
| Target Profile Pack | `clg.target.evm.ethereum-mainnet.v1` | Exactly one chain identity, supported transaction envelope, RPC/observation contract, finality/reorganization rules, client support observations, and target-specific evidence policy. | Syntax, semantics, proof rules, arbitrary code, wallet discovery, or the right to change the product boundary. |
| Resolved deployment profile | `config-registry.v1` + `ethereum-mainnet.v1` | One canonical, lockfile-bound combination used for a release and a target-action request. | Automatic fallback to another chain or a floating “any EVM” claim. |

Thus the first post-Milestone-6 implementation can ship one product Profile
Pack and one Target Profile Pack for Ethereum Mainnet. A later Base, Arbitrum,
or other EVM-compatible network normally adds **another Target Profile Pack**,
not another product pack and not a compiler feature. If product constraints
also change—for example, governance replaces the fixed owner—that is a new
product Profile Pack in addition to any target work.

### Decoupled evolution decision

Product Profile Packs and Target Profile Packs have separate, versioned
evolution paths. They are decoupled in ownership and release cadence, but they
are never loosely coupled at deployment time: a release resolves one exact,
explicitly compatible pair.

| Change | Pack that evolves | Required consequence |
| --- | --- | --- |
| Source/template, application limit, proof/release policy, or operator workflow changes | Product Profile Pack | Release a new product-pack version; retain target packs unchanged unless the compatibility rule changes. Re-run every affected product-level fixture and evidence gate. |
| Chain ID, supported transaction envelope, finality/reorganization policy, RPC contract, or client-support policy changes | Target Profile Pack | Release a new target-pack version; retain product packs unchanged unless the compatibility rule changes. Re-run every affected target-level fixture and evidence gate. |
| A new network is added | New Target Profile Pack | Use a new stable target ID and its own promotion evidence. Do not edit an existing Ethereum target pack to mean a different network. |
| The product and target both change | Both packs | Resolve and review the new pair as a new deployment profile; neither prior approval transfers automatically. |
| Syntax, semantics, proof model, standard-library capability, IR, code generation, or verifier behavior changes | Capability/language design first | Do not express the change as a pack version. Complete the separate capability process, then publish compatible pack revisions. |

The resolver must lock exact IDs and versions, for example
`clg.profile.config-registry@1.2.0` with
`clg.target.evm.ethereum-mainnet@1.1.0`. It must reject an absent,
incompatible, revoked, or ambiguous pair. Updating either pack requires an
explicit lockfile update and the evidence required by the affected change;
there are no floating versions, automatic upgrades, or cross-chain fallback.

Compatibility is directional and declared by the packs: a product pack names
the target-profile schema/ID/version ranges it accepts, and a target pack names
the product-profile schema/ID/version ranges it accepts. The resolver accepts
only their intersection. Trust-policy changes, signing-key rotation,
revocation, or a severity fix can invalidate an otherwise version-compatible
pair and must fail closed until an approved replacement is locked.

### Implementation path for an additional EVM target

An additional target is admitted only through this sequence:

1. **Propose one exact chain.** Assign a stable ID such as
   `clg.target.evm.<network>.v1`; lock its chain ID, native-value rule, and
   supported transaction envelope. “EVM-compatible” alone is not an input.
2. **Design and sign the target manifest.** Declare compatible compiler,
   verifier, product-profile, standard-library, and assurance-profile ranges;
   bind one canonical chain ID and fail closed on every mismatch.
3. **Implement declarative resolution.** Extend the resolver and project
   lockfile so an explicit product/target pair resolves deterministically and
   is bound into proof, release, request, observation, and offline-verifier
   artifacts. No installer hooks, scripts, or network probes may run during
   resolution.
4. **Validate the target boundary.** Add deterministic positive, wrong-chain,
   wrong-ID, unsupported-transaction, RPC-shape, receipt/event, client-
   disagreement, reorganization, tamper, downgrade, offline, and no-secret
   fixtures. Obtain clean-environment reproduction and independent review.
5. **Promote the exact combination.** Run the same security, correspondence,
   precheck, launch-decision, and closure gates as Milestone 6 for that
   product/target combination. A prior Ethereum result is evidence only for
   Ethereum; it never authorizes another chain.

This path assumes the new chain accepts the existing bounded target behavior.
If it needs a new opcode assumption, transaction semantic, precompile,
standard-library API, proof model, or verifier behavior, stop before Step 2.
That is a separate language/capability change and must follow the
Language-Feature Decision process below before any pack can declare it.

### First target decision

Milestone 6 selects **Ethereum Mainnet**, chain ID `1`, as its only production
target. The selection is documented in
[`phase-50.2-ethereum-mainnet-selection.md`](phase-50.2-ethereum-mainnet-selection.md).
It is a planning and policy decision only: the remaining target-profile lock,
security gates, and explicit launch approval are still required before any
mainnet submission.

## Real Use Case: Public Configuration-Revision Beacon

The first profile is useful for a small non-custodial application that needs a
public, auditable answer to: **which published configuration revision is
currently authorized?**

For example, an open-source service can publish its human-readable integration
configuration outside the chain as numbered releases: `configuration-v41`,
`configuration-v42`, and so on. The Configuration Registry Profile stores only
the approved revision number. Wallets, indexers, integrators, and support
staff can independently read the registry and agree that revision `42` is the
currently announced revision without trusting a mutable web page or a private
database.

This is a deliberately modest use case. The registry is a public coordination
and audit point, not the source of the configuration itself and not an
enforcement engine for another application.

### What is deployed

The first-pack reference source is the v1 `ConfigRegistry`; it has exactly this
state and transition:

| Item | Meaning |
| --- | --- |
| `owner: std::eth::Address` | The one address permitted to announce a new revision. It is initialized once and cannot change in the initial profile. |
| `value: U64` | The announced configuration revision, from `0` through `18,446,744,073,709,551,615`. |
| `set_value(next)` | The sole mutable operation. It accepts the owner's caller identity, writes `next`, and emits one `ValueChanged(next)` event. |

No value is transferred, no other contract is called, no list or map is
maintained, and no hidden signer, endpoint, nonce, or fee is selected by the
tooling.

### A real operator flow

1. The application team publishes and reviews its off-chain configuration as
   revision `42` through its normal public release process.
2. The release owner builds, proves, and independently verifies the exact
   `clg.profile.config-registry.v1` release bundle and records the approved
   target inputs.
3. The authorized operator submits the explicit, zero-value
   `set_value(42)` request under the approved signer and transaction policy.
4. The profile captures the resulting `ValueChanged(42)` observation, receipt,
   and release identities; they are replayed and verified offline.
5. An integrator reads `value == 42` and the corresponding event history, then
   selects the separately published configuration revision `42` using its own
   normal trust and availability process.

The on-chain evidence answers *who announced which revision and when under the
selected target's observation rules*. The independent release verifier answers
whether the deployed candidate corresponds to the reviewed ClearLang source
and release evidence. These are different claims and must remain separate.

### What this example does not do

- It does not store, hash, fetch, validate, or guarantee availability of the
  off-chain configuration document.
- It does not force any client, wallet, indexer, or service to follow the
  announced revision.
- It does not rotate the owner, implement multiple owners or multisig, or
  provide recovery, delegation, pause, timelock, or role management.
- It does not authorize transfers, hold assets, make economic decisions, or
  provide token, custody, DeFi, proxy, or external-call behavior.

Binding a configuration content digest, managing multiple configuration keys,
or introducing governance would be a separate future capability/profile
decision. None is required to validate the first Profile Pack architecture.

## Language-Feature Decision

No new language feature is required for either Milestone 6 or the first
Configuration Registry Profile Pack. It relies on the existing bounded registry
surface and validates the pack mechanism without broadening the compiler,
solver, target, or verifier trust boundary.

A later Profile Pack that needs a new capability must not smuggle it through a
manifest. It requires, in this order:

1. a concrete user or operator workflow blocked by the current profile;
2. a separate capability design/threat-model/proof-claim lock;
3. defined parser/type/effect, VC/solver, IR, target, verifier, diagnostic,
   and compatibility behavior;
4. deterministic positive, negative, tamper, replay, and differential fixtures;
5. clean-environment evidence and proportional independent review; and
6. a new or revised Profile Pack that declares the approved capability
   explicitly.

Candidate later packs include a governed registry (fixed roles and threshold
administration), a bounded allowlist, and bounded parameter policy. Signed
intents, tokens, custody, value transfer, external calls, and DeFi remain
separate high-risk programs rather than ordinary pack additions.

## Adoption Sequence

Profile Pack work begins only after Milestone 6 has closed for its named
registry profile. It belongs to the subsequent production-platform-foundation
work, in this order:

1. lock the manifest, identity, signing, trust policy, resolver, project-lock,
   compatibility, and diagnostic contracts;
2. implement declarative resolution and artifact binding using only the
   existing Configuration Registry Profile;
3. add deterministic happy-path, conflict, tamper, downgrade, unknown-version,
   unsafe-path, and no-network fixtures;
4. publish the independent install/inspect/verify workflow and reproduce it
   from a clean directory; and
5. consider a new capability only when a concrete bounded workflow is blocked.

No generic plugin API is a goal of this sequence.

## Deferred Implementation Task List

This is the post-Milestone-6 task list. It is intentionally not a Milestone 6
acceptance requirement and it begins only after the Ethereum-bound registry
profile has a closed evidence record.

- [ ] **P0 — lock pack and evolution contracts.** Specify canonical manifest
  schemas; product/target identities; independent versioning; directional
  compatibility; signing, trust, revocation, and deprecation; exact lockfile
  fields; stable diagnostics; and the no-network/no-code resolution boundary.
- [ ] **P1 — implement fail-closed resolution.** Resolve one explicit product
  pack plus one explicit target pack deterministically; reject unknown,
  unsigned, incompatible, revoked, ambiguous, unsafe-path, or floating inputs;
  bind the resolved pair into build, proof, release, request, and verifier
  artifacts.
- [ ] **P2 — ship the first product pack.** Package the existing Configuration
  Registry template, its proof/release fixtures, operator documentation,
  exclusions, and evidence requirements without changing source semantics.
- [ ] **P3 — ship the Ethereum Target Profile Pack.** Package the completed
  Ethereum Mainnet chain/transaction/RPC/finality policy and its target
  fixtures. This step reuses the Milestone 6 decision; it does not reopen the
  product boundary.
- [ ] **P4 — test evolution and recovery.** Add clean-directory, offline,
  positive, conflict, downgrade, target-mismatch, tamper, signer-key rotation,
  revocation, and deprecation fixtures. Prove that an older locked pair remains
  reproducible unless revoked by the approved policy.
- [ ] **P5 — admit each later EVM target separately.** For each proposed chain,
  create a new Target Profile Pack, complete the additional-EVM-target sequence,
  and publish a fresh product/target combination record. Do not add a generic
  `evm` target or make existing packs float to the new chain.

## References

- `README.md` (design principles and separation of concerns)
- `docs/todo/milestone_6_roadmap.md`
- `docs/todo/general_production_readiness_path.md`
- `docs/runtime/chain-packages.md`
- `docs/release/milestone_5_pilot_product_contract.md`
