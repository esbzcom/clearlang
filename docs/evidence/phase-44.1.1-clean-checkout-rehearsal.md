# Phase 44.1.1 - Clean Checkout Rehearsal

Date: 2026-10-16
Status: Passed locally
Commit: `3135e56ba811d37c0149fdf554b52c11be6b76e7`

## Scope

This is the clean-workspace prerequisite for the Milestone 5 closure rehearsal. It demonstrates
that the checked-in source, version-controlled `Cargo.lock`, pinned Rust toolchain, documented
local Cargo cache prerequisite, and public reference fixtures suffice to build the verifier and
exercise the reference registry. It is local/offline evidence only; it does not certify a live
target action, mainnet use, custody, or generic EVM support.

## Clean Workspace

An isolated clone was created from the local approved repository with `git clone --no-local` at
the commit above. Before any build, both `git status --porcelain` and
`git ls-files --others --exclude-standard` were empty, and the clone contained the tracked
`Cargo.lock` with SHA-256:

```text
b8160e2eb024ae88c63f55c1f3857ccf2a825eafbebc7e6fcc8e8d07c5cf6c29
```

The clone used the checked-in `rust-toolchain.toml`: Cargo `1.89.0` and Rust `1.89.0` on
`x86_64-pc-windows-msvc`. No source worktree, target directory, local project configuration,
endpoint, wallet, private key, or remote CI result was copied into it.

## Offline Results

With `CARGO_NET_OFFLINE=true` and `--locked`:

1. `cargo build --offline --locked -p clg-cli --bin clg-contract-verifier` passed.
2. `cargo test --offline --locked -p clg-cli --test cli_it
   basic::reference_configuration_registry_migrates_releases_and_binds_target_receipts` passed.

The reference fixture builds and verifies its local release/migration/receipt chain with only
repository fixtures and test-owned ephemeral signing material. It does not use operator
credentials or contact a network. After the commands, the clone still had no tracked or
untracked source changes; build outputs remained under ignored `target/`.

## Boundary

This completes 44.1.1 only. The canonical full pilot-precheck report, independent distributed
verifier run, authorization/tamper matrix, offline replay, and second clean-directory comparison
remain required under 44.1.2 and 44.1.3 before Pilot Closure can be claimed.
