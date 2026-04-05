# ClearLang Unit Test + Mock Example (Gate D Planning)

This project is an example layout for Phase `25.3` (`clg test` + deterministic mocks).

Status:
- `clg test` runner core is shipped in the current branch.
- Mock-set execution remains fail-closed until deterministic mock-runner slices land.

## Layout

- `main.clear`: runnable app entrypoint for current `clg run` workflows.
- `domain/`, `services/`: production modules.
- `tests/unit/*.clear`: unit test sources with `test_*` functions.
- `tests/mocks/<set>/...`: mock sets with production-matching module paths.
- `tests/test-plan.json`: deterministic per-test mock binding plan.

## Current verification (available now)

```powershell
clg run examples/projects/testing/main.clear
```

## Gate D runner invocation (available now)

```powershell
clg test examples/projects/testing --report json
```

IDE/profile shape (planned):

```powershell
clg --non-interactive --json-errors --json-events test examples/projects/testing --report json
```

## Why this example exists

- Demonstrates deterministic mock binding via `tests/test-plan.json`.
- Demonstrates layered mock sets (`common` + `promo`) for per-test behavior.
- Demonstrates release-safety intent: test/mock files are test-only and must not enter release artifacts.
