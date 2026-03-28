# Phase 25.0.3 - Policy Decision Lock (`release == proved`)

## Status
Design lock for `25.0.3` in `docs/TODO.md`.

## Goal
Lock the milestone policy decision that production release requires theorem-grade proof completion, while allowing non-proved compilation only for non-release workflows.

## Policy Decision (Locked)
1. Release policy:
   - `release == proved`.
2. Production release eligibility:
   - only artifacts certified `proved_all` are publishable as production releases.
3. Non-proved compilation:
   - allowed for developer workflows (local iteration, debugging, exploration, non-release CI).
4. Non-proved publication:
   - forbidden for production release channels.

## Workflow Matrix (Locked)

| Workflow | Compiler/Profile Mode | Allowed proof status | Publishable as production release |
| --- | --- | --- | --- |
| Local dev build/run | permissive/standard/strict | `proved_all` or `not_proved_all` | No |
| Dev/test CI | standard/strict | `proved_all` or `not_proved_all` | No |
| Release candidate build | strict | `proved_all` only | Yes (candidate) |
| Production publish/tag | strict + release gates | `proved_all` only | Yes |

## Fail-Closed Release Rule
Release/publish operations must fail when proof status is not `proved_all`, including:
1. `failed` VCs,
2. `unknown` VCs,
3. `timeout` VCs,
4. `assumed` boundaries on release-enabled surfaces,
5. missing/ambiguous proof certification inputs.

No policy bypass is permitted in production release commands.

## Non-Release Rule
For non-release workflows:
1. compilation and runtime execution may proceed with `not_proved_all`,
2. artifacts must remain marked as non-release assurance evidence,
3. tooling/docs must not present these outputs as release-grade proved artifacts.

## Scope Boundary
- This slice locks policy only.
- Implementation is tracked separately:
  - `25.0.4` fail-closed enforcement in release flow,
  - `25.0.5` artifact/signature status emission,
  - `25.0.6` verify gate requiring `proved_all`,
  - `25.0.7` release compile profile enforcement.

## Non-Goals
1. No new language syntax/keyword changes.
2. No solver-coverage expansion in this slice.
3. No distribution channel policy beyond production release gating.

## Exit Criteria for 25.0.3
1. Policy decision is documented and locked.
2. `docs/TODO.md` marks `25.0.3` complete with reference to this doc.
3. Release process docs explicitly state that production release requires `proved_all`.

## References
- `docs/TODO.md`
- `docs/design/phase-25.0.1-milestone-3-design-lock.md`
- `docs/design/phase-25.0.2-theorem-grade-certification-policy.md`
- `docs/release-process.md`
