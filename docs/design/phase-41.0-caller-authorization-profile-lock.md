# Phase 41.0 - Caller Authorization Profile Lock

Date: 2026-10-13
Status: Locked
Owner: language-and-proof-owner

## Purpose

Define the smallest caller-authorization extension needed by the Milestone 5 configuration
registry without creating a global address type, generic authentication framework, or implicit
target context. This is a pre-release cutover: no released consumer needs the earlier scalar-only
target profile, so ClearLang may replace that incomplete profile rather than maintain two paths.

## Profile Decision

The sole stateful EVM profile identifier remains `clg.evm-stateful-scalar.v1`.

It gains only an EVM-chain-scoped `std::eth::Address` owner field, an
`std::eth::context::caller()` observation in one transition precondition, structural address
equality, and the EVM `CALLER`/guard mapping necessary to enforce that precondition. Every schema,
ABI, proof, artifact, campaign trace, target receipt, signature, assurance manifest, and release
bundle must name the profile exactly.

Existing unreleased scalar-only artifacts are intentionally unsupported after this cutover. They
must be rebuilt from source; no compatibility parser, migration, or dual target backend is added.

## Source Surface

The profile permits the following shapes only:

```clearlang
import std::eth::{Address}

contract ConfigRegistry {
    state {
        owner: Address;
        value: U64;
    }

    init(owner: Address, value: U64) {
        state.owner = owner;
        state.value = value;
    }

    mut function set_value(next: U64) -> U64
        require { std::eth::address::equals(std::eth::context::caller(), state.owner) }
        ensure { result == next }
    {
        state.value = next;
        emit ValueChanged(next);
        next
    }
}
```

This is illustrative syntax; the acceptance surface is the following contract, not a broader
address API:

1. `std::eth::Address` is the chain-scoped identity type. No global `Address` type is introduced.
2. An address may appear only as an `init` parameter or a persistent `state` field named `owner`.
   Address transition parameters, returns, events, collections, resources, conversions, and
   refinement predicates are outside this profile.
3. `std::eth::context::caller()` has no arguments, returns `std::eth::Address`, and may appear
   only as the left operand of the one permitted `require` guard in a contract-owned `mut`
   transition. It is unavailable in `init`, `pure` reads, function bodies, `ensure`, events, and
   arbitrary expressions.
4. `std::eth::address::equals(lhs, rhs)` accepts exactly two `std::eth::Address` operands and
   returns `Bool`. For this profile its only allowed use is the permitted caller/`state.owner`
   `require` guard. Source `==` is not extended for address values.
5. Source address literals and source constructors are not part of this profile. The owner enters
   only through the typed constructor input; callers enter only through explicit simulator/target
   context.

Every other caller/context operation fails during type checking with a new stable `T834`
diagnostic: caller authorization is outside the caller-aware contract profile. Unsupported address
placement or operation uses the same diagnostic rather than falling back to bytes, strings, or
integers.

## Compiler-Boundary Exception

`std::eth` remains an external-package candidate in the standard-library architecture. The
caller-aware profile permits only the opaque `std::eth::Address` representation to be
compiler-known, and only in these eight implementation locations: profile checking, call
lowering, VC generation, SMT refinement encoding, state-schema generation, EVM artifact
generation, EVM wire-ABI generation, and simulation. The `context::caller` and `address::equals`
operations are compiler-known only at the four sites that validate, lower, model, or record the
single permitted guard. The architecture conformance check encodes these path-and-symbol
allowlists exactly. It rejects every other Ethereum-adapter compiler reference until this locked
profile is deliberately amended.

## Canonical Address Representation

An EVM address is exactly 20 bytes. Its external JSON and CLI spelling is exactly:

```text
0x + 40 lowercase hexadecimal digits
```

No checksum/mixed-case form, whitespace, short form, integer, byte array, arbitrary string, or
null is accepted. Parsing normalizes neither case nor length: a non-canonical spelling is rejected
before simulation, ABI encoding, proof/release evidence creation, or target submission.

The state schema records `std::eth::Address` explicitly. EVM storage and ABI words encode the
address in the low 20 bytes of one 32-byte word with 12 leading zero bytes. An EVM value with a
non-zero high 12-byte prefix is rejected rather than truncated. Canonical traces, campaign inputs,
constructor arguments, receipts, and bundle evidence use the lowercase JSON form above.

## Type, Effect, and Proof Semantics

`std::eth::context::caller()` is target context, not a pure mathematical constant. The checker
allows it only in the restricted `mut` transition precondition described above. This prevents
ambient caller data from leaking into pure proofs, constructors, results, or arbitrary state
calculation.

For each caller-aware transition, the VC model introduces one fresh 160-bit entry identity
`caller@entry`. `state.owner` is a 160-bit entry/exit identity with ordinary frame conditions.
The successful transition relation includes exactly:

```text
caller@entry == state.owner@entry
```

Address equality is bitwise equality over those 160-bit values; it is not string comparison,
address derivation, signature verification, role membership, or an assumption about key control.
The release-proof artifact records the caller-aware profile and the guard identity. A proof cannot
claim success if it omits the guard relation or substitutes a scalar/string symbol.

## Runtime and Target Correspondence

### Simulator

For a caller-aware profile, `clg simulate --caller` accepts only the canonical address spelling.
The trace records it in `caller`. A failed guard produces the existing trace-backed `C142`
simulation failure, writes no `state_out` success state, emits no events, and records no successful
transition result.

### ABI and EVM artifact

The constructor accepts the owner as one static ABI `address` word. The owner storage field uses
the existing field-ID-derived slot rule with the address word representation above. The artifact
records the new profile, address storage mapping, caller guard mapping, and the exact IR-to-EVM
correspondence.

For the permitted guard, the emitted EVM code obtains the transaction context through `CALLER`,
compares it with the owner slot using `EQ`, and reverts before any `SSTORE`, `LOGn`, or successful
return when false. The target may not emulate this condition with calldata, a hard-coded caller,
or simulator-only behavior. Any IR/source shape without the listed mapping rejects before artifact
emission.

### Deploy/invoke and receipt

`clg target invoke` derives the canonical sender from the signing key and requires it to match the
explicit `--sender`; that exact canonical sender is the EVM `CALLER` for the submitted invocation.
The caller-aware target receipt must bind the canonical sender, chain ID, target profile, release
bundle/artifact identities, and invocation input. A reverted or malformed target result produces
no success receipt. A receipt with a substituted sender or profile fails release/receipt identity
verification.

## Unsupported Authorization Features

This profile explicitly excludes multiple owners, roles, owner rotation, multisig, key recovery,
pause controls, signature verification, EIP-712/domain separation, contract-wallet signatures,
delegation, arbitrary `msg.sender` access, `tx.origin`, calldata-controlled authorization,
address arithmetic/ordering/conversion/hashing, and every non-EVM address family. These require a
new design lock and profile.

## Required Acceptance Evidence

`41.1` and `41.2` complete only when local tests show all of the following:

1. Valid source parses/types/lowers/proves with an owner `Address` and permitted guard.
2. Every forbidden source placement/operation and malformed address spelling fails with `T834` or
   the locked trace-backed `C142` execution diagnostic, as appropriate.
3. An authorized simulator campaign commits the expected state/event; an unauthorized campaign
   commits neither state nor event and replays deterministically.
4. ABI constructor encoding, storage word layout, EVM `CALLER` behavior, and simulator traces
   agree for the same canonical owner/caller values.
5. Mutating the caller independently in source-derived evidence, campaign input/trace, ABI,
   artifact, target receipt, or release bundle fails identity verification and cannot yield a
   success receipt.
6. A scalar-only source remains valid when rebuilt, while an old artifact/evidence set cannot be
   accepted as caller-aware evidence without a source rebuild and complete identity regeneration.

## Design Principles Check

- **Simple for users:** one owner guard is easy to read and leaves no implicit wallet/context
  behavior.
- **AI-friendly:** a chain-scoped type, canonical spelling, one legal expression shape, and one
  stable diagnostic make generation and repair deterministic.
- **Provably correct:** the source guard, VC identity, simulator context, EVM `CALLER`, and receipt
  sender have an explicit correspondence and unsupported forms fail closed.
- **Crypto-focused:** it adds only the authorization primitive needed for a non-custodial registry;
  signatures, assets, and generic crypto remain out of scope.

## Exit Criteria

1. The roadmap marks `41.0`, `41.0.1`, `41.0.2`, and `41.0.3` complete and references this lock.
2. The new target-profile identifier, source shape, address encoding, VC relation, target mapping,
   receipt binding, diagnostics, and exclusions are unambiguous.
3. The lock preserves the existing chain-package rule against global address types and records the
   pre-release replacement of the incomplete scalar-only target profile.

## References

- `README.md`
- `docs/runtime/chain-packages.md`
- `docs/todo/milestone_5_roadmap.md`
- `docs/design/phase-30.3.3.3.0-stateful-scalar-evm-profile-lock.md`
- `docs/design/phase-30.3.3.4.0-evm-transaction-signing-policy-lock.md`
- `docs/design/phase-40.0-milestone-5-pilot-product-lock.md`
