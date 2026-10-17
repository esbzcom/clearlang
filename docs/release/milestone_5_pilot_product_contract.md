# Milestone 5 Pilot Product Contract

## Product and boundary

ClearLang Milestone 5 is a **locally validated, testnet-only pilot** for one proof-carrying,
caller-authorized configuration registry. It supports only `clg.evm-sepolia-pilot.v1` with
`clg.evm-stateful-scalar.v1` on Sepolia chain ID `11155111`.

It must not be described as generic EVM support, remote-CI certification, mainnet readiness, or
custody safety. It is not a token, wallet, vault, exchange, lending protocol, proxy system,
general-purpose contract runtime, or financial-product approval.

| Evidence layer | What it supports | What it does not support |
| --- | --- | --- |
| Local source/proof | Supported source obligations under the named compiler and solver profile. | Unmodeled language behavior, cryptographic hardness, host correctness, or chain behavior. |
| Release verification | Bundle-local identity and signing/assurance verification. | Edited artifacts, an untrusted verifier/key, or a live deployment. |
| Simulation and replay | Bounded simulator and retained observation consistency. | RPC honesty, consensus, finality, or another chain. |
| Testnet observation | A retained, bound Sepolia observation. | Mainnet suitability, permanence, custody, or production financial safety. |

## Supported workflow

The required local acceptance command is:

```text
cargo run --offline -p xtask -- pilot-precheck --report out/pilot-precheck-full.json
```

It runs only under the exact local host/toolchain/solver contract in
[`phase-40.2.1-local-toolchain-support-lock.md`](../design/phase-40.2.1-local-toolchain-support-lock.md).
A passing `clg.pilot-precheck-report.v1` report with `scope: "full"` is required before an
operator considers an explicit testnet action. Remote automation is informational only and cannot
replace a local report.

1. Build/release the reference registry from the approved checkout and retain its bundle.
2. Run the full local pilot precheck and the clean-directory digest-projection fixture.
3. Verify the bundle with `clg-contract-verifier` and an explicit release public key.
4. Follow the pilot operator runbook for an explicit Sepolia deployment, authorized transition,
   and unauthorized negative transition.
5. Capture sanitized observations and replay them locally without a network.

No command may infer an endpoint, chain ID, signer, nonce, fee, gas policy, release key,
verifier distribution key, or target address. These are reviewed operator inputs.

## Compatibility and verifier inputs

Phase 40.2 is authoritative for local toolchain and evidence compatibility/rollback. Phase 43.0.2
is authoritative for verifier compatibility. The fixed v1 interfaces are:

| Input or output | Required form |
| --- | --- |
| Release bundle | `clg.contract-release-bundle.v1`, schema `1` |
| Pilot target / execution profile | `clg.evm-sepolia-pilot.v1` / `clg.evm-stateful-scalar.v1` |
| Verifier result | `clg.contract-release-verifier-result.v1`, schema `1` |
| Pilot precheck | `clg.pilot-precheck-report.v1`, schema `1`, `scope: "full"` |
| Verifier trust inputs | Exact archive, checksum listing, signed distribution manifest, externally trusted distribution public key/key ID, and explicit release public key |

The verifier has no source checkout, wallet, signer, RPC, telemetry, plugin, or network-discovery
requirement. Unknown schemas, profiles, capabilities, paths, keys, or versions are rejected; no
nearest-version fallback is permitted.

## Responsibilities and exclusions

| Actor | Must do | Must not assume |
| --- | --- | --- |
| Release owner | Review source, approve bundle/key, retain local evidence. | A signature proves deployment or protects edited artifacts. |
| Deploy operator | Review explicit target inputs and retain sanitized observations. | The CLI or RPC chooses a chain, wallet, fee, nonce, or account. |
| Independent verifier | Validate package integrity and run offline verification with explicit public keys. | A source checkout, private key, endpoint, or network connection. |
| Observer | Review non-secret evidence and report mismatches. | Authority to submit, replace, or approve a transaction. |

The pilot excludes mainnet deployment, custody or token value, automatic key recovery, generic
EVM semantics, external calls, dynamic ABI values, arbitrary state, broader language features,
hosted verification, containers/emulation, and remote-CI approval.

Stop on any precheck, verification, chain-binding, receipt-identity, authorization, or offline
replay failure; on an unexpected receipt; or on suspected test-key exposure. Preserve non-secret
evidence and follow the incident guide. A failed report or verifier result is diagnostic evidence,
never a result to edit into acceptance.

## References

- [Pilot operator runbook](pilot-operator-runbook.md)
- [Offline contract-verifier walkthrough](contract-verifier-offline-walkthrough.md)
- [Pilot incident and rollback guide](milestone_5_pilot_incident_and_rollback.md)
- [Milestone 5 pilot closure decision](milestone_5_pilot_closure_decision.md)
- [Local toolchain support lock](../design/phase-40.2.1-local-toolchain-support-lock.md)
- [Evidence compatibility and rollback lock](../design/phase-40.2.2-local-evidence-compatibility-and-rollback-lock.md)
- [Verifier compatibility lock](../design/phase-43.0.2-verifier-compatibility-lock.md)
