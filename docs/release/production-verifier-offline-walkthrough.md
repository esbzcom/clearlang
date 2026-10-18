# Production Candidate Verifier Walkthrough

Status: Offline independent-verifier procedure. It validates only a future
distributed production-verifier package and its supplied candidate evidence. It
does not contact a network or authorize any Safe, signing, deployment,
invocation, or production launch.

## Start With Received Artifacts

Work in a new, local directory that contains only:

- the distributed production-verifier package, its signed manifest, checksums,
  and detached signature;
- the candidate release/evidence package, retaining its bundle-local layout;
- the public release verification key; and
- the expected distribution public key and key ID from a separately trusted
  channel.

Do not use a ClearLang source checkout, Cargo, package registry, wallet,
private key, seed phrase, endpoint, or account. Validate the package manifest,
detached signature, and archive/executable checksums using the procedure in
[`contract-verifier-offline-walkthrough.md`](contract-verifier-offline-walkthrough.md).
The distribution key is public trust input; it is not obtained from the
received package.

## Required Future Production Result

Only after those package checks pass, execute the received verifier locally:

```powershell
./clg-contract-verifier --json-errors verify-release --bundle <BUNDLE> --pubkey <PUBLIC_RELEASE_KEY>
```

Acceptance of a future production candidate requires a canonical verifier
result that states all of the following exactly:

- a versioned, explicitly supported production verifier format;
- `target_profile: clg.evm-ethereum-mainnet-safe-registry.v1`;
- chain ID `1` bound in the package evidence;
- an exact release bundle digest and a `verified` result; and
- no secret, endpoint, wallet, machine path, or live-chain observation.

Retain the exact verifier package, manifest/signature/checksum inputs, public
trust inputs, candidate package, verifier JSON result, and their SHA-256
digests in the approved evidence store. A verifier result is evidence only; it
does not authorize a Safe request, signing, or a target action.

## Current Expected Result: Stop

There is no distributed production verifier in this milestone yet. The current
`clg-contract-verifier` is restricted to
`clg.evm-sepolia-pilot.v1`. It must reject a mainnet Safe-registry candidate as
an unsupported profile/version. Treat that rejection as the expected current
result, preserve the non-secret report, and stop. Do not substitute a source
build, alter the profile, reuse a Sepolia result, or weaken the trusted-key
procedure to obtain a passing result.

The production candidate remains blocked until a separately reviewed and
packaged production verifier exists and passes the clean-directory fixture.

## Failure Handling

Stop and retain evidence if a package file is missing, its signature/checksum
does not match, a key ID differs, the verifier output is malformed, a schema or
profile is unsupported, or the result is anything other than the exact future
production result above. Do not retry with another key, endpoint, target,
profile, or modified artifact. Report the digest and non-secret diagnostic to
the release owner and independent-review process.

## Boundary

This walkthrough does not claim target state, endpoint honesty, chain finality,
Safe authority, testnet-soak completion, independent-review disposition, or
production readiness.
