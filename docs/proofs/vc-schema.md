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
    "function": "pick",
    "vc_id": "vc:0",
    "pre":   { "ast": "true", "smt2": "true" },
    "post":  { "ast": "result == result", "smt2": "(= result result)" },
    "vc":    { "smt2": "; Option/Result variants use (tag, payload_lo, payload_hi)\n(declare-fun cl.variant.tag (Int) Int)\n(declare-fun cl.variant.payload_lo (Int) Int)\n(declare-fun cl.variant.payload_hi (Int) Int)\n(declare-fun cl.option.mk (Int Int Int) Int)\n(=> true (= (cl.option.mk 1 (+ (let ((cl_match$0 opt)) (ite (= (cl.variant.tag cl_match$0) 1) (let ((__coalesce_tmp92 (cl.variant.payload_lo cl_match$0))) __coalesce_tmp92) 7)) 1) 0) (cl.option.mk 1 (+ (let ((cl_match$1 opt)) (ite (= (cl.variant.tag cl_match$1) 1) (let ((__coalesce_tmp92 (cl.variant.payload_lo cl_match$1))) __coalesce_tmp92) 7)) 1) 0)))" },
    "status": "generated"
  }
]
```
VC outputs now declare cl.variant.tag/payload accessors and cl.option.mk/cl.result.mk so the tagged layout is explicit to solvers and tooling.

