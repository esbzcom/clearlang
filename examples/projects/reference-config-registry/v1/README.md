# Reference Configuration Registry — v1

`reference-config-registry/v1` is the single Milestone 5 pilot contract. It demonstrates one
caller-authorized scalar configuration update under the locked
`clg.evm-stateful-scalar.v1` execution profile and
`clg.evm-sepolia-pilot.v1` target profile. It is a testnet-only reference, not a token, wallet,
vault, proxy, value-transfer mechanism, or general registry framework.

## State and invariant

| Field | Type | Rule |
| --- | --- | --- |
| `owner` | `std::eth::Address` | Set exactly once by `init`; no v1 transition writes it. The canonical external form is `0x` plus 40 lowercase hexadecimal digits. |
| `value` | `U64` | The configuration value. `U64` makes its complete permitted domain `0..=18,446,744,073,709,551,615`; it cannot be negative or exceed that bound. |

The v1 invariant is stated in the source comment and README: `value` remains a `U64` and `owner`
remains the identity set during initialization. The locked caller-aware source profile deliberately
does not admit a separate contract-level invariant expression; the type, canonical ABI, and EVM
storage word enforce the complete `U64` domain. A successful `set_value(next)` has result
`next`, writes only `value`, and emits exactly one canonical `ValueChanged(next)` event.

## Constructor

`init(owner: Address, initial_value: U64)` requires two explicit ABI inputs:

1. `owner`: the fixed authorized caller;
2. `initial_value`: the initial bounded configuration value.

Initialization writes both state fields. It is not an owner-rotation path, and callers must not
interpret a new deployment as an in-place upgrade.

## Permitted transition

`set_value(next: U64) -> U64` is the only mutable v1 transition.

Its complete authorization condition is:

```text
std::eth::address::equals(std::eth::context::caller(), state.owner)
```

The source, proof model, simulator trace, EVM `CALLER` guard, signed transaction sender, and
target receipt all bind this same canonical caller identity. An owner invocation replaces `value`
with `next`, emits `ValueChanged(next)`, and returns `next`. A missing, malformed, or non-owner
caller fails; it must not commit state, emit a successful event, or yield a successful target
receipt.

## Explicitly excluded from v1

- owner rotation, multiple owners, roles, multisig, delegation, recovery, or pause controls;
- values outside `U64`, tokens, balances, value transfer, or external calls;
- proxy/delegate upgrades, mutable branching/loops, dynamic values, collections, and arbitrary
  ABI/EVM behavior; and
- mainnet, custody, finality, endpoint-honesty, or production-readiness claims.

The next version may be introduced only through a separately specified migration-compatible
scenario. It must retain v1 identity evidence and cannot silently change this contract's owner or
state semantics.

## References

- [Source](config_registry.clear)
- `docs/design/phase-41.0-caller-authorization-profile-lock.md`
- `docs/design/phase-42.0.2-sepolia-pilot-profile-lock.md`
- `docs/todo/milestone_5_roadmap.md`
