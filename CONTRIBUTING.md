# Contributing

Thanks for contributing to Clear Language.

## Development Setup

1. Install Rust stable toolchain.
2. Clone the repository.
3. Build once:
   - `cargo build --workspace`

## Branching Model

- `main` is the only long-lived production branch.
- Do not push directly to `main`.
- Use short-lived branches and open a PR:
  - `feat/<topic>` for feature work
  - `fix/<topic>` for bug fixes
  - `hotfix/<topic>` for urgent production fixes
- Keep branches small and rebase/merge frequently from `main`.

## Before Opening a PR

Run these checks locally:

1. `cargo fmt --all -- --check`
2. `cargo clippy --workspace --all-targets -- -D warnings`
3. `cargo test --workspace`
4. `cargo run -p xtask -- milestone2-perf-gate --self-test`
5. `cargo run -p xtask -- milestone2-supply-chain-gate --self-test`

## Pull Requests

- Keep changes focused and minimal.
- Include tests for behavior changes.
- Update docs when user-facing behavior changes.
- Reference related issues in the PR description.
- Target `main` as the base branch.
- Ensure required checks pass before merge.
- Prefer squash merge to keep `main` history clean.

## Commit Style

Use clear, imperative commit messages, for example:
- `Add runtime-link digest validation`
- `Fix deterministic ordering in strict diagnostics`
