# Phase 30.4.2 - Contract Crypto Fixture Policy

Date: 2026-08-09
Status: Locked
Owner: crypto-and-proof-owner

There are no authorization or signature contract APIs in the selected 30.4.0 surface. Therefore
this phase adds no synthetic malformed-key, signature, domain, or replay fixture that could imply
such support. Any future fixture set must arrive with a primitive-specific API, semantic model or
independent attestation, deterministic valid and malformed inputs, domain-separation rules, and
replay identity rules. Until then, a contract cannot claim authorization/signature verification.

## References

- `docs/design/phase-30.4.0-contract-crypto-surface-lock.md`
- `docs/design/phase-30.4.1-contract-crypto-assurance-boundary.md`
