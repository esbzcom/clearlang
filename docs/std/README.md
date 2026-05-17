# ClearLang Standard Library Packages

This directory defines the Phase 26 draft package surface for a production-grade standard library.

First production release policy:
- embedded std linking only,
- deterministic dead-code elimination (tree-shaken symbol emission),
- dynamic/shared std linking deferred to a follow-up phase.

## Package Index

### Must-Have (first production release target)
- [`std::core`](core.md)
- [`std::str`](str.md)
- [`std::bytes`](bytes.md)
- [`std::int`](int.md)
- [`std::collections`](collections.md)
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
