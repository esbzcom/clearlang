# Crypto Proof Limitations

This document captures what ClearLang proofs do *not* guarantee for cryptographic primitives today, and what options exist to migrate toward stronger guarantees.

## Scope

- Applies to: `std::crypto::{hash,hmac,verify}`, `std::bytes::eq_ct`, and bitwise/shift operations used by crypto-heavy code.
- Applies to: SMT/VC proofs (`--emit-vcs`) and any downstream tooling that consumes them.

## Current Limitations

1) **Uninterpreted crypto functions**
   - The SMT layer models `hash`, `hmac`, and `verify` as *uninterpreted functions*.
   - Proofs cannot derive cryptographic properties (collision resistance, preimage resistance, PRF security, unforgeability).
   - Equality of crypto outputs is only available if asserted directly in a contract.

2) **Opaque signature verification**
   - `std::crypto::verify` returns an unconstrained `Bool` in SMT.
   - Proofs cannot infer that a `true` result implies authenticity; it is an external assumption.

3) **No constant-time / side-channel model**
   - `std::bytes::eq_ct` is modeled only as a pure boolean equality helper.
   - Timing/leakage properties are out of scope for SMT proofs.

4) **Unsigned arithmetic ≠ bit-precise arithmetic**
   - The SMT encoding uses unbounded `Int` for unsigned arithmetic.
   - Bitwise and shift operators are modeled as uninterpreted functions.
   - Proofs cannot reason about overflow behavior or exact bit patterns without extra assumptions.

5) **Runtime errors are not part of proofs**
   - Runtime diagnostics (e.g., R006-R008) are enforced by execution and covered by tests, not by SMT.

## Migration Options

1) **Add explicit axioms for specific properties**
   - Example: treat `hash` as injective over a constrained domain, or assert `verify` implies ownership under a known key model.
   - This improves proof strength but must be carefully scoped to avoid unsoundness.

2) **Use bitvector SMT encoding for `U64`**
   - Replace `Int` modeling with SMT bitvectors for unsigned operations and bitwise/shift semantics.
   - This enables overflow/bit-pattern reasoning at the cost of solver complexity and migration effort.

3) **Introduce proof-aware wrappers**
   - Add builtins like `std::crypto::hash_assumed_eq` or `std::crypto::verify_assumed` with explicit contract obligations.
   - Keeps assumptions visible in source and proof artifacts.

4) **External attestation / proof linkage**
   - Combine SMT proofs with external attestations for crypto (e.g., signed verification artifacts, chain registries).
   - This aligns with Phase 16.7's attestation design to connect runtime evidence to proofs.

5) **Hybrid assurance in CI**
   - Use runtime tests for cryptographic correctness and SMT proofs for structural correctness, preserving deterministic behavior.
   - This is the recommended baseline until richer proof infrastructure is available.

## Practical Guidance

- VC artifacts expose assumption boundaries directly: `unsigned.int_model`, `bitwise.uninterpreted`, and `crypto.uninterpreted` under `assumptions.items`.
- If a property depends on cryptographic strength, document it as an assumption in the contract or proof notes.
- Avoid proving security-critical claims (e.g., "only signer can authorize") without an explicit attestation layer.
- For arithmetic-level invariants that rely on overflow/bit patterns, do not rely on SMT unless bitvector encoding is enabled.
