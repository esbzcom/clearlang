# Phase 29.12.1 - Multi-Platform Solver Prerequisite Lock

Date: 2026-06-27
Status: Locked
Owner: release-owner

## Purpose

Make the first blocking prerequisite for multi-platform GA hardening explicit.

`29.12.0` locked the target GA matrix as `windows`, `linux`, and `macos`.

However, the repository cannot truthfully enforce full proof parity on that matrix yet, because
the current self-contained trusted solver bundle is still Windows-only.

## Current Blocking State

The current repo state is:

1. `tools/proof/z3/windows/z3.exe` exists with checksum and detached signature metadata
2. there is no parallel trusted bundled solver binary under:
   - `tools/proof/z3/linux/z3`
   - `tools/proof/z3/macos/z3`
3. `docs/design/phase-25.1.15-solver-support-matrix.lock.json` still pins
   `release_target_platforms = ["windows-x64"]`
4. strict proof execution remains fail-closed with `C124` when an acceptable bundled solver is not
   available

That means the originally planned CI-gate expansion cannot be completed honestly until the solver
support contract is expanded first.

## Locked Decision

The execution order inside `29.12` is revised as follows:

1. `29.12.2` expands the self-contained solver support matrix and trusted vendor-bundle coverage to
   the intended GA platforms
2. `29.12.3` extends CI, release-train, and binary evidence gates only after `29.12.2` is
   complete
3. `29.12.4` updates operator and release documentation after the behavior contract changes land

This is a prerequisite lock, not a retreat from the `29.12.0` target state.

## Required Output Of `29.12.2`

`29.12.2` succeeds only when all of the following are true:

1. the solver support-matrix lock is updated from Windows-only to the intended multi-platform set
2. trusted bundled solver artifacts, checksums, and detached signatures exist for every intended GA
   platform
3. the pinned solver supply-chain policy remains valid for the expanded bundle set
4. strict proof execution on each intended GA platform can resolve a trusted bundled solver without
   falling back to unsupported behavior

## Why This Revision Is Correct

### 1. It Preserves Honesty In The Roadmap

The project should not mark cross-platform proof parity as executable when the trusted solver input
is still missing on two of the three intended GA platforms.

### 2. It Protects The Current Correctness Contract

The existing `C124` fail-closed behavior is correct.

The fix is to expand trusted solver availability, not to weaken the proof pipeline or make CI
pretend unsupported platforms are release-grade.

### 3. It Still Matches The README Principles

- simple for users: support claims stay exact
- AI-friendly: the platform contract remains explicit and machine-checkable
- provably correct: fail-closed behavior remains intact until prerequisites are real
- crypto-focused: trusted supply-chain inputs stay mandatory

## Non-Goals

This prerequisite lock does not authorize:

1. bypassing detached signature or checksum verification for new solver bundles
2. marking `linux` or `macos` proof execution as GA by documentation alone
3. broadening installer channels or std/package surfaces in the same slice

## Next Task

The next task after this prerequisite lock is the implementation body of `29.12.2`:

- expand the trusted self-contained solver bundle set and update the solver support matrix

## References

- `docs/design/phase-25.1.15-solver-support-matrix.md`
- `docs/design/phase-25.1.16-solver-supply-chain-security-gates.md`
- `docs/design/phase-29.12.0-multi-platform-ga-support-lock.md`
- `crates/cli/src/commands/build/solver/discovery.rs`
- `tools/proof/z3/windows/z3.exe`
- `docs/todo/milestone_3_roadmap.md`
