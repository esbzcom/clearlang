# Phase 30.6.4 - Campaign Evidence Binding Lock

`clg contract release` packages `clg.contract-campaign-evidence-index.v1` alongside the
campaign report. The canonical index binds the report digest, deterministic replay metadata,
per-case arguments, and the four simulator inputs/outputs for every case: arguments, initial
state, final state, and trace. Each file is named relative to the bundle-local campaign trace
directory and recorded with its SHA-256 digest.

`clg contract verify-release` validates the index format, report binding, case order, replay and
argument equality, required file set, and every file digest. It accepts only single-component
relative names, rejects traversal and substituted paths, and rejects symbolic links or resolved
trace directories outside the bundle. A missing, failed, malformed, or altered campaign evidence
entry fails release verification.
