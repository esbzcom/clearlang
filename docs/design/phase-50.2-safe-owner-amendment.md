# Phase 50.2 - Safe Owner Amendment

Date: 2026-10-17

Status: Locked policy amendment. It authorizes local documentation, fixtures,
and policy validation only. It authorizes no Safe deployment, Safe
configuration, endpoint contact, signer use, transaction construction,
registry deployment, invocation, or mainnet submission.

## Supersession Decision

The direct-owner target profile
`clg.evm-ethereum-mainnet-registry.v1` is superseded before any target action.
The only candidate target profile is now
`clg.evm-ethereum-mainnet-safe-registry.v1`.

The ClearLang `ConfigRegistry` source does not change. Its immutable `owner`
constructor input must instead be the address of an independently established
Ethereum Mainnet **Safe Smart Account v1.4.1** with exactly three owners and a
threshold of two. The registry has no knowledge of Safe, makes no external
call, and receives no new language or standard-library feature.

The Safe is an external owner/governance system. Its threshold enforces
authority for later `set_value` operations; ClearLang source proof continues to
cover only the registry's `msg.sender == owner` behavior. Safe documents define
its threshold as the minimum number of owners required for execution and its
`execTransaction` flow as the owner-authorized action path. [Safe smart-account
concepts](https://docs.safe.global/advanced/smart-account-concepts)
[Safe transaction execution](https://docs.safe.global/sdk/protocol-kit/guides/execute-transactions)

## Hardware-Backed Governance Boundary

| Role | Locked rule |
| --- | --- |
| Safe owners | Exactly three separately held, hardware-backed Ethereum signing keys. No person, device, seed phrase, or recovery material may control more than one owner key. |
| Safe threshold | Exactly 2 of 3 owners must sign each registry invocation or any Safe-governance action that could affect registry control. |
| Custody separation | Owner devices, recovery material, and custodians must occupy independent physical/security domains. Device-vendor/model diversity is recorded in later non-secret operations evidence. |
| Executor | A separately controlled, non-owner fee-paying EOA may broadcast a fully signed Safe transaction. It obtains no registry authority from broadcasting. |
| Release/policy/deploy roles | The Phase 50.0 role separation remains mandatory. A Safe signature is approval evidence, not proof, release verification, endpoint honesty, or finality. |

Private keys, seed phrases, device serial numbers, recovery material, endpoint
credentials, and exact physical locations are prohibited from the repository,
fixtures, release evidence, reports, and logs.

## Permitted Action Shapes

### One limited launch deployment

The Phase 54.4 limited launch may directly deploy the reviewed registry with a
separate, hardware-backed deployment EOA. The constructor input `owner` must
be the already-established Safe address; `initial_value` is an explicit
approved `U64`; transaction value is exactly zero. The deployment request
requires non-secret approval evidence from at least two Safe owners, but the
Safe does not execute this creation transaction.

This preserves the one limited ClearLang launch action and avoids introducing
a factory, proxy, module, delegatecall, or batched deployment into the first
profile. An attacker who controls the deployment EOA cannot gain later registry
write authority because the immutable registry owner is the Safe address.

### Later registry invocation

The only post-launch mutation is a Safe transaction whose inner action is:

| Safe field | Locked value |
| --- | --- |
| `to` | Recorded deployed `ConfigRegistry` address |
| `value` | Exactly `0` |
| `data` | Canonical ABI calldata for `set_value(next)` with one approved `U64` |
| `operation` | `CALL` only; `DELEGATECALL` is prohibited |
| signatures | Exactly or at least two valid signatures from the frozen three-owner set, according to Safe's canonical ordering/validation rules |
| Safe nonce | Explicit request field; observed and compared but never inferred, incremented, or replaced automatically |

The separately signed outer submission invokes the pinned Safe
`execTransaction` ABI with the approved Safe transaction and signatures. It
may use the existing locally signed legacy EIP-155 outer envelope only. EIP-1559
support, account-managed transactions, relay services, modules, guards,
fallback extensions, delegatecalls, batching, and arbitrary Safe calls remain
outside this profile.

## Required Safe Identity Before Gate B Completes

No Safe address is selected by this amendment. Before Phase 51 may accept a
request, the policy record must provide and independently review all of these
explicit inputs:

1. Ethereum Mainnet Safe proxy address and its pinned singleton/version;
2. proxy and singleton runtime-code identities and Safe setup evidence;
3. the exact three public owner addresses, threshold `2`, Safe nonce, and
   absence of unapproved modules, guards, fallback extensions, or delegated
   authority;
4. the Safe ABI/version identity used to construct and independently decode
   signatures and `execTransaction` calldata; and
5. a non-secret hardware-custody attestation for each owner and the separate
   executor/deployment key policy.

Those records are evidence inputs, not secrets and not a substitute for
on-chain observation. If the Safe is not already established and independently
reviewable, the production profile stops before Gate B. Creating or changing a
Safe is an external governance program, not a ClearLang launch shortcut.

## Revised RPC and Observation Policy

The outer request retains the direct-profile chain binding, zero-value rule,
confirmation/finality rule, 60-second interval, 30-poll window, 12-block
minimum, two post-finality observations, and independent-observer requirement.
In addition to the direct-profile observation surface, later local policy and
correspondence work must validate Safe configuration reads through canonical
ABI `eth_call` requests and must bind:

- Safe proxy address, runtime code, singleton identity, owners, threshold,
  nonce, and disallowed-configuration results;
- Safe typed-data/request digest, ordered owner signatures, `execTransaction`
  calldata, and outer transaction hash; and
- the inner registry target, zero value, `CALL` operation, `set_value` calldata,
  Safe execution result, and the registry's `ValueChanged(next)` event.

Any wrong Safe, owner set, threshold, code identity, Safe nonce, signature,
inner target, value, operation, event, receipt, block identity, finality
observation, or independent observer result is a stop condition. Automatic
replacement, cancellation, configuration change, module enablement, owner
rotation, or recovery is prohibited in this profile; each needs a fresh
governance review and a revised policy profile.

## Claim Boundary

The Safe gives on-chain threshold control for the named registry owner only. It
does not prove the Safe implementation, hardware device, signer UI, endpoint,
Ethereum consensus, finality, source proof, release correspondence, or generic
EVM safety. The production claim remains unavailable until every remaining
Milestone 6 gate closes.

The terminology fixture is revised to require qualified Safe-governance claims
and continues to reject bare `production`, `mainnet`, `verified`, and `proved`
claims.

## References

- `docs/design/phase-50.2-ethereum-mainnet-target-profile-lock.md`
- `docs/design/phase-50.0-milestone-6-production-profile-lock.md`
- `docs/fixtures/phase-50.2/ethereum-mainnet-claim-terminology-fixtures.json`
- [Safe Smart Account Reference](https://docs.safe.global/reference-smart-account/overview)
