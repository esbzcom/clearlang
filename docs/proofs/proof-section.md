# `clearlang.proof` Custom Section (v2)

This document specifies the binary layout, hashing rules, and signing payloads for the
`clearlang.proof` custom section introduced in Phase 6.5. The goal is to keep proof
artifacts portable, deterministic, and easy to verify offline.

## Design Goals

- **Deterministic**: identical inputs yield identical section bytes (stable ordering,
  canonical encoding).
- **Traceable**: every verification condition (VC) emitted via `--emit-vcs` is linked to
  its Wasm module by `vc_id` and function name.
- **Extensible**: future versions can extend the schema without breaking existing
  consumers.
- **AI-friendly**: section contents mirror the JSON schema used for VCs so tools can map
  diagnostics back to source.

## Binary Layout

The section uses the standard Wasm custom section encoding:

```
0x00            ; section id for custom
uleb( name_len )
"clearlang.proof" (UTF-8)
uleb( payload_len )
payload_bytes
```

`payload_bytes` are encoded using [CBOR](https://www.rfc-editor.org/rfc/rfc8949) with the
canonical encoding profile. Canonical CBOR gives us deterministic ordering, compact
representation, and a straightforward path to hashing/signing.

### Top-Level Map

The payload is a CBOR map with the following fields:

| Key             | Type                     | Description                                           |
|-----------------|--------------------------|-------------------------------------------------------|
| `version`       | unsigned integer         | Section version (current: 2).                         |
| `generated_by`  | text                     | Tool metadata (`clg-cli/<version>`).                  |
| `module_hash`   | byte string (32 bytes)   | SHA-256 of the full Wasm module bytes.                |
| `proofs_hash`   | byte string (32 bytes)   | SHA-256 of concatenated VC payloads (defined below).  |
| `functions`     | array of function maps   | One entry per function carrying contracts/VCs.        |
| `assurance`     | optional assurance map   | Module-level assurance tier + stable `L0`-`L3` labels. |

### Function Entry

Each element in `functions` is a CBOR map with keys:

| Key        | Type                   | Description                                           |
|------------|------------------------|-------------------------------------------------------|
| `name`     | text                   | Function identifier (matches Wasm export when present).|
| `canonical_name` | optional text     | Canonical unshortened function name when `name` was shortened. |
| `requires` | optional proof expr map | Combined `require` clauses as a `ProofExpr`.            |
| `ensures`  | array of ensure maps   | One per `ensure` clause.                              |
| `vcs`      | array of VC maps       | Verification conditions emitted for the function.     |

`requires` may be omitted when the precondition is `true` (no clauses). Both `requires` and
`ensures` use the `ProofExpr` shape (`{ ast, smt2, span? }`).

### VC Entry

Verification condition maps carry the data required to re-run or validate proofs:

| Key           | Type                     | Description                                           |
|---------------|--------------------------|-------------------------------------------------------|
| `vc_id`       | text                     | Stable identifier (`vc:<index>` or `mut_pre:<...>`).  |
| `pre`         | map                      | `{ ast: text, smt2: text, span: { start, end }? }`.    |
| `post`        | map                      | Same shape as `pre`.                                  |
| `vc_smt2`     | text                     | Full SMT-LIB 2 obligation (`(=> pre post)` form).     |
| `status`      | text                     | `"generated"`, `"proved"`, or `"failed"`.           |
| `proof`       | optional proof map       | Present once solver integration lands.                |
| `refinements` | optional refinements map | Refinement premises for this VC (v2+).               |
| `assumptions` | optional assumptions map | Explicit `assumed` proof-model boundaries for this VC (v2+). |
| `assurance`   | optional assurance map   | VC-level assurance tier + stable `L0`-`L3` labels.   |

The optional `proof` map contains `{ format: text, bytes: byte-string }` where `bytes`
are the raw proof artifact (Alethe, LFSC, etc.). Phase 6.5 stores `null` / omits this
field; it is reserved for future phases.

`refinements` is a map with a single `premises` array. Each premise mirrors the VC JSON
shape: `{ id, alias, binder, substitution: { ast, smt2 }, predicate: { ast, smt2 }, attachment }`,
where `attachment` is tagged with `kind` and carries `detail` for `param`, `return`, or
`flow` sites. See `docs/proofs/vc-schema.md` for the full refinement attachment schema.

`assumptions` is a map with a single `items` array. Each item mirrors VC JSON:
`{ id, category, status, message, symbols[], intrinsic_levels[]? }`, where `status` is currently `assumed`
and `symbols` lists the exact touched operators/types/intrinsics that triggered the boundary.
Current categories include `unsigned`, `bitwise`, `crypto`, `primitive`, and `external`.
For `crypto.uninterpreted`, `intrinsic_levels` records deterministic per-intrinsic assurance entries
(`intrinsic`, `tier`, `label`) aligned to `symbols`.
In strict compiler mode, assumption labels are validated first (`C031`), and any remaining assumption item then fails the strict language profile gate (`C033`).

`assurance` is a map with:
- `tier`: current tier (`L0` for VCs/modules with assumption boundaries, `L1` otherwise in current implementation).
- `label`: tier name (`assumed`, `checked core`, `verified module`, `verified package profile`).
- `levels`: stable map of all tier labels:
  - `L0`: `assumed`
  - `L1`: `checked core`
  - `L2`: `verified module`
  - `L3`: `verified package profile`

## Hashing Rules

Proof packaging relies on two SHA-256 digests:

1. **Module hash (`module_hash`)**: computed over the Wasm bytes with the `module_hash` field
   zeroed. We first emit the section with zeroes, hash the full module, then rewrite the section
   with the resulting digest. Verifiers repeat the hash after zeroing the field to avoid
   self-referential cycles.
2. **Proofs hash (`proofs_hash`)**: computed over the concatenation of each VC entry's
   canonical CBOR encoding, ordered lexicographically by `(function.name, vc_id)`.

This separation lets verifiers detect mismatches between code and proof artifacts while
keeping the signature payload compact.

## Signing Payloads

The signing payload is a canonical JSON document encoded with
[JCS](https://www.rfc-editor.org/rfc/rfc8785) before hashing/signing. The module hash here
uses the same zeroed-field procedure described above. Shape:

```
{
  "module_hash": <hex string>,
  "proofs_hash": <hex string>,
  "toolchain": "clg-cli/<version>",
  "timestamp": "<RFC3339 UTC>",
  "scope": "module" | "proofs" | "both",
  "assurance": { "tier": "...", "label": "...", "levels": { "L0": "...", "L1": "...", "L2": "...", "L3": "..." } },
  "trust_anchors": { "lean_checker": "<version>", "coq_checker": "<version>" }
}
```

- `scope` selects which components are attested (`module`, `proofs`, or both).
- Hex strings are lowercase, zero-padded to 64 chars.
- Timestamps use UTC with seconds precision.
- `trust_anchors` is optional and only emitted when checker versions are pinned at build/sign time.
- `clg verify --verify-mode compile-time --trust-policy <FILE>` requires
  `trust_anchors` in the signature payload and checks exact version equality.

The signature itself is stored externally (e.g., `--sig-out <file>`). Verification rehashes
the payload, checks the signature, then inspects the embedded `clearlang.proof` section.

## Signed Assurance Manifest (Phase 19.5.1)

When `clg build` is run with `--sign`, the CLI now also emits a signed assurance manifest JSON
for audit/release workflows.

- Default output path is derived from `--sig-out`:
  - `out.sig.json` -> `out.assurance.json`
- Override with:
  - `--assurance-manifest-out <FILE>`

Manifest envelope shape:

```
{
  "schema_version": 1,
  "payload": {
    "format": "clg.assurance_manifest.v1",
    "generated_at": "<RFC3339 UTC>",
    "toolchain": {
      "name": "clg-cli/<version>",
      "fingerprint_sha256": "<sha256(toolchain-name)>"
    },
    "build": {
      "compiler_mode": "permissive|standard|strict",
      "proof_strict": true|false
    },
    "artifacts": {
      "module_hash": "<hex>",
      "proofs_hash": "<hex>"
    },
    "assurance": { "tier": "...", "label": "...", "levels": { ... } },
    "assumptions": { "total": <n>, "items": [...] },
    "dependency_trust_labels": [
      { "dependency": "<symbol>", "kind": "primitive|external", "label": "assumed" }
    ]
  },
  "signature": {
    "key_id": "<key-id>",
    "signature_format": "ed25519",
    "payload_hash": "<sha256(canonical payload json)>",
    "signature": "<ed25519-hex>"
  }
}
```

The manifest signature is computed over canonical JSON for `payload` only (same canonicalization
strategy as signature payload signing), keeping verification deterministic.

`clg verify --explain` can summarize these signed/embedded artifacts for humans, including:
- assurance tier label,
- checked-core vs assumed-boundary VC counts,
- per-boundary `why` reasons,
- dependency trust labels for primitive/external assumed surfaces.

## Backwards Compatibility

- v1 sections remain parseable; v2 adds optional `refinements`/`assumptions`/`assurance` on VC entries, optional top-level `assurance`, and optional `canonical_name` on function entries.
- Future versions must bump `version` and keep earlier versions parseable.
- New fields should be optional to avoid breaking existing tooling.
- When proofs become mandatory, the `proof` field will be required for `status = "proved"`.

## Open Questions / Follow-ups

- Key formats (PEM/JSON) for signing are defined in CLI docs, not here.
- The project may adopt additional hash algorithms alongside SHA-256; they would appear as
  additional fields (e.g., `module_hashes: { sha256: ..., blake3: ... }`).
- Proof compression (e.g., gzip) can be added via a `encoding` field in the proof map.
