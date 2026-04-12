# Phase 25.4 Manifest/Lock Consistency Fixture

This fixture is used by the CI drift gate (`xtask manifest-lock-drift-check`) to fail closed when:
- `clg.project.json` dependency roots change without lockfile updates, or
- `clg.lock.json` roots diverge from manifest dependency declarations.

Scope:
- Root/dependency consistency contract only (`project.name` + `dependencies[]` vs `lock.roots[]`).
- Full package solver/metadata compatibility is covered by separate `pkg lock` tests.
