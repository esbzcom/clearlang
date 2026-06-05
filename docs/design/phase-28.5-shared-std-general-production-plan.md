# Phase 28.5 - Shared Std General Production Plan

Date: 2026-06-05
Status: Locked for planning
Owner: std-arch-owner

## Purpose

Milestone 3 deliberately stops with:

1. `embedded` std as the production-safe default
2. `shared` std as an auditable experimental opt-in

That is the correct Milestone 3 stopping point, but it is not yet the product end state for a normal shared-std production path.

The purpose of this plan is to define the post-Milestone 3 execution order required to make `shared` std a usable, supportable, debt-free production option.

## Target Final State

The project reaches the final shared-std product state only when all of the following are true:

1. normal `clg pkg lock`, `clg release`, and `clg verify-bundle` workflows handle `shared` std without hand-edited or side-channel lockfiles
2. selected shared-std artifacts are represented by one canonical validated schema-v2 model across lock, runtime, release, provenance, and verification flows
3. production release fails closed whenever shared-std identity, trust, ABI, digest, provenance, or replay evidence is missing or mismatched
4. CI and release-precheck prove behavior end to end with command-level fixtures rather than source-shape audits
5. packaging, signing, provenance, upgrade, rollback, and support workflows are routine and documented
6. the project can explicitly promote `shared` from `experimental` to a supported production option without carrying known schema split, compatibility-branch, or validation duplication debt

`embedded` remains a valid production mode in this final state. This plan does not require `shared` to replace it as the default.

## Non-Goals

This plan does not authorize:

1. changing the default from `embedded` to `shared` before the exit criteria are met
2. preserving permanent schema-v1/schema-v2 compatibility debt in the normal shared-std release path
3. warning-only downgrade behavior for missing or invalid shared-std evidence
4. undocumented or profile-specific activation shortcuts
5. implicit migration of existing embedded-only projects

Because shared std is not yet an officially released normal production path, this plan intentionally does not preserve schema-v1 compatibility for `shared` workflows. The clean contract is to finish the schema-v2 design and reject older shared-std workflow shapes rather than carry migration debt forward.

## Debt-Free Design Rules

To avoid shipping a fragile product, every phase in this plan must preserve these rules:

1. one authoritative validator
   - shared-std lock semantics are validated, normalized, and serialized once and projected everywhere else
2. one normal workflow
   - normal `pkg lock` and `release` flows must produce the required evidence directly
3. behavior gates over shape gates
   - grep/string audits may exist as secondary drift checks, but they do not authorize production rollout
4. fail-closed activation
   - `shared` requests must either produce a complete evidence chain or fail immediately
5. product-readiness is part of done
   - packaging, docs, upgrade, rollback, and support playbooks ship with the code path
6. no compatibility branches for unreleased shared workflows
   - schema-v1 may continue to exist for embedded or historical workflows, but `shared` productionization is built around schema v2 only

## Execution Order

The post-Milestone 3 work must proceed in this order.

### Gate F - Normal Workflow Completion

The first blocker is that `shared` std must work in the ordinary release workflow rather than only in isolated schema-v2 scenarios.

Required outcomes:

1. `clg pkg lock --generate` and `--update` emit schema-v2 shared-std lock records when a project explicitly selects `shared`
2. `clg release` consumes that normal lock output and projects non-empty shared-std evidence when shared artifacts are selected
3. `clg release` rejects requested `shared` std if the regenerated lockfile downgrades, omits, or empties the shared-std evidence chain
4. `shared` std release flows reject schema-v1 lockfiles rather than attempting compatibility projection or fallback behavior

This gate removes the most immediate product blocker: a nominally wired feature that is not reachable from the supported release flow.

### Gate G - Evidence Model Unification

Once the normal workflow exists, all release-side readers must stop re-implementing partial schema parsing.

Required outcomes:

1. the current runtime-only schema-v2 validator is extracted into a reusable command-layer shared-std lock model rather than leaving separate raw readers in runtime, release, and strict import-map paths
2. the authoritative validated model retains all fields required by downstream projections:
   - package id and version
   - verified std ABI range
   - artifact path, digest, and size
   - signer key id / algorithm / signed_at / signature
   - provenance digest / format
   - symbols and shared-std dependency ids
3. `pkg lock` writer/serializer/normalizer, runtime loading, strict import-map projection, release manifest generation, provenance generation, and `verify-bundle` all consume that validated schema-v2 model
4. partial release/import-map JSON readers are deleted or reduced to thin projections over that validated model
5. release-side and import-map-side negative tests prove the same validation contract for duplicates, ordering, dependency integrity, digest format, signer fields, artifact size/path, provenance shape, and delivery-mode semantics
6. deterministic upgrade/rollback coverage proves `pkg lock --update` and release/verify flows remain stable across shared-std version changes, ABI-range changes, signer rotation, provenance rotation, and rollback to earlier locked shared-std versions

This gate removes the structural tech debt of having multiple near-duplicate validators with inconsistent safety contracts.

### Gate H - Command-Level Release and CI Gates

Once the model is unified, rollout gates must prove behavior rather than merely prove strings exist in source files.

Required outcomes:

1. shared-std activation gates run fixture-driven `clg release` and `clg verify-bundle` flows
2. the happy-path gate proves non-empty shared-std evidence survives across:
   - lockfile
   - strict import-map
   - provenance payload
   - release bundle manifest
3. negative-path gates prove fail-closed behavior for:
   - tampered manifest/import-map/provenance evidence
   - missing artifact
   - digest mismatch
   - signer/provenance mismatch
   - ABI mismatch
   - replay or upgrade drift
4. cheap drift audits may remain, but they cannot be the primary activation gate

This gate removes rollout debt by aligning CI with the actual supported product contract.

### Gate I - Productization and Operational Readiness

A normal production path is not just a passing test. It must also be operable.

Required outcomes:

1. shared-std artifact publishing and signing workflows are routine
2. provenance statements and key rotation policies are integrated into the release process
3. registry/distribution rules are documented and deterministic
4. user-facing docs cover:
   - explicit shared-std opt-in
   - release and verify flows
   - upgrade and rollback
   - incident response
5. support playbooks exist for diagnosing packaging, trust, ABI, and replay failures without relying on hidden fallback behavior

This gate removes product debt by making the feature supportable outside the original implementation context.

### Gate J - Rollout Upgrade

Only after Gates F-I are complete may the project revisit the rollout decision.

Required outcomes:

1. explicit review confirms there are no unresolved fail-closed gaps
2. explicit review confirms there is no remaining schema split or duplicate-validation debt in the supported workflow
3. the project decides whether `shared` becomes a supported production option
4. the final product contract is locked:
   - supported activation semantics
   - CI/profile coverage expectations
   - compatibility and deprecation rules
   - documentation and operations ownership

## Usable Product Contract

When this plan is complete, `shared` std is a usable product only if an end user can do all of the following through documented, supported flows:

1. select `shared` std explicitly in project inputs
2. run `clg pkg lock` and get the correct locked shared-std records
3. run `clg release` and obtain a deterministic, signed, proved release bundle with non-empty shared-std evidence
4. run `clg verify-bundle` and confirm the shared-std evidence chain end to end
5. upgrade or roll back shared-std package versions with deterministic diagnostics
6. diagnose failures using documented, stable error contracts

If any of those require hidden manual steps, out-of-band file editing, or undocumented recovery procedures, the product is not done.

## Exit Criteria

This plan is complete only when:

1. the normal shared-std release path is reachable and fail-closed
2. the supported workflow has no schema split, compatibility-branch, or duplicate-validation debt
3. release-precheck and CI validate behavior end to end for representative shared-std matrices
4. packaging and operational support are routine and documented
5. deterministic upgrade/rollback and rotation scenarios are covered as supported workflow behavior
6. the project explicitly records the rollout decision to keep `shared` experimental or promote it to supported production based on green Gates F-I

When these criteria are met, the project can achieve the final state of `shared` std as a normal production path without carrying known architectural or rollout debt.
