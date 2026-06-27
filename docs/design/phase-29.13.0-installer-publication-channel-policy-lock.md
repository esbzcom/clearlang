# Phase 29.13.0 - Installer/Publication-Channel Policy Lock

Date: 2026-06-27
Status: Locked
Owner: release-owner

## Purpose

Lock the first supported post-GitHub-release installer/publication channels before implementation
work lands.

`29.12` closes the multi-platform GA baseline across `windows`, `linux`, and `macos`.

The next remaining Milestone 3 distribution step is to reduce install friction without creating new
binary trust roots, platform-specific release drift, or opaque installer behavior.

## Decision

The first bounded post-GitHub-release channels are:

1. Homebrew formula publication for `macos` and `linux`
2. Winget portable-manifest publication for `windows`

GitHub release artifacts remain the canonical release payloads and evidence source.

These new channels are metadata/discovery layers over the existing signed GitHub binary bundles,
not replacement build outputs and not independent trust roots.

## Why This Channel Set Is First

### 1. It Maximizes User Simplicity Without Fragmenting Linux Packaging

This choice gives one package-manager path for:

- `macos`
- `linux`
- `windows`

without immediately taking on separate:

- `deb`
- `rpm`
- `msi`

publication pipelines.

That keeps installation simpler for users while keeping the implementation surface bounded.

### 2. It Preserves AI-Friendly Determinism

Homebrew formulas and winget manifests can point at exact versioned GitHub release URLs and pinned
SHA-256 digests.

That is easier for tools to generate, audit, and repair safely than bespoke shell installers or
per-distribution packaging scripts.

### 3. It Keeps Correctness Bound To Existing Release Evidence

The shipped executable, checksum sidecars, signed release-bundle evidence, provenance, and proof
parity outputs continue to be produced once by the canonical release flow.

The installer channels must consume those exact locked outputs rather than rebuilding `clg`,
rewriting checksums, or inventing channel-only artifacts.

### 4. It Matches The Crypto-Focused Product Priority

For ClearLang, installer convenience is allowed only if:

1. the trust boundary stays explicit
2. checksums remain pinned
3. provenance-bearing release artifacts remain authoritative
4. channel metadata cannot silently weaken rollback or tamper detection

## Locked Publication Policy

### Canonical Release Source

GitHub release artifacts remain canonical for Milestone 3.

Every supported installer/publication channel must resolve to the exact versioned GitHub release
artifacts already produced by the GA release train.

Channel support does not authorize:

- alternate artifact hosts
- channel-specific rebuilds
- source-build installation as the supported production path

### First Supported Channels

The bounded supported channels for `29.13` are:

- Homebrew tap formula for `macos` and `linux`
- Winget portable manifest for `windows`

No other installer or package-manager channel becomes supported in this slice.

## Required Parity Contract

When `29.13` is complete, every supported channel must preserve all of the following:

1. exact version parity with the canonical GitHub release tag
2. exact asset-URL parity with the released platform binary bundle
3. exact SHA-256 parity with the published checksum sidecar
4. release-notes linkage to the same Milestone 3 release notes
5. install/upgrade/uninstall smoke coverage for the channel-managed path
6. no silent platform exclusions inside the locked GA matrix

Channel manifests or formulas must be generated from authoritative release metadata rather than
hand-maintained per-platform copies with drift-prone values.

If channel metadata disagrees with the canonical release bundle, the release is incomplete.

## Signing And Trust Expectations

### Canonical Signing Authority Stays Unchanged

The trust root remains the existing Milestone 3 release-signing and provenance chain:

1. signed binary bundle metadata
2. checksum sidecars
3. signed release-bundle evidence
4. provenance artifacts

Homebrew and winget metadata are distribution descriptors, not new trust anchors.

### Allowed Channel Metadata Shape

Supported channel metadata must:

- reference the canonical GitHub release asset URL
- embed the canonical SHA-256 digest for that asset
- keep version metadata identical to the released tag
- remain human-reviewable text checked into the repository or a repository-controlled publication
  path

### Disallowed Trust Downgrades

This slice does not authorize:

1. unsigned opaque installer scripts as the supported path
2. channel-specific binaries with no matching GitHub release artifact
3. checksum-free package manager publication
4. channel-managed source compilation as the production default
5. replacing provenance verification with channel reputation or package-manager trust alone

## Success Criteria For `29.13`

`29.13` is complete only when:

1. Homebrew and winget publication metadata can be produced deterministically from the release
   outputs
2. the resulting channel definitions preserve version/URL/checksum parity with GitHub release
   artifacts
3. CI or release-train coverage proves the channel files are present and structurally valid
4. installation and operations docs name the exact supported channels and keep unsupported channels
   explicit

## Non-Goals

This lock intentionally defers:

1. `deb` / `apt` repository publication
2. `rpm` repository publication
3. Windows `msi` packaging
4. `curl | sh` or `Invoke-WebRequest | iex` style bootstrap installers
5. changing the canonical release host away from GitHub
6. expanding the GA platform matrix beyond `windows`, `linux`, and `macos`

Those require separate follow-up locks if they are ever prioritized.

## Next Task

The next task after this lock is `29.13.1`:

- implement deterministic Homebrew and winget publication outputs under the policy above, without
  weakening existing provenance, checksum, or fail-closed release guarantees

## References

- `README.md`
- `docs/TODO.md`
- `docs/design/phase-25.6.13-binary-publication-policy-lock.md`
- `docs/design/phase-29.10.0-remaining-production-candidate-inventory-lock.md`
- `docs/design/phase-29.12.0-multi-platform-ga-support-lock.md`
- `docs/release/milestone_3-binary-operations.md`
- `docs/release-process.md`
- `docs/todo/milestone_3_roadmap.md`
