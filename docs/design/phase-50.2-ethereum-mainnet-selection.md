# Phase 50.2 - Ethereum Mainnet Target Selection

Date: 2026-10-17

Status: Selected for the Milestone 6 target-profile lock. This decision
authorizes documentation and local policy/fixture work only. It authorizes no
RPC contact, signer use, transaction construction, deployment, invocation, or
mainnet submission.

## Decision

The sole eligible mainnet for the first ClearLang production profile is
**Ethereum Mainnet**, with EIP-155 chain ID **`1`**.

The product remains the fixed-owner scalar Configuration Registry Profile: a
zero-native-value public configuration-revision beacon. It is not a generic
EVM claim and it does not make any other EVM-compatible network supported.

Ethereum Mainnet was selected because this profile values conservative,
widely recognizable provenance and a clear finality model more than frequent,
low-cost updates. This is a product-fit decision, not a statement that other
networks are less safe or unsuitable for other products.

## Boundaries

| Decision | Locked meaning |
| --- | --- |
| Network | Ethereum Mainnet only, chain ID `1`. |
| Product | The Phase 50.0 fixed-owner, caller-authorized scalar registry only. |
| Value/exposure | Native transaction value is `0`; the contract has no asset, custody, transfer, or external-call behavior. |
| Later networks | They require independently versioned Target Profile Packs and their own full evidence/approval sequence. They are not implied by Ethereum compatibility. |
| Authority | This record changes no signer, endpoint, transaction, fee, nonce, client, or launch authority. |

## Still Required Before Phase 50.2 Closes

This selection deliberately does **not** lock transaction or observation
semantics. The final Ethereum target profile must still name, review, and test:

1. the approved transaction type and exact constructor/invocation encoding;
2. the minimum RPC method set, endpoint authentication boundary, and supported
   client/version observations;
3. confirmation depth, Ethereum finality condition, observation interval,
   reorganization window, and failure response;
4. gas, fee, nonce, sender, replacement/cancellation, and zero-value policy;
5. terminology fixtures that bind each qualified claim to its evidence layer.

Those inputs belong to the remaining Phase 50.2 tasks and later security and
launch gates. Only Phase 54.4, after an explicit Phase 54.3 approval, may
submit the one permitted limited mainnet action.

## Future Extension Rule

The future package split is:

```
Configuration Registry Profile Pack
        + one signed Target Profile Pack
        = one lockfile-bound product/chain deployment profile
```

For example, a future Base target would be a new
`clg.target.evm.base-mainnet.v1` Target Profile Pack that must independently
lock and prove Base-specific target behavior. It must not alter
`clg.profile.config-registry.v1`, make a generic EVM claim, or silently fall
back from Ethereum to Base. See
[`verified-profile-packs.md`](verified-profile-packs.md#target-profiles-and-future-evm-networks)
for the complete implementation path.

## References

- [Ethereum network documentation](https://ethereum.org/developers/docs/networks/)
- [EIP-2228: Ethereum Mainnet name and chain ID 1](https://eips.ethereum.org/EIPS/eip-2228)
- [Ethereum proof-of-stake finality overview](https://ethereum.org/developers/docs/consensus-mechanisms/pos/faqs)
- `docs/design/phase-50.0-milestone-6-production-profile-lock.md`
- `docs/todo/milestone_6_roadmap.md`
- `docs/design/verified-profile-packs.md`
