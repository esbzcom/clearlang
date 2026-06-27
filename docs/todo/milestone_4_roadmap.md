# Milestone 4 - Post-Release Hardening and Expansion (30+)

Milestone 4 starts only after the Milestone 3 production contract is closed and release-ready.

Execution order for Milestone 4: **planning lock -> candidate inventory -> first-slice selection -> platform hardening -> channel expansion -> next surface selection**.

- [x] 30.0 Milestone 4 planning lock [Planning Gate A] `Completed: 2026-06-27`
  - [x] 30.0.0 Publish the Milestone 4 planning lock, defining scope, non-goals, release invariants, and success criteria for post-release work so future changes do not reopen Milestone 3 by accident. (`docs/design/phase-30.0.0-milestone-4-planning-lock.md`) `Completed: 2026-06-27`

- [x] 30.1 Post-release candidate inventory [Planning Gate B] `Completed: 2026-06-27`
  - [x] 30.1.0 Inventory the explicitly deferred post-release tracks and classify them against the README design principles so future work is selected from an exact candidate set rather than roadmap drift. (`docs/design/phase-30.1.0-post-release-candidate-inventory-lock.md`) `Completed: 2026-06-27`

- [x] 30.2 First Milestone 4 slice selection [Planning Gate C] `Completed: 2026-06-27`
  - [x] 30.2.0 Select the first bounded Milestone 4 execution slice, define its success target, and name the remaining deferred tracks explicitly. (`docs/design/phase-30.2.0-first-milestone-4-slice-selection.md`) `Completed: 2026-06-27`

- [ ] 30.3 Multi-platform GA hardening [Execution Gate A]
  - [ ] 30.3.0 Publish the platform-support and GA promotion lock covering Windows/Linux/macOS status, deterministic proof/release parity scope, and release-blocking criteria before implementation changes land.
  - [ ] 30.3.1 Extend CI, release-train, and binary evidence gates so the locked GA target matrix is enforced under one deterministic proof/release parity contract.
  - [ ] 30.3.2 Update binary operations, release-train, release-process, and incident-response documentation after the platform contract changes land so operator guidance stays exact.

- [ ] 30.4 Installer channel expansion [Execution Gate B]
  - [ ] 30.4.0 Publish the installer/publication-channel policy lock for the first supported post-GitHub-release channels, including parity requirements, signing expectations, and non-goals.
  - [ ] 30.4.1 Implement the first bounded installer/distribution channels under the locked publication policy without weakening provenance or checksum requirements.
  - [ ] 30.4.2 Update installation and operations documentation only after the new channels are shipped and covered.

- [ ] 30.5 Next additive product-surface selection [Planning Gate D]
  - [ ] 30.5.0 Re-rank the remaining additive product tracks after `30.3` and `30.4`, then select the next bounded surface decision from: shared-package expansion, richer chain-helper surfaces, or delivery-mode simplification.

