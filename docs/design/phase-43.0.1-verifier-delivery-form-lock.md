# Phase 43.0.1: Independent verifier delivery-form lock

Date: 2026-10-15  
Status: Locked  
Owner: release-assurance-owner

## Decision

Milestone 5 distributes one dedicated native executable named `clg-contract-verifier`. It is the
supported independent-verifier form for the pilot. It will expose the same release-bundle
verification operation as `clg contract verify-release`, require only the documented bundle-local
inputs and approved public key, and run without a source checkout, Rust toolchain, RPC endpoint,
wallet, or private key.

An OCI/container image is **not** a supported verifier distribution for this milestone. It is
deferred rather than offered as a second implementation or a convenience alias.

## Rationale

The dedicated executable is the smallest independently runnable form for a non-Rust user. It
removes the source-checkout and Cargo requirements while preserving the existing local/offline
trust boundary. An OCI image would additionally require a container engine, an image acquisition
and digest-verification procedure, platform/container compatibility policy, and a second user
experience. For the bounded pilot it adds packaging surface but no new verification semantics.

This choice follows the product principles:

- **Simple for users:** one downloaded executable and one verification command.
- **AI-friendly:** one artifact name, one argument shape, and one result schema avoid
  container-versus-native drift.
- **Provably correct:** both CLI forms must call one verification core, so parsing, hashing,
  signature verification, profile checks, and diagnostics cannot diverge by packaging route.
- **Crypto-focused:** the user verifies a pinned executable artifact and its explicit trust input,
  not a mutable container tag or a container registry's reputation.

## One-core / one-schema rule

`clg-contract-verifier` must be a thin binary front end over the single release-bundle verifier
library entrypoint. `clg contract verify-release` must use that same entrypoint. Neither binary
may copy or fork artifact parsing, path validation, digest computation, signature/assurance
verification, profile checks, trust-policy handling, exit-code mapping, or result rendering.

The two forms must produce byte-identical machine-readable results and equivalent human results
for identical inputs. The distributable binary may add only its executable identity/version
metadata outside the canonical verifier result object, as defined in 43.0.2. It must not add a
second acceptance decision, policy, or schema.

## Scope and deferred work

This decision authorizes 43.1 to extract the common entrypoint and package the dedicated binary.
It does not yet select supported operating systems, processor architectures, artifact filenames,
checksum/signature envelopes, trust-key provisioning, compatibility ranges, or final output
schema; 43.0.2 locks those details before packaging.

An OCI image may be reconsidered only in a later design lock after it can demonstrate all of the
following:

1. it invokes the exact same verifier core and canonical result schema;
2. its image digest, signature/provenance, base-image policy, and offline acquisition workflow are
   fully specified;
3. it does not become a required runtime dependency for users of the native verifier; and
4. it adds a concrete supported-user benefit that outweighs the additional trust and support
   surface.

Until then, a container image, wrapper script, hosted verifier, browser verifier, or source-build
path is not a supported independent-verifier distribution form.

## Acceptance criteria for the implementation phase

43.1 is complete only when a clean-directory black-box test shows that the dedicated executable
and `clg contract verify-release` invoke the shared core, return the same canonical result for the
same bundle/public key, and fail identically for each supported tamper class without a network
attempt. A differently implemented container or wrapper cannot satisfy this criterion.

## References

- `docs/design/phase-30.6.1-contract-release-verification-lock.md`
- `docs/design/phase-40.0-milestone-5-pilot-product-lock.md`
- `docs/todo/milestone_5_roadmap.md`
