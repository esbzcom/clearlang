# Phase 51.0 - Safe Operator and Environment-Isolation Policy

Date: 2026-10-17

Status: Locked local policy. It authorizes documentation, fixtures, offline
validation, and local simulation only. It authorizes no key creation or import,
Safe deployment or configuration, RPC access, transaction construction,
signature, broadcast, registry deployment, registry invocation, or mainnet
claim.

## Decision

ClearLang uses distinct development, pilot, and production security domains.
They may promote the same reviewed source and release bundle, but they must
never share signing material, recovery material, Safe account, endpoint
credential, nonce record, request identifier, or retained environment secret.
Environment selection is explicit in every later request and validator result;
it is never inferred from a wallet, an RPC URL, a default network, or a
configuration file.

This policy implements the Safe-owner amendment
[`phase-50.2-safe-owner-amendment.md`](phase-50.2-safe-owner-amendment.md).
It does not add a ClearLang language feature, a standard-library API, an EVM
adapter, or a generic plugin system.

## Environment Boundary

| Domain | Identity and purpose | Signing authority | Hard boundary |
| --- | --- | --- | --- |
| Development | Local, disposable developer/test-chain work. It is not pilot or production evidence. | Locally generated, short-lived developer keys are permitted only outside artifacts and logs. | No production or pilot key, Safe owner, endpoint credential, address, nonce, or request record may enter development. |
| Pilot / staging | Historical `clg.evm-sepolia-pilot.v1`, chain ID `11155111`, for explicitly authorized public-testnet exercises only. | Dedicated test-only EOAs or a separately established test Safe. | It may not use a production owner, production Safe, production deployer/executor, or production secret. A testnet result never authorizes mainnet action. |
| Production | `clg.evm-ethereum-mainnet-safe-registry.v1`, Ethereum Mainnet chain ID `1`, subject to every Milestone 6 gate. | The fixed registry owner is the separately established 2-of-3 Safe specified in Phase 50.2; deployment and fee-paying executor roles remain separately controlled. | Production material never appears in development or pilot records. No actual production identity is selected or accepted by this policy. |

The words *development*, *pilot*, *staging*, and *production* are labels, not
permissions. Only an exact named target profile plus its chain ID and the
later approved request format can identify an action.

## Credential and Authority Rules

1. A key, seed phrase, hardware-wallet export, recovery secret, API/RPC
   credential, or encrypted key store belongs to exactly one environment. It
   must not be copied, restored, imported, or reused in another environment.
2. Production Safe ownership is exactly the three separately held,
   hardware-backed owner keys and threshold specified in Phase 50.2. A person,
   device, seed phrase, recovery process, or security domain must not control
   more than one production owner key.
3. The production deployment EOA and fee-paying Safe executor, if selected,
   are distinct from Safe owner keys and from each other unless a later
   approved policy explicitly makes a narrower, reviewed exception. Neither
   role grants registry authority.
4. Development keys are disposable. Pilot keys are test-only. Neither may be
   promoted, funded as production authority, used as Safe owners, or treated
   as continuity/recovery material for production.
5. Repository content, fixtures, reports, logs, screenshots, tickets, and
   approval records may contain only public addresses, non-secret identifiers,
   public-code identities, and redacted attestations. They must never contain
   private material, endpoint credentials, raw device exports, exact physical
   locations, or machine-specific absolute paths.
6. An approval is bound to one environment, one target profile, one chain ID,
   one request digest, one release-bundle identity, and one expiry. An approval
   from another environment is invalid even if the source, address, or signer
   text happens to match.

## Production Safety Rules

### Before any future Gate B acceptance

The validator must fail closed unless the request binds the published product
profile, Ethereum Mainnet chain ID `1`, an exact approved release bundle, and
the Safe identity/configuration evidence required by Phase 50.2. It must reject
missing, substituted, stale, cross-environment, or implicit inputs.

No production request may discover a network, account, nonce, gas/fee value,
Safe configuration, owner set, threshold, module, guard, fallback handler, or
signer. Those are supplied as explicit reviewable inputs and independently
observed only at the later gate that permits observation.

### Allowed action shapes after all applicable gates

| Action | Authority and constraints |
| --- | --- |
| Registry deployment | A separate hardware-backed deployment EOA sends the one direct, zero-value registry creation. The immutable `owner` constructor input is the already-established approved Safe address. At least two Safe owners provide bound, non-secret approval evidence; the Safe does not execute this deployment. |
| Registry revision | The Safe executes one canonical `set_value(next)` inner `CALL` to the recorded registry, with inner value `0`, a valid 2-of-3 threshold signature set, and an explicit Safe nonce. A separate executor may only broadcast the fully authorized outer transaction. |

No other production action is allowed by this policy. It prohibits proxy/factory
deployment, ownership transfer, owner rotation during an action, Safe modules,
guards, fallback extensions, batching, delegatecall, arbitrary Safe calls,
relays, automatic replacement/cancellation, automatic nonce management, and
nonzero native value.

## Stop, Rotation, and Incident Policy

There is no on-chain pause or recovery capability in the registry profile.
The actual stop action is to stop local validation, signing, executor use, and
submission; preserve non-secret evidence; and notify the incident owner and
policy approver. A halted request must not be resumed by changing an input in
place.

| Condition | Required response |
| --- | --- |
| Suspected loss, compromise, duplication, or improper custody of any production key or credential | Stop all production activity. Quarantine the affected role, preserve non-secret evidence, and require a new governance/security review. Do not use a break-glass key. |
| Safe owner, threshold, code identity, module/guard/fallback configuration, or Safe nonce differs from the approved record | Reject before signing or submission. Do not normalize, repair, rotate, or accept the changed configuration automatically. |
| Pilot/development material is proposed for production, or vice versa | Reject as cross-environment contamination; revoke the request and create a new record only after review. |
| Deployment EOA compromise before registry deployment | Stop and retire that planned request; a replacement EOA requires fresh approval and a new request. |
| Deployment EOA compromise after registry deployment | Preserve evidence and assess it, but do not imply registry write authority: the immutable owner remains the Safe. Any change to operational policy still requires review. |
| Executor compromise or unavailable executor | Stop submission and arrange a separately approved executor change. Never reuse an owner key merely for convenience. |

Safe owner rotation, threshold changes, Safe replacement, recovery, or any
other governance configuration change are outside this profile. They require a
new policy profile, fresh identity evidence, fresh approval, and a reopened
review; they are not an operational escape hatch.

## Evidence and Retention Rules

Later Phase 51 records must retain only the minimum non-secret material needed
to show: environment/profile binding; public addresses; public code and Safe
configuration identities; request and release digests; approval references;
validator result; and incident/stop disposition. They must be canonical,
path-safe, reproducible, and redacted before review or publication.

The first request-schema work must include positive and negative fixtures that
prove a development or pilot approval cannot validate a production request,
and that a production record cannot be replayed in either lower environment.
Secret scanning and path-leakage checks are mandatory before any record is
accepted as evidence.

## Non-Claims

Environment separation does not prove hardware security, Safe correctness,
custody quality, endpoint honesty, transaction finality, source correctness,
or production readiness. It is one fail-closed operational boundary, to be
enforced by the subsequent local transaction-policy validator and later
evidence gates.

## References

- `docs/design/phase-50.0-milestone-6-production-profile-lock.md`
- `docs/design/phase-50.2-safe-owner-amendment.md`
- `docs/design/phase-42.0.2-sepolia-pilot-profile-lock.md`
- `docs/todo/milestone_6_roadmap.md`
