# Phase 40.2.1 - Local Toolchain Support Lock

Date: 2026-10-15
Status: Locked
Owner: release-assurance-owner

## Purpose

Define the exact local environment that may produce Milestone 5 pilot-precheck evidence. This is
a source-checkout acceptance contract, separate from the independently distributed offline
verifier contract. It makes the local command reproducible without treating a nearby toolchain,
host, solver, schema, or remote automation result as equivalent.

## One-command local acceptance path

From an approved checkout with its locked Cargo dependencies already available locally, run:

```text
cargo run --offline -p xtask -- pilot-precheck --report out/pilot-precheck-full.json
```

`CARGO_NET_OFFLINE=true` is equivalent. The command must run from the checkout root, use the
checked-in `Cargo.lock`, and retain the emitted report unchanged. An offline dependency cache is
a prerequisite; an offline failure to resolve a missing crate is a setup failure, not permission
to contact a registry during acceptance.

The command is the only supported local pilot-acceptance path. Individual fixture commands are
diagnostic aids and cannot be combined into an acceptance claim. A non-zero exit or a report whose
`overall_status` is not `passed` is a rejection.

## Supported local host matrix

The pilot source-checkout acceptance surface is supported only on these x86_64 host/toolchain
pairs. The labels intentionally match the trusted solver matrix and the native verifier delivery
contract where they overlap.

| Host label | Rust host target | Native toolchain prerequisite | Solver bundle |
| --- | --- | --- | --- |
| `windows-x64` | `x86_64-pc-windows-msvc` | Microsoft C++ Build Tools / MSVC linker | `tools/proof/z3/windows/z3.exe` |
| `linux-x64-glibc-2.39` | `x86_64-unknown-linux-gnu` | GNU-compatible C compiler and linker on glibc 2.39 | `tools/proof/z3/linux/z3` |
| `macos-x64-15.7.3` | `x86_64-apple-darwin` | Xcode Command Line Tools linker | `tools/proof/z3/macos/z3` |

ARM hosts, musl Linux, 32-bit hosts, other Windows toolchains, other operating-system versions,
emulation, containers, cross-compilation, and a hosted build service are not supported ways to
produce pilot acceptance evidence. They may be investigated separately, but their output is not a
substitute for a report from this matrix.

## Required local tools

`rust-toolchain.toml` pins Rust and Cargo to `1.89.0` and requires the `clippy` and `rustfmt`
components. The host-native linker in the table is required to compile the workspace. The
version-controlled `Cargo.lock` is part of the approved checkout and Cargo must honor it; no
lockfile update, dependency fetch, `--locked` bypass, or ambient executable is accepted as part
of the acceptance run.

The solver is bundled rather than installed from the host `PATH`. Its identity is Z3 `4.16.0`,
with the deterministic options and time limits in
`docs/design/phase-25.1.4-solver-profile.lock.json`. Each bundled executable requires its adjacent
SHA-256 and detached Ed25519 signature as governed by
`docs/design/phase-25.1.16-solver-supply-chain.lock.json`. A missing, unsigned, malformed,
wrong-version, or untrusted solver must fail closed; it must not fall back to an arbitrary host Z3.

No RPC endpoint, wallet, private key, Docker/OCI runtime, Node.js, package manager, cloud account,
or remote CI service is required or accepted by the local precheck. Git is needed only to obtain
the approved checkout; it is not invoked by the acceptance command.

## Locked compatibility set

| Element | Exact supported value |
| --- | --- |
| Rust / Cargo | `1.89.0` |
| Solver | Z3 `4.16.0`, external CLI backend, checked-in trusted bundle |
| Solver profile schema | `1`, policy version `25.1.4` |
| Solver supply-chain schema | `1`, policy version `25.1.16` |
| Pilot target profile | `clg.evm-sepolia-pilot.v1` |
| Pilot execution profile | `clg.evm-stateful-scalar.v1` |
| Release bundle | `clg.contract-release-bundle.v1`, schema `1` |
| Independent verifier line | `0.1.x`, with capabilities declared by its signed manifest |
| Verifier result | `clg.contract-release-verifier-result.v1`, schema `1` |
| Pilot-precheck report | `clg.pilot-precheck-report.v1`, schema `1`, `scope: "full"` |

The local precheck report records its own `xtask` version and evidence digests; it is not a general
host-attestation format. The host/toolchain contract above is checked before running it. The
upgrade, incompatibility, and rollback decision procedure is locked in
`phase-40.2.2-local-evidence-compatibility-and-rollback-lock.md`; an unlisted version or platform
is rejected for pilot acceptance rather than inferred compatible.

## Remote-automation boundary

Remote automation is informational only. It may report a regression, but it cannot create,
replace, bless, or repair local pilot evidence. A local passing report remains necessary, and a
remote passing result never overrules a local failure. No acceptance procedure downloads inputs,
uses credentials, or contacts a network after the offline prerequisites are present.

## Design Principles Check

- **Simple for users:** one documented offline command has a bounded set of prerequisites.
- **AI-friendly:** exact host labels, versions, paths, formats, and rejection boundaries prevent
  compatibility guessing.
- **Provably correct:** solver and evidence identities are pinned and failures remain fail-closed.
- **Crypto-focused:** supply-chain trust is explicit while wallets, custody, and live network
  behavior remain outside local acceptance.

## References

- `rust-toolchain.toml`
- `docs/design/phase-25.1.4-solver-profile.lock.json`
- `docs/design/phase-25.1.15-solver-support-matrix.lock.json`
- `docs/design/phase-25.1.16-solver-supply-chain.lock.json`
- `docs/design/phase-40.1.1-pilot-precheck-cli-report-lock.md`
- `docs/design/phase-43.0.2-verifier-compatibility-lock.md`
