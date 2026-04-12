# Phase 25.4.2 - Tool-Generated Deterministic `clg.lock.json`

## Status
Design lock + implementation record for `25.4.2` in `docs/TODO.md`.

## Goal
Keep `clg.lock.json` as a tool-generated deterministic lockfile that records exact package identity and resolved graph identity.

## Design Principles Check
- Simple for users: users edit `clg.project.json`; tooling owns `clg.lock.json`.
- AI-friendly: lockfile and resolved-graph outputs are deterministic and replayable byte-for-byte.
- Provably correct: strict build/release consume lockfile pins as source of truth.
- Crypto-focused: lockfile pins include exact digests for audit-grade supply-chain identity.

## Contract
1. `clg.lock.json` is generated/updated only by:
   - `clg pkg lock --generate`
   - `clg pkg lock --update`
   - `clg release` lock stage
2. Lockfile entries are exact pins per package:
   - `id` (`name@version`)
   - `name`
   - `version`
   - `digest`
   - `abi_id`
   - resolved dependency IDs
3. Deterministic ordering/canonicalization is required:
   - stable root/dependency/package ordering
   - canonical JSON bytes + deterministic hash reporting
4. `clg.resolved-graph.json` is resolver identity evidence and must match lockfile package/dependency identity.
5. `clg.resolved-graph.sha256` must match canonical resolved-graph bytes.

## Evidence
- lockfile/resolved-graph replay and hash determinism tests:
  - `crates/cli/tests/cli_it/pkg_lock/core.rs`
- explicit exact-pin + lock/graph identity parity coverage:
  - `pkg_lock_generate_emits_exact_pins_and_matching_resolved_graph_identity`

## References
- `docs/design/phase-22.0.3-lockfile-v1.md`
- `crates/cli/src/commands/pkg/artifacts.rs`
- `crates/cli/src/commands/pkg/lock_command.rs`
- `crates/cli/tests/cli_it/pkg_lock/core.rs`
- `docs/TODO.md`
