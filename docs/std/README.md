# ClearLang Standard Library Namespaces

This directory defines the Phase 26 draft namespace surface for a production-grade standard library.

First production release policy:
- embedded std linking only,
- deterministic dead-code elimination (tree-shaken symbol emission),
- dynamic/shared std linking deferred to a follow-up phase.

## Namespace Index

### Must-Have (first production release target)
- [`std::core`](core.md)
- [`std::str`](str.md)
- [`std::bytes`](bytes.md)
- [`std::int`](int.md)
- [Collections catalog (`std::list`, `std::set`, `std::map`)](collections.md)
- [`std::codec`](codec.md)
- [`std::crypto`](crypto.md)
- [`std::host`](host.md)
- [`std::unit`](unit.md)
- [`std::contract`](contract.md)

### Stretch / Deferred
- [`std::chain::<target>`](chain-targets.md)
- [`std::dynamic` (shared std runtime linking)](dynamic.md)

## Notes
- API details and proof contracts are expected to evolve under Phase 26 gates.
- Production release profile remains fail-closed for unsupported or non-proved std surfaces.
- Normative keywords in std package contracts:
  - `MUST`: mandatory requirement for conforming implementations.
  - `MUST NOT`: prohibited behavior.
  - `SHOULD`: recommended behavior; deviations require explicit justification.
- Conventions used in this directory:
  - `Sub-Namespaces` are logical API groups for documentation; they are not automatically importable module paths.
  - Concrete import paths are listed under an `Import Paths` section when they differ from the top-level namespace file name.
  - Function signatures use explicit first-argument form (no receiver shorthand), aligned with namespace-style APIs.

## Contract Maturity Matrix
- `std::core`: `draft` (normative baseline defined; implementation/proof parity pending)
- `std::str`: `draft` (API surface defined; UTF-8 canonicality and edge-case contracts pending)
- `std::bytes`: `draft` (constant-time contract defined at API level; measurement/conformance policy pending)
- `std::int`: `draft` (checked/wrapping/saturating model defined; full deterministic error-code map pending)
- Collections catalog (`std::list`, `std::set`, `std::map`): `draft` (deterministic behavior and finite-set proof contracts defined; list/map proof-roadmap closure pending)
- `std::codec`: `draft` (canonical encoding intent defined; full wire-format lock pending)
- `std::crypto`: `draft` (typed crypto API defined; algorithm-specific canonical encoding rules pending)
- `std::host`: `draft` (capability/fail-closed boundaries defined; strict conformance suites pending)
- `std::unit`: `draft` (minimal deterministic assertion baseline defined)
- `std::contract`: `draft` (chain-agnostic domain model defined; canonical address/amount constraints pending)
- `std::chain::<target>`: `deferred`
- `std::dynamic`: `deferred`

