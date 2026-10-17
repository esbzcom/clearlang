# Phase 44.2.2 - Second-Environment Public Verifier Walkthrough

Date: 2026-10-16
Status: Passed locally in the second clean environment
Clean clone commit: `928e9ab3822562493abe9291ee00fdad92c0e166`

## Reproduction

The second clean clone created for 44.1.3 at
`C:\Users\felto\AppData\Local\Temp\clg-m5-44.1.3-928e9ab-second` was reused without source
changes. Its initial clean-clone closure rehearsal had already passed. With
`CARGO_NET_OFFLINE=true`, it then ran:

```text
cargo test --offline -p clg-cli --test cli_it basic::published_offline_verifier_walkthrough_is_current_and_reproducible_without_network -- --exact
```

Exit status was `0`. The non-secret result transcript is retained in
`phase-44.2.2-public-verifier-walkthrough.log`; the retained versioned transcript SHA-256 is
`0cc5ee3db89e97a45dc77dc4fc5dd23d5b449febd45c7e368309ae3034827406`.

## What the Reproduction Checks

The exercised walkthrough test creates an independent receiving directory, copies only the
reference release evidence, an explicit public release key, and the verifier executable, then
removes the source file outside that directory. It runs the published
`--json-errors verify-release --bundle ... --pubkey ...` command with ambient HTTP(S) proxy
variables set to a local non-accepting trap. The test passes only when verification succeeds,
the trap receives no connection, and the retained verifier result is canonical
`clg.contract-release-verifier-result.v1` JSON.

No private key, wallet, RPC URL, credential, test fixture directory, or ephemeral release artifact
was retained. This is reproducible local evidence for the public offline verifier walkthrough; it
does not claim a live target action or production readiness.
