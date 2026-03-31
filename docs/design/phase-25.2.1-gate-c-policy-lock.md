# Phase 25.2.1 - Gate C Policy Lock (Discussion Conclusions)

## Status
Policy lock for `25.2.1` in `docs/TODO.md`.

## Goal
Record Gate C discussion conclusions as binding policy inputs for implementation tasks `25.2.2+`.

## Locked Conclusions
1. **Simplified command options with proved-only release assumptions**
   - Primary command surface is simplified for end users and AI tools.
   - Release policy remains `release == proved` with theorem-grade requirement (`proved_all`) for release artifacts.
2. **One-command release path**
   - `clg release` is the canonical release entrypoint for production artifacts.
   - Release flow is deterministic and fail-closed (`lock -> build/prove -> sign -> verify -> bundle`).
3. **Built-in Z3 migration path**
   - Solver path must support migration from external CLI invocation to built-in Rust-managed integration (`rust-z3-lib`) with deterministic parity gates before cutover.
   - Release policy cannot weaken during migration.
4. **IDE/VSCode-first CLI contracts**
   - Primary commands must support stable machine-readable outputs, deterministic exit-code mapping, and non-interactive plugin-safe behavior.
   - Contract stability is treated as a compatibility surface, not a best-effort feature.

## Transitional Policy for Non-Strict Modes
- `permissive`/`standard` compiler modes are explicitly **transitional** for migration, diagnostics, and development.
- They are **not** accepted for production release artifacts.
- Production release path remains strict and theorem-grade only (`release == proved`, `proved_all` required).
- Pre-production policy: backward-compatibility is **not** a goal; remove legacy/compatibility surfaces as soon as replacement strict-first flows are available.
- Gate C cutover tasks:
  - `25.2.6`: simplify and remove legacy flag-heavy release path; emit migration guidance without compatibility alias requirements.
  - `25.2.7`: provide `clg check` as strict-aligned deterministic preflight for local iteration.
- Post-cutover intent: strict-first defaults for user-facing primary workflows.

## Non-Goals
- This lock does not itself implement IDE event schema, solver backend abstraction, or release prechecks; those are covered by later `25.2.x` tasks.

## References
- `docs/TODO.md`
- `docs/design/phase-25.2.2-release-ux-design-lock.md`
- `docs/design/phase-25.2.3-release-command-orchestration.md`
- `docs/design/phase-25.2.4-release-proved-only-default.md`
