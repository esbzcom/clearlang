# Phase 25.7.1 - Online Testbed Go/No-Go Gate

## Status
Gate contract locked before any online testbed implementation work.

## Decision Inputs (All Required)
Go/no-go is fail-closed. `GO` requires all four inputs below to be present and approved.

1. Threat model
- Documented attack surface for compile/run endpoints (input abuse, resource exhaustion, sandbox escape, artifact tampering, dependency supply-chain abuse).
- Explicit trust boundaries and data-flow for source upload, compile, execute, and result retrieval.
- Required mitigations mapped to each high/critical risk with owner.

2. Abuse controls
- Deterministic per-request limits:
  - wall-clock timeout,
  - CPU quota/fuel,
  - memory cap,
  - max source size,
  - max output size/log size.
- Authentication/rate-limit policy for public/internal callers.
- Fail-closed behavior and stable diagnostics for rejected/terminated runs.

3. Ops budget
- Estimated monthly cost envelope for compute, storage, and observability.
- On-call and maintenance budget allocation.
- Capacity plan for peak testbed usage with explicit SLO target assumptions.

4. Owner assignment
- Product owner: `@felto` (scope and rollout decision).
- Security owner: `@felto` (threat model sign-off until dedicated security owner is assigned).
- Runtime/operations owner: `@felto` (SLO/runbook readiness gate).

## Go/No-Go Rule
- `GO` only if all required inputs above are present, approved, and tracked in milestone_3/phase docs.
- `NO_GO` if any input is missing, unapproved, or has unresolved high/critical risk.
- `NO_GO` outcome triggers `25.7.4` defer publication and keeps CLI-first workflow canonical.

## Deliverables Required for 25.7.2 Start
- Threat-model doc path and approval record.
- Abuse-control policy doc path with fixed limits.
- Ops budget sheet path and approval record.
- Named owner list with explicit ack.

## References
- `docs/TODO.md` (`25.7.1`..`25.7.4`)
- `docs/release-process.md`
