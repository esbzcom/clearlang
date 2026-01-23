# Technical Decisions

This document records key implementation decisions for ClearLang.

## Phase 15.1 - Unsigned integer overflow policy
Question: What should the default overflow behavior be for `U64`/`U128`/`U256`?

Options:
- Wrap (modulo 2^N).
- Checked (trap or fail on overflow).
- Saturating (clamp to min/max).

Decision: Checked by default, with explicit `wrap_*`/`sat_*` operations for opt-in behavior.

Reason: Matches the "provably correct" and "crypto-focused" principles, keeps runtime and proofs aligned, and makes overflow diagnostics explicit and AI-friendly.

## Phase 15.1 - Unsigned literal syntax
Question: How should unsigned integer literals be expressed?

Options:
- Suffix literals (e.g., `42u64`, `42u128`, `42u256`).
- Postpone literals and only allow unsigned values via params/returns.
- Contextual typing with explicit casts as a fallback.

Decision: Contextual typing (bare literals adopt the expected unsigned type) with explicit casts such as `U64(42)`/`U128(42)`/`U256(42)` when no context exists.

Reason: Reduces syntax burden for users while keeping unambiguous escapes and range checks at cast boundaries.

## Phase 15.1 - Proof/VC handling for `U64`
Question: How should `U64` be represented in verification conditions?

Options:
- Treat `U64` as an unbounded Int (tech debt).
- Treat `U64` as a bounded Int with explicit no-overflow constraints.
- Block VC generation for functions that use `U64` until SMT work in Phase 16.5.

Decision: Model `U64` as a bounded Int with explicit no-overflow constraints; keep runtime/codegen native using Wasm `i64`.

Reason: Preserves proof usefulness without delaying the feature, while keeping runtime behavior deterministic and aligned with checked-overflow semantics.
