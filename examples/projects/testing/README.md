# ClearLang Unit Test + Mock Example (Gate D Planning)

This project is an example layout for Phase `25.3` (`clg test` + deterministic mocks).

Status:
- `clg test` runner core is shipped in the current branch.
- Deterministic mock-set execution is shipped (explicit binding + deterministic merge/override).

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

IDE/profile shape (available now):

```powershell
clg --non-interactive --json-errors --json-events test examples/projects/testing --report json
```

## Why this example exists

- Demonstrates deterministic mock binding via `tests/test-plan.json`.
- Demonstrates balanced critical-path coverage with both non-mocked and mocked (`promo`) cases.
- Demonstrates release-safety intent: test/mock files are test-only and must not enter release artifacts.
