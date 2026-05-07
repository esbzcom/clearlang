# ClearLang Release Process (Current CLI + Gate C Orchestration)

This is the current production-style flow using existing commands.
Primary user path is `clg strict init` + `clg release`; manual `build`/`verify` release gating is expert/debug-only.

## Scope
- Strict preflight and deterministic lock input
- Strict build with VC emission
- Signing + verification gates
- Publishable release artifacts

Phase 25.6 policy lock:
- GA artifact scope is distributable `clg` binaries (release bundles remain assurance evidence).
- GA baseline targets are Windows + Linux; macOS is preview/non-blocking in this phase.
- Provenance attestation is required for GA release-train artifacts.

## Primary UX Surface (Gate C Lock)
Shipped primary commands:
- `clg check`
- `clg test` (shipped in `25.3.2`)
- `clg release`
- `clg verify-bundle`

Gate C history: `clg test` was tracked as a reserved contract before `25.3.2`.
Gate D lock: `clg test` keeps one deterministic default unit-test proof policy, and non-essential test flags remain deferred to `tests/test-plan.json` policy controls.

Local release-precheck gate (required before strict signed publish flow):

```powershell
cargo run -p xtask -- release-precheck
```

Deterministic `.clear` source formatting check:

```powershell
clg fmt examples/projects/generic --check
```

Deterministic `.clear` lint gate:

```powershell
clg lint examples/projects/generic --deny-warnings
```

Bootstrap strict preflight/release defaults once per project root:

```powershell
clg strict init examples/projects/generic
```

Fast local preflight (non-release):

```powershell
clg check examples/projects/generic/main.clear --root examples/projects/generic
```

Concrete `clg release` CLI shape:

```powershell
clg release --key keys/signing.json --pubkey keys/public.json --root examples/projects/generic
```

Deterministic pre-signing readiness gate:

```powershell
clg release --check-only --root examples/projects/generic
```

`clg release` resolves required non-secret defaults from `clg.project.json`:
- `release_defaults.advisory_as_of`
- `release_defaults.key_id`
- `release_defaults.out_dir`
- `release_defaults.trust_policy`
`clg.project.json` schema v1 is also the user-authored project/dependency manifest for lock generation.

Minimal `clg.project.json` syntax sample used by `clg release`/`clg pkg lock`:

```json
{
  "schema_version": 1,
  "project": {
    "name": "example-app",
    "description": "Example project",
    "version": "0.1.0",
    "clg_version": "^0.1.0",
    "entry": "main.clear",
    "website": "https://example.com",
    "contact": {
      "name": "Maintainer",
      "email": "maintainer@example.com"
    }
  },
  "dependencies": [
    { "name": "std::core", "requirement": "^1.0.0" }
  ],
  "release_defaults": {
    "advisory_as_of": "2026-03-26T00:00:00Z",
    "key_id": "release-2026q2",
    "out_dir": "out/release",
    "trust_policy": "trust-policy.json"
  }
}
```

`project.clg_version` must be a valid semver requirement and must match the running `clg` version (fail-closed).

`clg release` now executes one-command orchestration (`lock -> build/prove -> sign -> verify -> bundle`) and writes a deterministic release bundle manifest JSON (`<stem>.release-bundle.json`) containing artifact hashes and stage status.

`clg verify-bundle` verifies the bundle manifest/artifact hashes and then runs compile-time signature/assurance verification from manifest wiring (no manual `--module/--sig/--assurance-manifest` flag set required):

```powershell
clg verify-bundle --bundle out/generic.release-bundle.json --pubkey keys/public.json
```

Rotated-key workflow (recommended for historical verification):

```powershell
clg verify-bundle --bundle out/generic.release-bundle.json --keyring keys/release-keyring.json
```

`--keyring` resolves the signature `key_id` to a public-key file path and fails closed if the key is unknown or revoked.

GA release-train provenance requirement:

```powershell
clg verify-bundle --bundle out/generic.release-bundle.json --keyring keys/release-keyring.json --require-provenance
```

## VSCode/IDE Plugin Profile (25.2.10)
Use the plugin-safe profile for primary commands:

```powershell
clg --non-interactive --json-errors --json-events check examples/projects/generic/main.clear --root examples/projects/generic
clg --non-interactive --json-errors --json-events test examples/projects/testing --report json
clg --non-interactive --json-errors --json-events release --key keys/signing.json --pubkey keys/public.json --root examples/projects/generic
```

Contract summary:
- `--json-errors` emits failure payload JSON to `stdout`.
- `--json-events` emits NDJSON events to `stderr` (`schema_version: 1`).
- Exit codes are pinned: `0` success, `1` deterministic command failure, `2` usage/argument failure.
- Timeout/cancellation is managed by the IDE runner (process termination on cancel/timeout).

Full profile and schema/versioning details:
- `docs/ide/vscode-cli-profile.md`

## Prerequisites
Project module root must include:
- `clg.project.json` (schema v1 project metadata + dependencies + release defaults; generated by `clg strict init`)
- `clg.package-metadata.json`
- `clg.package-abi.json`
- `clg.trust-policy.json`
- `clg.host-profile.json`
- `trust-policy.json` (schema v1 trust-anchor policy for compile-time verify)

Windows-first self-contained solver setup (no system install):

```powershell
cargo run -p xtask -- solver-vendor-stage --from C:\path\to\z3.exe --platform windows
```

Set publisher signing key env var before staging:

```powershell
$env:CLG_SOLVER_VENDOR_SIGNING_KEY_HEX = "<32-byte-ed25519-private-key-hex>"
```

This command stages `z3(.exe)` and emits required integrity/authenticity sidecars (`.sha256`, `.sig`).
If you use `CLG_SOLVER_BIN` to override solver location, that binary must also include
matching `.sha256` and `.sig` sidecars.
`.sig` sidecars are verified as cryptographic Ed25519 signatures against pinned vendor keys and rotation policy (`docs/design/phase-25.1.16-solver-supply-chain.lock.json`).

Solver backend selection policy:
- default backend is `external-z3-cli`
- optional override: `CLG_SOLVER_BACKEND=external-z3-cli|rust-z3-lib`
- unsupported backend values fail closed
- selecting `rust-z3-lib` requires a build with Cargo feature `rust-z3-lib` enabled
  - current build path statically links Z3 and requires `cmake` in the build environment
  - Windows builds also require a non-isolated Python runtime (for Z3 script generation imports):
    - `python -c "import sys; print(sys.flags.isolated)"` should print `0`
    - if it prints `1` due `python312._pth` isolation, disable that mode (for example by renaming/removing the `python312._pth` file) before compiling `rust-z3-lib`
- migration cutover flag: `CLG_SOLVER_RUST_Z3_CUTOVER=1|0` (or `true|false|on|off|yes|no`)
  - `CLG_SOLVER_BACKEND=rust-z3-lib` + cutover false/unset -> deterministic fallback to `external-z3-cli`
  - `CLG_SOLVER_BACKEND=rust-z3-lib` + cutover true -> use in-process `rust-z3-lib` backend
- parity gate for migration safety:
  - `cargo test -p clg-cli --features rust-z3-lib --test solver_backend_parity`
- cutover packaging gate (no runtime dependency on `tools/proof/z3` when cutover is enabled):
  - `cargo test -p clg-cli --features rust-z3-lib --test solver_rust_cutover_packaging`

## 1) Initialize Strict Preflight Inputs

```powershell
clg strict init examples/projects/generic
```

This command generates missing strict preflight inputs, validates strict schemas, and writes `clg.project.json` schema v1 with:
- project metadata (`name`, `description`, `version`, `clg_version`, `entry`, `website`, `contact`)
- dependency requirements (`dependencies[]`)
- release defaults placeholders (`REQUIRED_RFC3339_UTC`, `REQUIRED_KEY_ID`) that must be replaced before production release.

`clg release`/`clg pkg lock` also fail closed if `project.clg_version` does not match the running `clg` binary version.

## 2) Generate Deterministic Lock Inputs
Use an explicit advisory time for replay-stable policy evaluation.

```powershell
clg pkg lock --generate --compiler-mode strict --advisory-as-of 2026-03-26T00:00:00Z --root examples/projects/generic
```

Outputs:
- `clg.lock.json`
- `clg.resolved-graph.json`
- `clg.resolved-graph.sha256`

`clg release` performs this automatically (generate/update chosen by lockfile presence).
When `clg.project.json` uses schema v1, lock roots are read from `dependencies[]`; package candidates are sourced from `clg.package-metadata.json`.

## 3) Advanced Expert Flow: Strict Build + VC Emission
Build Wasm and emit verification conditions.

```powershell
clg build examples/projects/generic/main.clear -o out/generic.wasm --emit-vcs out/generic.vc.json --compiler-mode strict --release-profile production --validate
```

Optional (recommended for Gate B workflows):

```powershell
clg build examples/projects/generic/main.clear -o out/generic.wasm --emit-vcs out/generic.vc.json --emit-proof out/generic.proof.json --compiler-mode strict --release-profile production --validate
```

## 4) Advanced Expert Flow: Sign Release Artifacts
Sign module and proofs in one pass and emit assurance manifest.

```powershell
clg build examples/projects/generic/main.clear -o out/generic.wasm --emit-vcs out/generic.vc.json --emit-proof out/generic.proof.json --compiler-mode strict --release-profile production --validate --sign --key keys/signing.json --key-id release-2026q1 --scope both --sig-out out/generic.sig.json --assurance-manifest-out out/generic.assurance.json
```

## 5) Advanced Expert Flow: Verify Before Publish
Run verification gate against the produced artifacts.

```powershell
clg verify --module out/generic.wasm --sig out/generic.sig.json --pubkey keys/public.json --verify-mode compile-time --trust-policy examples/projects/generic/trust-policy.json --assurance-manifest out/generic.assurance.json --explain
```

If proof-artifact claims are present, include:

```powershell
clg verify --module out/generic.wasm --sig out/generic.sig.json --pubkey keys/public.json --assurance-manifest out/generic.assurance.json --proof-artifact out/generic.proof.json
```

Theorem-grade gate for release workflows:

```powershell
clg verify --module out/generic.wasm --sig out/generic.sig.json --pubkey keys/public.json --verify-mode compile-time --trust-policy examples/projects/generic/trust-policy.json --assurance-manifest out/generic.assurance.json --require-assurance proved_all
```

Note: example project sources may intentionally exercise non-proved std APIs; adapt `project.entry`/sources to production-allowed surfaces before expecting `clg release` to pass.

Optional release policy gate:

```powershell
clg verify --module out/generic.wasm --sig out/generic.sig.json --pubkey keys/public.json --assurance-manifest out/generic.assurance.json --release-policy release-policy.json
```

## 6) Publish Bundle
Publish these files together:
- `out/generic.wasm`
- `out/generic.strict-import-map.json`
- `out/generic.vc.json`
- `out/generic.sig.json`
- `out/generic.assurance.json`
- `out/generic.provenance.json`
- `out/generic.proof.json` (when `--emit-proof` was used)
- `out/generic.release-bundle.json`
- checksums/SBOM/release notes as needed by your distribution process

For milestone_3 binary distribution artifacts (Windows/Linux GA baseline), emit the signed binary bundle:

```powershell
$env:CLG_BINARY_RELEASE_SIGNING_KEY_HEX = "<32-byte-ed25519-private-key-hex>"
cargo run -p xtask -- milestone3-binary-bundle --platform <windows|linux> --out-dir tmp/milestone3-binary/<platform>
```

Bundle outputs include the platform binary, checksum manifest, signed metadata, and SBOM/license evidence.

## Notes
- Milestone 3 policy lock: production release is `release == proved` (`proved_all` required). Non-proved outputs are dev/non-release only.
- `--compiler-mode permissive|standard` are transitional dev/evidence workflows and are not accepted for production release artifacts.
- `--release-profile production` enforces fail-closed theorem-grade gating at build/sign time.
- `--release-profile production` also enforces release-surface policy (`C122`): used `std::...` symbols must be listed as `proved` in `docs/proofs/proof-coverage-matrix.json`.
- `--release-profile production` enforces crypto proof-boundary policy (`C123`): any remaining `crypto.uninterpreted` boundary blocks release-grade/theorem-grade claims.
- `--release-profile production` enforces test isolation policy (`C128`): module graph must not reference `tests/` or `tests/mocks/`.
- `clg release` enforces release artifact scan policy (`C129`): strict import-map/release-bundle evidence must not include test/mock source paths.
- Fail-closed release gate: block production release on any proof outcome `failed|unknown|timeout|assumed`.
- `--require-assurance proved_all` now also requires zero assumption boundaries in signed payloads/manifests (`unsigned.int_model`, `bitwise.uninterpreted`, `crypto.uninterpreted` are prohibited in release bundles).
- Signed assurance payloads include deterministic `proof_status` (`proved_all|not_proved_all`) for release-policy tooling.
- Milestone 3 CI/tag gate is locked by `docs/evidence/milestone_3-proof-gate.lock.json` and enforced by `crates/cli/tests/milestone3_release_gate.rs`.
- Milestone 3 release-target parity gate currently runs on Windows (`windows-latest`) and enforces deterministic proof-parity artifact emission before `milestone_3` tag release gating.
  - `cargo test -p clg-cli --features rust-z3-lib --test solver_backend_parity`
  - `cargo test -p clg-cli --features rust-z3-lib --test solver_rust_cutover_packaging`
- Solver supply-chain gate is locked by `docs/design/phase-25.1.16-solver-supply-chain.lock.json` (pinned version + checksum/signature + legal notices + CVE/rollback policy).
- `--emit-proof` emits deterministic proof artifact summaries and binds optional proof/solver hashes into signed payload/manifest claims for verify-time consistency checks.
- `clg release` is implemented for one-command orchestration (`25.2.3`) and fails closed on any stage error.
- `clg release` is proved-only by default (`25.2.4`): there is no downgrade flag path for non-`proved_all` release artifacts.
- `clg strict init <root>` (`25.2.5`) generates strict preflight templates and `clg.project.json` (schema v1 project/dependency metadata + release defaults) for reduced `clg release` flag surface.
- Legacy release-like `clg build`/`clg verify` flows emit migration guidance to `clg release` (`25.2.6`) and are documented as expert/debug-only.
- Pre-production roadmap policy: compatibility debt is not preserved; legacy release paths should be removed once strict-first replacements are in place.
- Gate E cutover policy (`25.4.7`): pre-GA legacy manifest/release inputs are removed with fail-closed behavior and no backward-compatibility commitment before GA.
- Release precheck gate (`25.2.12`) is fail-closed for local+CI release workflows: `cargo run -p xtask -- release-precheck` (`fmt --check` + lint + tests + manifest/lock drift gate + `clg test` schema gate) must pass before strict signed publish flow.
- Milestone 3 binary operations/checklist/runbook:
  - `docs/release/milestone_3-binary-operations.md`
  - `docs/release/milestone_3-release-train-checklist.md`
  - `docs/release/milestone_3-binary-incident-runbook.md`
- Binary reproducibility witness gate (`25.6.12`) is wired through:
  - `cargo run -p xtask -- binary-repro-witness --out <FILE>`
  - `.github/workflows/ci.yml` (`milestone3-binary-repro`, `milestone3-binary-repro-compare`)
- `clg fmt` (`25.2.13`) provides deterministic `.clear` formatting and supports `--check` fail-closed drift gating for release workflows.
- `clg lint` (`25.2.14`) provides deterministic `.clear` quality/safety checks with `--deny-warnings` fail-closed mode.
- `clg.trust-policy.json` (strict preflight schema v0) and `trust-policy.json` (compile-time verify schema v1 with trust anchors) are separate contracts.
