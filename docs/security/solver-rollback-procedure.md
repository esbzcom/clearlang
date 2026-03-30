# Solver Rollback Procedure

Use this runbook when a vendored solver bundle is found broken or vulnerable.

## Steps
1. Stop release tagging (`milestone_3` tags) until rollback validation is complete.
2. Revert solver bundle to last-known-good version/checksum/signature set.
3. Re-run proof gate tests:
   - `solver_outcomes`
   - `solver_replay_stability`
   - `phase25_solver_supply_chain_gate`
   - `milestone3_release_gate`
4. Record incident details and rollback hash/version in release notes.
5. Open follow-up issue for forward upgrade path and CVE remediation.

## Required Artifacts
- Last-known-good solver checksum manifest.
- Detached signature metadata.
- Updated audit note linking incident, rollback commit, and validation evidence.
