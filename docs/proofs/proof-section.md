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

### Function Entry

Each element in `functions` is a CBOR map with keys:

| Key        | Type                   | Description                                           |
|------------|------------------------|-------------------------------------------------------|
| `name`     | text                   | Function identifier (matches Wasm export when present).|
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

The optional `proof` map contains `{ format: text, bytes: byte-string }` where `bytes`
are the raw proof artifact (Alethe, LFSC, etc.). Phase 6.5 stores `null` / omits this
field; it is reserved for future phases.

`refinements` is a map with a single `premises` array. Each premise mirrors the VC JSON
shape: `{ id, alias, binder, substitution: { ast, smt2 }, predicate: { ast, smt2 }, attachment }`,
where `attachment` is tagged with `kind` and carries `detail` for `param`, `return`, or
`flow` sites. See `docs/proofs/vc-schema.md` for the full refinement attachment schema.

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
  "scope": "module" | "proofs" | "both"
}
```

- `scope` selects which components are attested (`module`, `proofs`, or both).
- Hex strings are lowercase, zero-padded to 64 chars.
- Timestamps use UTC with seconds precision.

The signature itself is stored externally (e.g., `--sig-out <file>`). Verification rehashes
the payload, checks the signature, then inspects the embedded `clearlang.proof` section.

## Backwards Compatibility

- v1 sections remain parseable; v2 adds optional `refinements` on VC entries.
- Future versions must bump `version` and keep earlier versions parseable.
- New fields should be optional to avoid breaking existing tooling.
- When proofs become mandatory, the `proof` field will be required for `status = "proved"`.

## Open Questions / Follow-ups

- Key formats (PEM/JSON) for signing are defined in CLI docs, not here.
- The project may adopt additional hash algorithms alongside SHA-256; they would appear as
  additional fields (e.g., `module_hashes: { sha256: ..., blake3: ... }`).
- Proof compression (e.g., gzip) can be added via a `encoding` field in the proof map.
