# Verification Conditions (VC) — JSON Schema (Draft)

Principles
- AI‑Friendly: stable field names, explicit codes, no implicit context.
- Deterministic: canonical ordering; all positions use byte offsets; optional line:col adjuncts.
- Extensible: `version` and `extra` fields reserved.

Top-Level
- File: JSON array of VC objects.
- Order: ascending by `function` then `vc_id` (string compare).

VC Object
- `version`: number (1)
- `function`: string (fully-qualified name)
- `vc_id`: string (stable per function; `vc:0`, `vc:1`, …)
- `pre`: object — AST string and SMT2
  - `ast`: string (surface syntax of precondition)
  - `smt2`: string (SMT-LIB2 encoding; QF_LIA + Bool)
- `post`: object — AST string and SMT2
  - `ast`: string (surface syntax of postcondition)
  - `smt2`: string
- `vc`: object — the VC implication P ⇒ Q[e/result]
  - `smt2`: string
- `status`: string — one of `generated|proved|failed`
- `positions`: object — optional source mapping
  - `file`: string (path)
  - `pre_start`: number (byte offset)
  - `pre_end`: number (byte offset)
  - `post_start`: number
  - `post_end`: number
- `extra`: object — reserved for tool metadata

Example
```
[
  {
    "version": 1,
    "function": "inc",
    "vc_id": "vc:0",
    "pre": { "ast": "x >= 0", "smt2": "(>= x 0)" },
    "post": { "ast": "result >= x", "smt2": "(>= result x)" },
    "vc":   { "smt2": "(=> (>= x 0) (>= (+ x 1) x))" },
    "status": "generated",
    "positions": { "file": "examples/inc.clear", "pre_start": 42, "pre_end": 49, "post_start": 64, "post_end": 75 },
    "extra": { "tool": "clg 0.1.0", "commit": "d46d03d" }
  }
]
```

CLI Contract
- `clg build file.clear --emit-vcs out.json` writes exactly the array above.
- No solver integration in this phase; `status` is always `generated`.
- Future: `--emit-proof proofs/` adds per-VC proof files keyed by `vc_id`.

Compatibility
- Stable for Phase 6; only additive changes allowed (new optional fields).

## Example: ADT sugar (experimental)
```
[
  {
    "version": 1,
    "function": "default_or_zero",
    "vc_id": "vc:0",
    "pre":   { "ast": "true", "smt2": "true" },
    "post":  { "ast": "result >= 0", "smt2": "(>= result 0)" },
    "vc":    { "smt2": "(=> true ; unsupported expr If { ... })" },
    "status": "generated"
  },
  {
    "version": 1,
    "function": "bump_when_positive",
    "vc_id": "vc:0",
    "pre":   { "ast": "true", "smt2": "true" },
    "post":  { "ast": "result == result", "smt2": "(= result result)" },
    "vc":    { "smt2": "(=> true (= ; unsupported expr If { ... Try { ... } } ; unsupported expr If { ... Try { ... } }))" },
    "status": "generated"
  }
]
```
Current SMT strings still contain placeholder comments because Option/Result lowering and SMT encoding are not final. Once the lowering work (Phase 6.6 follow-up) lands, these placeholders will be replaced by tagged encodings and the examples above will be updated with canonical snapshots.
