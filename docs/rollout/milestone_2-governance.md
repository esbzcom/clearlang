# Milestone 2 Governance Controls

## Scope
Execution governance for Phase 20-24 delivery toward `milestone_2`.

## 24.3.1 DRI Ownership
| Parent task | DRI | Backup owner | Status |
|---|---|---|---|
| Phase 20 (runnable namespace + strict gates) | `@felto` | `@felto` | complete |
| Phase 21 (precompiled std-core packaging) | `@felto` | `@felto` | complete |
| Phase 22 (package trust + dependency resolution) | `@felto` | `@felto` | complete |
| Phase 23 (runtime loader + linker) | `@felto` | `@felto` | complete |
| Phase 24 (host profiles + milestone exit) | `@felto` | `@felto` | in_progress |

## 24.3.2 Target Dates + Critical Path
| Parent task | Planned start | Planned end | Critical-path dependencies |
|---|---|---|---|
| Phase 20 | 2026-01-06 | 2026-02-02 | none |
| Phase 21 | 2026-02-03 | 2026-02-17 | Phase 20 |
| Phase 22 | 2026-02-18 | 2026-03-07 | Phase 21 |
| Phase 23 | 2026-03-08 | 2026-03-18 | Phase 22 |
| Phase 24 | 2026-03-19 | 2026-04-05 | Phase 23 |

Critical path lock:
`20 -> 21 -> 22 -> 23 -> 24 -> milestone_2 tag`

## 24.3.3 Risk Register (Weekly Review)
Review cadence: weekly (every Friday).
Last reviewed: 2026-03-21.

| Risk | Impact | Mitigation | Rollback owner | Status |
|---|---|---|---|---|
| Release readiness evidence (`24.2.8`) incomplete at tag time | high | block tag until security/runbook/signing evidence links are attached in release notes | `@felto` | open |
| Performance budget evidence (`24.2.9`) not measured or over threshold | high | require measurement artifact and threshold pass/fail report before release | `@felto` | open |
| Supply-chain compliance evidence (`24.2.10`) missing | high | require SBOM/license gate artifacts before release sign-off | `@felto` | open |
| Release-train bypass by tagging before go-live checklist is complete | high | enforce tag-time release-train gate (`milestone2_release_train_gate`) | `@felto` | mitigated |

## 24.3.4 Release-Train Gate Policy
- `milestone_2` tag attempts must run an explicit CI gate (`milestone2-release-train-gate`).
- That gate enforces:
  - all `24.2.x` checklist items in `docs/TODO.md` are checked (`[x]`),
  - `release_notes/milestone_2.md` exists.
- If either condition fails, the tag pipeline fails closed.

## Post-Review Hardening Decisions (Option 1)
Decision date: 2026-03-21.

1. Supply-chain license gate has no workspace exemption:
   - workspace crates must carry explicit `license` or `license_file` metadata.
2. Supply-chain gate runs after std-core artifact generation:
   - generated package metadata/artifacts are included in compliance scope.
3. Runtime host-profile capability validation is fail-closed:
   - unsupported/duplicate/empty capability ids are rejected with deterministic `R016`.
4. Performance gate includes runtime-link startup KPI:
   - runtime loader/linker path is measured directly, not only build-time package import resolution.
