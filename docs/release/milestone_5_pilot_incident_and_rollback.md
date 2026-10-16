# Milestone 5 Pilot Incident and Rollback Guide

## Scope and immediate rule

This guide applies to the **locally validated, testnet-only pilot** only. It must not be described
as generic EVM support, remote-CI certification, mainnet readiness, or custody safety.

For every incident: stop new submissions, preserve non-secret evidence byte-for-byte, record the
exact stop point and stable diagnostic, and do not edit a report, receipt, release bundle, or
verifier result to recover a passing state. Evidence preservation is local forensic work;
chain-side remediation is a separate explicit operator action, never an inferred ClearLang action.

## Failed release verification

1. Stop before any target submission or promotion.
2. Retain the bundle directory, verifier package names/digests, explicit release public key,
   verifier result, and failed local precheck report if present.
3. Record `C140` or the verifier's non-secret rejection and compare bundle, profile, schema, key,
   and capability inputs with the reviewed plan.
4. Do not substitute a nearby verifier, public key, profile, or bundle member. Repair only the
   identified input under the compatibility policy, then rerun local verification.

There is no chain-side remediation because no submission is authorized after failed verification.

## Wrong-chain submission attempt

1. Stop before signing or submitting when chain ID is not `11155111` or the profile is not
   `clg.evm-sepolia-pilot.v1`.
2. Preserve the reviewed plan and sanitized preflight/error output; never preserve signer material,
   RPC credentials, or credential-bearing URLs.
3. If a transaction was sent, record only non-secret transaction/receipt identifiers, notify the
   release owner, and suspend further use of that signer.
4. Chain-side remediation is the deploy operator's explicit testnet procedure. It does not make
   the attempt valid pilot evidence or authorize a replacement run without a new reviewed plan.

## Unexpected receipt or target behavior

1. Stop when a receipt, address, status, caller, event, request digest, or artifact identity
   differs from the reviewed plan.
2. Preserve the release bundle, preflight, sanitized observations, request/receipt files, verifier
   result, and local reports without overwriting earlier bytes.
3. Do not retry with altered nonce, fee, sender, artifact, ABI, schema, or target address merely
   to obtain an expected result.
4. Chain-side action is an external operator decision. Evidence may diagnose a mismatch but does
   not reverse, replace, or finalize a transaction.

## Leaked or suspected leaked test key

1. Stop all uses of the affected signer. Do not place its contents in an issue, fixture, report,
   terminal log, or support request.
2. Preserve non-secret evidence only: key identifier, affected plan/release identifiers, and
   transaction identifiers.
3. Retire the test key and use a newly generated test-only key only after review. Do not claim a
   signature rotation repaired old evidence.
4. Any testnet-fund or deployment response is an explicit external operator procedure and is
   retained only as external context.

## Discovered artifact or evidence mismatch

1. Stop verification, submission, and promotion; preserve expected and observed non-secret
   digests, names, and reports.
2. Identify whether the mismatch is in source, proof, schema, ABI, EVM artifact, campaign,
   signature/assurance evidence, bundle, verifier package, or receipt binding.
3. Do not hand-edit an artifact, digest, projection, or report. Restore the exact approved input
   or create a newly reviewed compatible evidence set and rerun the full local chain.
4. For a published verifier artifact, follow Phase 43.0.3's named-artifact withdrawal/rollback
   procedure; never select an unversioned or nearest release.

## Evidence retention and closure

Retain canonical report bytes, bundle/verifier identifiers, public keys, sanitized observations,
and stable diagnostics. Do not retain private keys, wallet files, credential-bearing RPC URLs,
tokens, or raw environment dumps. Resume testnet activity only after a new local validation
result—not remote CI—supports the reviewed plan.

## References

- [Milestone 5 pilot product contract](milestone_5_pilot_product_contract.md)
- [Pilot operator runbook](pilot-operator-runbook.md)
- [Evidence compatibility and rollback lock](../design/phase-40.2.2-local-evidence-compatibility-and-rollback-lock.md)
- [Verifier lifecycle and rollback lock](../design/phase-43.0.3-verifier-lifecycle-and-rollback-lock.md)
