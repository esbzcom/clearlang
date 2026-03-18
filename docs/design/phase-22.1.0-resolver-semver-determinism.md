# Phase 22.1.0 - Deterministic Resolver and Semver Solver Policy

## Status
Design lock target for `22.1.0` in `docs/TODO.md`.

## Goal
Lock deterministic rules for transitive dependency resolution and semver solving.

## Inputs
1. Canonical metadata v1 (`clg.package-metadata.json`)
2. Trust policy
3. Root dependency requirements
4. Optional existing lockfile baseline
5. Optional advisory constraints (deny/forced-upgrade)

## Deterministic Resolver Policy
1. Build candidate package sets from trusted local source only.
2. Resolve package names in lexical order.
3. For each requirement, candidate versions are sorted:
   - highest semantic version first,
   - tie-break by digest lexical ascending,
   - final tie-break by artifact path lexical ascending.
4. Backtracking order is deterministic:
   - earliest unresolved package name first,
   - first candidate in sorted list first.
5. Cycles are detected on package id graph (`name@version`) and reported with canonical cycle path formatting.

## Deterministic Semver Policy
1. Requirement grammar accepted:
   - exact: `=1.2.3`
   - compatible: `^1.2.3`
   - patch range: `~1.2.3`
2. Pre-release handling:
   - excluded by default unless the requirement explicitly includes pre-release.
3. If multiple candidates satisfy constraints, choose deterministic winner per resolver ordering.
4. If no candidate satisfies all constraints, fail with deterministic unsat diagnostic.

## Output Lock
1. Resolved graph artifact and lockfile are canonicalized and hashable.
2. Diagnostics are emitted in stable order:
   - code -> package name -> package version -> symbol/import.

## Non-Goals
- Runtime loading behavior (Phase 23).
- Registry/network protocol design (host ops layer).

## References
- `docs/TODO.md` (`22.1.0`, `22.1.1`, `22.1.2`, `22.1.4`)
- `docs/design/phase-22.0.3-lockfile-v1.md`
- `docs/design/phase-22.1.3-vulnerability-response-policy.md`

