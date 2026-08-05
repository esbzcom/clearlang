# Phase 30.1.0 - Contract State Schema and Migration Lock

Date: 2026-08-05
Status: Locked
Owner: language-and-proof-owner

## Purpose

Define the single source-level model for persistent contract state before parser, type checker,
storage lowering, or target-specific implementation work begins. The model must make every
deployed state transition and every upgrade boundary visible to users and proof tooling.

## Design Principles Check

- Simple for users: a contract has one explicit state block, one constructor, and an
  append-only versioning rule rather than a separate storage DSL or implicit global state.
- AI-friendly: source declarations produce canonical schema and layout manifests, so tools can
  generate, compare, and repair contract changes deterministically.
- Provably correct: pre-state (`old`) and post-state (`state`) references have unambiguous
  scopes; upgrades require a declared, checked migration instead of untyped storage casts.
- Crypto-focused: canonical schema/layout digests are included in signed release evidence, so a
  deployment cannot silently bind a proof to a different persistent-state layout.

## Source Model

Only a `contract` compilation unit may declare persistent state. The initial syntax shape is:

```clearlang
contract Vault version 1 {
    state {
        owner: Address;
        total: U64;
        balances: Map[Address, U64];
    }

    invariant { state.total >= 0 }

    init(owner: Address) {
        state.owner = owner;
        state.total = 0;
        state.balances = std::map::new();
    }

    mut function deposit(amount: U64) -> Unit
        require { amount > 0 }
        ensure { state.total == old(state.total) + amount }
    {
        state.total = state.total + amount;
    }
}
```

The exact token spelling may be adjusted only for parser ambiguity; the semantic names and
boundaries in this lock are fixed.

### Contract Unit

1. A source package may contain at most one deployable `contract` unit per declared entrypoint.
2. `version` is a positive, monotonically increasing integer and is part of the contract schema
   identity. Omitting it is an error for deployable contract builds.
3. `state`, `invariant`, and `init` are contract-only declarations. Ordinary modules and Wasm
   programs keep their existing semantics.
4. `init` executes exactly once for a new deployment and must initialize every state field on all
   successful paths. It cannot perform an outbound call.

### State Fields

1. A state block is required for a stateful contract and appears exactly once.
2. Field names are unique, declaration order is canonical, and fields are addressed in source as
   `state.<field>`.
3. Initial supported field types are deterministically serializable value types: booleans,
   signed/unsigned integers, strings, bytes, structs/enums composed of supported values, and
   `List`/`Set`/`Map` whose members are supported values.
4. Resources, closures, function values, host handles, dynamic-package references, and types with
   an unbounded or target-dependent representation are forbidden in state fields.
5. State reads are allowed in contracts; writes are allowed only in contract transition bodies and
   must target a declared field or an approved collection operation rooted at a declared field.

### Contracts, Snapshots, and Invariants

1. `state.<field>` in a `require`, `ensure`, or `invariant` expression denotes the current
   post-expression state.
2. `old(state.<field>)` denotes the value at entry to the current public contract transition.
   `old` accepts only state-rooted, pure expressions; nested `old`, local values, host reads, and
   calls with effects are rejected.
3. A public state-changing function must prove every contract invariant on exit. Invariants are
   assumed at public-transition entry only after constructor or migration validity is established.
4. `require` clauses describe caller-visible pre-state; `ensure` clauses describe relation between
   entry state, parameters, return value, and exit state. The VC artifact records their distinct
   scopes.
5. `pure` functions may read state only through an explicit read-only contract context. `mut`
   transition functions may update state. External calls are excluded from this gate and are
   governed by `30.2`.

## Schema and Layout Artifacts

Each contract build emits a canonical `contract-state-schema` artifact containing:

1. schema format version, contract package/name, and source `version`;
2. fields in declaration order with field name, canonical type, and stable field identifier;
3. invariant identifiers and normalized expressions;
4. constructor and migration entrypoint identifiers;
5. layout algorithm/version, target-profile identifier, and layout digest;
6. compiler version and source/artifact digests.

Stable field identifiers are derived from the contract identifier plus field name and are never
reused. The target adapter may choose its physical storage-key encoding, but it must consume this
artifact and prove that its generated keys/layout fingerprint match the canonical schema. The
schema digest and layout digest are signed release-bundle inputs.

## Upgrade and Migration Policy

1. A deployment upgrade must declare `version N`, where `N` is greater than the prior released
   version, and name the exact prior schema digest it accepts.
2. Additive field changes are allowed only by appending fields. Renaming, reordering, deleting,
   changing the type of, or reusing an existing field identifier is rejected.
3. Any non-additive state change requires a migration declaration:

```clearlang
migrate from schema "sha256:<prior-schema-digest>" {
    // pure, deterministic state transformation
}
```

4. A migration executes once, has exclusive state access, cannot make outbound calls, cannot
   consult ambient time/randomness/caller data, and must prove all new invariants on exit.
5. The migration artifact records both schema digests, the migration code digest, proof result,
   and target execution receipt. A release must reject an upgrade lacking this complete chain.
6. Downgrades, implicit migrations, fallback decoding, and automatic default values for new
   fields are forbidden. Rollback means deploying a separately versioned, forward migration;
   it never means reinterpreting newer state as an older schema.

## Diagnostics and Fail-Closed Rules

The implementation reserves deterministic contract diagnostics for at least:

- missing/duplicate state declaration or uninitialized state field;
- invalid state field type or state write outside a contract transition;
- illegal `old` scope or state reference;
- invariant not established/preserved;
- absent, mismatched, or non-monotonic schema version/digest;
- incompatible layout change, field-identifier reuse, or forbidden migration effect;
- release artifact whose state-schema or layout digest differs from its proof/target evidence.

All of these conditions block strict contract release. No permissive-mode result may be labelled
as a migration-safe or production-contract release.

## Non-Goals

This lock does not define:

1. external-call/reentrancy semantics, target storage-key mechanics, ABI encoding, or deployment
   procedures;
2. arbitrary live state transformations, proxy upgrades, or cross-contract storage sharing;
3. automatic schema evolution or backwards-compatible decoding heuristics;
4. multi-chain schema portability beyond the canonical target-neutral artifact;
5. stateful resources or garbage collection semantics.

## Exit Criteria for 30.1.0

1. The syntax and semantic boundary for `contract`, `state`, `init`, `invariant`, and `old` are
   locked.
2. Schema/layout artifact contents and signed identity requirements are defined.
3. Compatible upgrade, mandatory migration, and forbidden evolution rules are explicit.
4. The roadmap references this lock before 30.1 implementation begins.

## References

- `docs/todo/milestone_4_roadmap.md`
- `docs/design/phase-30.0.0-milestone-4-production-contract-platform-lock.md`
- `docs/design/phase-30.0.1-milestone-4-gate-governance-lock.md`
- `docs/proofs/vc-schema.md`
- `docs/proofs/proof-artifact-schema.md`
