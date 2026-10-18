# Phase 53.1 - Production Support, Compatibility, and Response Policy

Date: 2026-10-17

Status: Locked candidate-support policy. It authorizes no endpoint contact,
signing, Safe operation, deployment, invocation, or production claim.

## Supported Candidate Surface

| Element | Supported candidate value | Rule |
| --- | --- | --- |
| Rust / Cargo | `1.89.0` | Exact value from `rust-toolchain.toml`; `clippy` and `rustfmt` must be present. |
| Solver | Z3 `4.16.0` | Only the checked-in, signed, checksummed bundle under the Phase 25.1.16 policy. |
| Local host | `windows-x64`, `linux-x64-glibc-2.39`, or `macos-x64-15.7.3` | Exact Phase 40.2.1 host labels and native prerequisites only. |
| Release bundle | `clg.contract-release-bundle.v1`, schema `1` | Its verified identity must equal the separately retained replay anchor. |
| Independent verifier | Signed verifier line `0.1.x` | Its signed capability manifest must explicitly support every exact production record/profile format; a pilot-only capability does not qualify. |
| Target profile | `clg.evm-ethereum-mainnet-safe-registry.v1`, chain ID `1` | No alternate chain or profile is compatible by proximity. |
| Transaction envelope | Legacy EIP-155, zero native value | Deployment is direct; later revision is a Safe v1.4.1 `execTransaction` with inner zero-value `CALL` to canonical `set_value(next)`. |
| Observation clients | Exactly two separately operated observers with distinct client families | Each exact family/version is an explicit candidate input recorded in the Phase 52 evidence; no unlisted version is inferred compatible. |

The table identifies a candidate support surface, not an assertion that any
runtime, client, Safe, endpoint, hardware device, chain, or verifier is
correct. A currently unavailable production-capable verifier or an unapproved
observer-client record leaves the production precheck `blocked`.

## Upgrade, Vulnerability, and Compatibility Rules

Every compiler, solver, verifier, host, target-profile, Safe-version,
transaction-format, observer-family/version, schema, bundle, or evidence-format
change is incompatible until a new support entry, release verification,
identity anchor, replay, and production precheck are created. No automatic
upgrade, compatibility range, version alias, downgrade, or fallback is
allowed.

Solver advisories follow the existing 14-day maximum advisory-age policy. A
revoked, compromised, malformed, expired, or capability-incomplete verifier;
an affected toolchain/solver; or an observer/Safe identity drift stops the
candidate. Preserve secret-free evidence, quarantine the affected artifact or
role, and restart from a newly approved candidate; do not repair a report or
reuse prior approval by implication.

Emergency response is operational stop and evidence preservation. It does not
permit transaction cancellation, Safe reconfiguration, key recovery,
credential disclosure, endpoint substitution, automatic retry, or a
production claim.

## Retention and Public Evidence

Retain the secret-free production-profile verification record for at least
seven years after the final Gate E decision, including the support matrix,
release verification identity, identity anchor, policy validator report,
correspondence evidence, replay report, independent-observation record,
precheck report, review disposition, and launch decision. Publish only the
approved non-secret subset: profile/chain, tool and format identifiers,
digests, diagnostics, decision status, and retention policy.

Never publish or retain in repository artifacts a private key, seed, recovery
material, endpoint credential/URL, raw signature, absolute path, device serial,
or exact device location. Protected operational records remain outside the
repository and do not substitute for the required secret-free evidence.
