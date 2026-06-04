# Phase 28.1.2 - Shared Std Migration and Coexistence Diagnostics

Date: 2026-06-04
Status: Locked
Owner: std-arch-owner

## Purpose

Define fail-closed migration and coexistence diagnostics for projects that mix embedded-only assumptions with shared std package inputs.

This phase does not introduce new feature capability.
It locks deterministic failure behavior during the transition from:

1. embedded-only manifest/lock assumptions
2. mixed embedded/shared declarations
3. schema-v1-only project expectations

to the Phase 28 shared std model.

## Scope

This coexistence policy applies to:

1. `clg.project.json`
2. `clg.lock.json`
3. canonical package metadata / lock generation flows
4. build/update flows that compare manifest intent against existing lock state

## Core Policy

The project must never silently reinterpret embedded-only inputs as shared-std inputs, or shared-std inputs as embedded-only inputs.

Migration is explicit and fail closed.

## Coexistence Rules

### 1. Schema v1 Embedded-Only Assumption

`clg.project.json` schema v1 means:

1. no shared std intent exists
2. std delivery is implicitly embedded-only
3. any shared std section or shared std requirement is invalid

If shared std intent appears alongside schema v1 assumptions, the command must fail closed.

### 2. Schema v2 Shared-Std Intent

`clg.project.json` schema v2 with `std` section means:

1. std delivery intent is explicit
2. lock generation/update must preserve that intent exactly
3. embedded-only lockfiles or defaults are not silently reused when they conflict with explicit shared std intent

### 3. Existing Lockfile Coexistence

When updating an existing lockfile:

1. an embedded-only lockfile must not be silently upgraded to shared std mode
2. a shared std lockfile must not be silently downgraded to embedded-only mode
3. explicit manifest intent must match existing lockfile std-delivery intent exactly
4. if intent differs, operator action is required before update proceeds

### 4. Dependency-Space Separation

A shared std package id must not appear in both:

1. normal user dependency space
2. the dedicated shared std package space

If the same logical std package is declared in both places, the command must fail closed and require the operator to normalize the project to one canonical std section.

## Diagnostic Mapping

Shared std coexistence diagnostics reuse the existing deterministic metadata-model conflict family.

### `C109` is required for:

1. schema-v1 project inputs combined with shared std declarations
2. embedded-only manifest intent conflicting with shared std lock intent
3. shared std manifest intent conflicting with embedded-only lock intent
4. shared std package ids appearing in both normal dependency space and std package space
5. mixed mode declarations across manifest, lockfile, and explicit mode-selection inputs

### `C111` is required for:

1. update/generate flows where an existing lockfile cannot satisfy the required shared std transition contract
2. malformed or incomplete existing lockfile state that prevents deterministic migration

### `C027` is required for:

1. malformed shared std shape encountered during permissive/standard metadata-indexing paths
2. invalid shared std package metadata model in non-strict indexing flows

No new dedicated diagnostic code is introduced for this migration step.

## Deterministic Message Contract

When coexistence fails, the message must always name:

1. the source of the old assumption
   - manifest schema v1
   - embedded-only lockfile
   - normal dependency space
2. the conflicting new shared std input
3. the canonical corrective action

The corrective action must be one of:

1. upgrade manifest to schema v2 and declare `std` explicitly
2. regenerate lockfile from manifest intent
3. remove duplicate std package declaration from normal dependency space
4. align explicit delivery-mode input with manifest/lock state

## Canonical Failure Cases

The following are explicitly locked:

1. `clg.project.json` schema v1 plus `std.delivery = shared` intent -> `C109`
2. manifest schema v2 requests `shared` but existing `clg.lock.json` is embedded-only -> `C109`
3. manifest schema v2 requests `embedded` but existing `clg.lock.json` records shared std packages -> `C109`
4. `dependencies[]` includes `std::text` while `std.packages[]` also includes `std::text` -> `C109`
5. lock update over stale schema-v1 lock assumptions where deterministic std mode cannot be inferred -> `C111`
6. permissive/standard indexing encounters malformed shared std section shape -> `C027`

## Migration Guidance

The locked migration path is:

1. upgrade `clg.project.json` to schema v2
2. move shared std intent into `std.delivery` + `std.packages[]`
3. remove shared std package ids from normal `dependencies[]`
4. regenerate lockfile under schema v2
5. only then proceed with update/build flows that rely on shared std intent

## Gate Closure Meaning

`28.1.2` is complete once the project has a deterministic answer to:

"What happens when an embedded-only project or lockfile collides with shared std inputs?"

The answer is:

"The command fails closed with deterministic coexistence diagnostics and explicit migration guidance."

## References

- `docs/design/phase-25.4.6-manifest-lock-migration-coexistence-policy.md`
- `docs/design/phase-28.1.0-shared-std-manifest-lock-extension.md`
- `docs/design/phase-28.1.1-shared-std-resolver-lock-validation.md`
