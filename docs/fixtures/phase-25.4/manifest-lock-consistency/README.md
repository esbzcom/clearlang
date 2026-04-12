# Phase 25.4 Manifest/Lock Consistency Fixture

This fixture is used by the CI drift gate (`xtask manifest-lock-drift-check`) to fail closed when:
- `clg.project.json` dependency roots change without lockfile updates, or
- `clg.lock.json` roots diverge from manifest dependency declarations,
- `clg.lock.json` bytes drift from canonical tool-owned format, or
- resolved-graph/hash sidecars diverge from lock identity.

Scope:
- Root/dependency consistency contract (`project.name` + `dependencies[]` vs `lock.roots[]`).
- Canonical lockfile byte-shape contract (`clg.lock.json` must match canonical serializer output).
- Resolved graph replay contract (`clg.resolved-graph.json` + `clg.resolved-graph.sha256` must match lock identity and canonical graph hash).
