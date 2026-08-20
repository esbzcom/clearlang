# Phase 30.3.4 - EVM Target Conformance Fixture Lock

Date: 2026-08-20
Status: Locked
Owner: target-runtime-owner

## Fixture Matrix

The target conformance suite executes the emitted stateful scalar bytecode through a deterministic
London-opcode harness and validates the explicit JSON-RPC adapter against a strict in-process
EVM-compatible endpoint fixture. It covers constructor ABI words, dispatch, `SLOAD`/`SSTORE`,
ABI results, `LOG1`, EIP-155 submission, receipt confirmation, and canonical receipt evidence.

The scalar matrix covers `Bool`, `U8`, `U64`, `U128`, and signed `Int`, including non-trivial
128-bit values and negative two's-complement words. It validates field-ID-derived storage slots
before and after a state transition.

## Compatibility Boundary

This is compatibility evidence for the locked `clg.evm-stateful-scalar.v1` opcode and JSON-RPC
surface. It does not claim general client, network, EIP-1559, proxy, dynamic ABI, or external-call
compatibility. Every omitted feature remains rejected before artifact emission or submission.

## Acceptance Commands

- `cargo test -p clg-cli --lib commands::build::evm_artifact::tests::stateful_scalar_profile_conforms_for_every_supported_storage_word_type`
- `cargo test -p clg-cli --lib commands::target::tests`
- `cargo test -p clg-cli --test cli_it target_`
