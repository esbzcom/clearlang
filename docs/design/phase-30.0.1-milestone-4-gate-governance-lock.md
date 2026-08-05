# Phase 30.0.1 - Milestone 4 Gate Governance Lock

Date: 2026-08-05
Status: Locked
Owner: product-and-runtime-owner

## Purpose

Define accountable owners, objective acceptance evidence, and hard dependencies for the Milestone
4 parent gates. This prevents a feature from being called product-ready merely because a local
implementation exists.

Role names are ownership boundaries, not individual assignments. A release cannot proceed while
any required role is unassigned.

## Design Principles Check

- Simple for users: each gate has one accountable owner and one evidence set, avoiding ambiguous
  release decisions.
- AI-friendly: ownership, dependencies, artifact names, and pass/fail conditions are explicit
  structured inputs for automation.
- Provably correct: proof, target, and release claims are separately evidenced and cannot be
  inferred from a simulator-only result.
- Crypto-focused: crypto and external-call boundaries require dedicated security review and
  fail-closed evidence before release.

## Parent-Gate Accountability

| Gate | Accountable owner | Required acceptance evidence | Cannot complete before |
|---|---|---|---|
| 30.0 Product/architecture | product-and-runtime-owner | Locked product boundary; target decision; named roles; dependency graph | Milestone 3 production gates remain green |
| 30.1 Stateful semantics | language-and-proof-owner | Syntax/typing/VC design lock; deterministic storage-layout fixtures; snapshot/invariant proof artifacts; migration-negative tests | 30.0.0 |
| 30.2 External-call safety | language-security-owner | External-call design lock; ordering/reentrancy negative fixtures; strict diagnostics; adversarial-release evidence | 30.1.0 |
| 30.3 EVM target/simulator | target-runtime-owner | Target-profile lock; ABI/schema fixtures; simulator traces; deploy/call conformance results; compatibility matrix | 30.1.0 and 30.2.0 |
| 30.4 Crypto assurance | crypto-and-proof-owner | Primitive/threat-model lock; proof or versioned attestation artifacts; malformed/replay test corpus; security sign-off | 30.3.0 |
| 30.5 DX/runtime operations | developer-experience-owner | Memory-limit evidence; deterministic fuzz replay artifacts; editor integration contract tests; operator runbooks | 30.1.0 and 30.3.0 |
| 30.6 Release closure | release-assurance-owner | Signed contract bundle; independent verification result; CI matrix; tamper/replay/reentrancy-negative evidence; final support policy | 30.1 through 30.5 complete |

## Dependency Graph

```text
30.0.0 product lock
  └─ 30.1 state design ─┬─ 30.2 external-call safety ─┐
                        ├─ 30.3 EVM target/simulator ──┼─ 30.4 crypto assurance
                        └─ 30.5 DX/runtime operations ──┤
                                                         └─ 30.6 release closure
```

Implementation within `30.1` may begin after its own design lock. `30.2`, `30.3`, and `30.5`
may execute in parallel only after their listed design dependencies are locked. `30.6` is strictly
last: partial evidence from any predecessor is not sufficient.

## Evidence Contract

Each parent gate must contribute an entry to `docs/evidence/milestone_4-contract-platform.md`
and a machine-readable lock or manifest under `docs/evidence/` before its checklist item can be
checked complete. The evidence entry must contain:

1. owner and reviewer roles;
2. exact toolchain, target-profile, and schema versions;
3. deterministic commands and expected artifact hashes or stable result identifiers;
4. happy-path and required negative-path results;
5. known exclusions and any remaining assumption boundaries;
6. a statement of whether the gate is release-blocking and which diagnostic fails it closed.

Evidence may reference generated artifacts outside the repository, but their digest, canonical
format version, retention location, and verification command must be recorded. A link without
verifiable identity is not acceptance evidence.

## Review and Escalation Rules

1. The accountable owner cannot self-approve a change that alters the gate's proof, crypto, or
   security boundary; the corresponding proof or security owner must review it.
2. Any target compatibility expansion, new crypto primitive, state-schema migration rule, or
   external-call exception requires a new design lock before implementation.
3. A failed reentrancy, state-invariant, ABI-conformance, crypto-boundary, or bundle-verification
   test blocks `30.6` until the failure is fixed and recorded in the evidence index.
4. Release-assurance-owner has final authority to reject incomplete or internally inconsistent
   evidence, but cannot waive a fail-closed condition.

## Exit Criteria for 30.0.1

1. Every Milestone 4 parent gate has one accountable owner role.
2. Every parent gate has required acceptance evidence and hard predecessors.
3. The final release gate has a complete dependency path to state, safety, target, crypto, and
   operations evidence.
4. The Milestone 4 roadmap links this lock and marks `30.0.1` complete.

## References

- `docs/todo/milestone_4_roadmap.md`
- `docs/design/phase-30.0.0-milestone-4-production-contract-platform-lock.md`
- `docs/evidence/milestone_3-proof-gate.md`
