# ClearLang Contract Product Contract

## Supported product surface

ClearLang's Milestone 4 contract product supports one deterministic EVM-compatible workflow for
the `clg.evm-stateful-scalar.v1` execution profile through the
`clg.evm-compatible.v1` target adapter.

The supported source surface is one contract with direct `init`, pure reads, and straight-line
mutable transitions over `Bool`, `U8`, `U64`, `U128`, and `Int` state fields and parameters.
State schemas, ABI/wire ABI, EVM artifacts, proof evidence, simulator traces, campaign reports,
signatures, assurance manifests, target receipts, and contract-release bundles are canonical,
versioned JSON artifacts.

Primary commands:

- `clg contract test` runs a seeded local simulator campaign and preserves replay inputs.
- `clg simulate` executes one bounded local transition and writes a trace.
- `clg contract release` builds, proves, signs, verifies, campaigns, and packages a release set.
- `clg contract verify-release` independently verifies a release bundle from local artifacts.
- `clg target deploy`, `clg target call`, and `clg target invoke` execute explicit target actions.

Target selection, RPC endpoint, chain ID, signer, nonce, value, gas limit, and gas price are
always explicit. No command discovers a wallet, chooses a network, estimates fees, or turns a
local simulation into a target transaction.

## Compatibility policy

Compatibility is limited to the locked London-opcode scalar profile and strict JSON-RPC fixture
surface. Artifact/schema/profile compatibility is exact and fail-closed: target commands and
bundle verification reject changed compiler/source identities, schema or wire-ABI digest drift,
malformed evidence, unsafe artifact paths, invalid signatures, and unknown profile versions.

Storage changes are append-only only when the version increases and every prior field retains its
identity, order, name, and type. Other changes require an explicit migration declaration bound to
the exact prior schema digest. Deployment addresses are immutable; neither proxy upgrades nor
in-place automatic rollback are supported.

The stable IDE contract is the documented CLI profile: `--non-interactive --json-errors
--json-events`, with JSON diagnostics on stdout and events on stderr. Simulation execution errors
use C142 and carry matching trace source/stack evidence.

## Release checklist

1. Review the source boundary and ensure unsupported features are absent.
2. Check storage compatibility against the prior schema, or review the exact migration evidence.
3. Run a deterministic `clg contract test` campaign and retain its plan, report, inputs, and
   traces.
4. Run `clg contract release` with the approved Ed25519 key, public key, key ID, and output
   directory; retain every generated artifact unchanged.
5. Run `clg contract verify-release` from the bundle and approved public key on an independent
   checkout or workstation.
6. Before target submission, review all explicit target/signer/nonce/value/gas inputs and run a
   read-only target call where applicable.
7. Retain successful target receipts with the release bundle. A revert or RPC failure is not a
   successful receipt and must not be substituted with a simulator trace.
8. For upgrade, rollback, or incident actions, follow `docs/contract-operations.md` and preserve
   the original artifacts and evidence before changing clients or submitting another transaction.

## Known exclusions

The product does not support arbitrary EVM bytecode or clients, multiple chain targets, EIP-1559
transactions, automatic nonce/fee selection, proxy/delegate calls, fallback dispatch, value
transfer, external calls/callbacks, reentrancy execution, dynamic ABI values, collections,
`U256`, branches or loops that mutate state, remote signing, wallet discovery, transaction
replacement, confirmation-depth policy, or automatic deployment upgrade/rollback.

The proof claim excludes cryptographic hardness, constant-time or side-channel behavior, and
target-runtime implementation correctness unless separately modeled and verified. Unsupported
features fail closed; no manual artifact edit, raw calldata substitution, or alternate target
profile expands this contract.

## References

- `README.md`
- `docs/contract-operations.md`
- `docs/ide/vscode-cli-profile.md`
- `docs/evidence/phase-30.6-contract-release-ci.md`
- `docs/design/phase-30.3.3.3.0-stateful-scalar-evm-profile-lock.md`
