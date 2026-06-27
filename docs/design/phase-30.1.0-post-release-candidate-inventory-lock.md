# Phase 30.1.0 - Post-Release Candidate Inventory Lock

Date: 2026-06-27
Status: Locked
Owner: roadmap-owner

## Purpose

Inventory the explicitly deferred additive tracks after Milestone 3 so Milestone 4 work is chosen
from a named candidate set rather than from ad hoc requests.

## Candidate Tracks

### 1. Multi-Platform GA Hardening

Current status:

- Windows and Linux are GA baseline binary targets
- macOS remains preview and non-blocking for the current GA closure
- proof parity gating remains narrower than a full all-platform GA matrix

Why it is a candidate:

- simple for users: reduces ambiguity about where the product is fully supported
- AI-friendly: a locked target matrix and parity policy are easy to reason about mechanically
- provably correct: extends deterministic release/proof expectations across a broader supported
  matrix
- crypto-focused: strengthens release integrity and operational trust on the shipped platforms

### 2. Installer And Publication Channel Expansion

Current status:

- GitHub release artifacts are the canonical publication channel
- Homebrew, apt/rpm, and Windows MSI/winget channels are explicitly deferred

Why it is a candidate:

- simple for users: lowers installation and upgrade friction
- AI-friendly: canonical installer contracts reduce environment-specific setup ambiguity
- provably correct: channel parity can preserve the same evidence and checksum contract
- crypto-focused: installer channels still need deterministic supply-chain and signing guarantees

### 3. Additional Shared-Package Surface Expansion

Current status:

- the supported shared package-id set is locked to eight packages
- broader package-set expansion requires a new design/doc/coverage update

Why it is a candidate:

- simple for users: can reduce embedded-only exceptions if a new package is genuinely common
- AI-friendly: explicit package boundaries keep generated imports and migration paths predictable
- provably correct: bounded package expansion can reuse the existing fail-closed publication model
- crypto-focused: some future package surfaces may matter directly to contract ergonomics

### 4. Richer Chain-Helper Surface Decisions

Current status:

- the supported chain adapters are intentionally bounded to constructor/type-entry surfaces
- broader chain-specific helpers, event surfaces, serializers, and runtime IO remain deferred

Why it is a candidate:

- simple for users: richer chain helpers may reduce application boilerplate
- AI-friendly: only if the surfaces stay narrow and diagnostics remain exact
- provably correct: this is higher risk because broader helper surfaces can hide correctness
  boundaries
- crypto-focused: potentially high-value, but only after the rollout shape is well proven

### 5. Delivery-Mode Simplification

Current status:

- `embedded` remains the default supported production mode
- `shared` is explicit opt-in only
- changing that default is intentionally deferred

Why it is a candidate:

- simple for users: could reduce packaging differences if the shared path becomes routine enough
- AI-friendly: only if migration and activation semantics stay completely explicit
- provably correct: risky unless the current release/runtime evidence path is already routine
- crypto-focused: lower priority than hardening support guarantees and release integrity

## Inventory Decision

The candidate set above is the only authorized Milestone 4 starting pool for now.

Anything outside these tracks requires a new planning lock before it can claim Milestone 4
priority.

## Selection Guidance

The first Milestone 4 slice should prefer:

1. high impact on the supported product contract
2. low risk of weakening the Milestone 3 closure guarantees
3. minimal new language or std surface

That biases the first execution slice toward support-matrix and release/distribution hardening
instead of new package or language surface.

## Next Task

The next task after this inventory lock is `30.2.0`:

- select the first bounded Milestone 4 execution slice from the candidate set above

## References

- `README.md`
- `docs/design/phase-25.1.13-ci-replay-gates-release-target-platforms.md`
- `docs/design/phase-25.6.13-binary-publication-policy-lock.md`
- `docs/design/phase-28.9.2-shared-std-product-contract-lock.md`
- `docs/design/phase-29.8.1-milestone-3-closure-lock.md`
- `docs/todo/milestone_4_roadmap.md`
