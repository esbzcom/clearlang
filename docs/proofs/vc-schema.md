# Verification Conditions (VC) JSON Schema (Draft)

Principles
- AI-friendly: stable field names, explicit codes, no implicit context.
- Deterministic: canonical ordering; all positions use byte offsets; optional line:col adjuncts.
- Extensible: `version` and `extra` fields reserved for forwards compatibility.

Top-Level
- File: JSON array of VC objects.
- Order: ascending by `function` then `vc_id` (string compare).

VC Object (v2)
- `version`: number (2)
- `function`: string (fully-qualified name)
- `vc_id`: string (stable per function; `vc:0`, `vc:1`, ...)
- `pre`: object - AST string and SMT2
  - `ast`: string (surface syntax of precondition)
  - `smt2`: string (SMT-LIB2 encoding; QF_LIA + Bool)
- `post`: object - AST string and SMT2
  - `ast`: string (surface syntax of postcondition)
  - `smt2`: string
- `vc`: object - the VC implication `pre => post`
  - `smt2`: string
- `status`: string - one of `generated|proved|failed`
- `positions`: object - optional source mapping
  - `file`: string (path)
  - `pre_start`: number (byte offset)
  - `pre_end`: number (byte offset)
  - `post_start`: number
  - `post_end`: number
- `refinements`: object - optional refinement premises (v2+)
  - `premises`: array of refinement premise objects (may be empty)
- `extra`: object - reserved for tool metadata

Refinement Premise
- `id`: string (stable within VC; `ref:0`, `ref:1`, ...)
- `alias`: string (refined alias name, e.g., `Nat`)
- `binder`: string (alias binder name, e.g., `n`)
- `substitution`: object - expression substituted for the binder
  - `ast`: string
  - `smt2`: string
- `predicate`: object - predicate after substitution
  - `ast`: string
  - `smt2`: string
- `attachment`: object - where the premise came from
  - `kind`: string - `param|return|flow`
  - `detail`: object - kind-specific fields

Attachment Details (kind)
- `param`: `detail` includes `param` (name)
- `return`: `detail` includes `result` (name; usually `result`)
- `flow`: `detail` includes `flow_kind` and optional fields
  - `flow_kind`: `let|call_arg|call_return|match_binder|if_let_binder|coalesce|try|loop_invariant|rewrap|branch_join`
  - `name`: local/binder name (when applicable)
  - `callee`: string (for call_* kinds)
  - `arg_index`: number (for call_arg)
  - `variant`: string (for match/if_let binder source)
  - `arm`: number (for match branches)

Notes
- `refinements.premises` are trace data; `pre`/`post` and `vc.smt2` include the substituted predicates.
- Consumers that do not understand refinements can ignore `refinements` and rely on `vc.smt2`.

Example
```json
[
  {
    "version": 2,
    "function": "inc_nat",
    "vc_id": "vc:0",
    "pre": { "ast": "x >= 0", "smt2": "(>= x 0)" },
    "post": { "ast": "result >= 0", "smt2": "(>= result 0)" },
    "vc": { "smt2": "(=> (>= x 0) (>= (+ x 1) 0))" },
    "refinements": {
      "premises": [
        {
          "id": "ref:0",
          "alias": "Nat",
          "binder": "n",
          "substitution": { "ast": "x", "smt2": "x" },
          "predicate": { "ast": "x >= 0", "smt2": "(>= x 0)" },
          "attachment": { "kind": "param", "detail": { "param": "x" } }
        },
        {
          "id": "ref:1",
          "alias": "Nat",
          "binder": "n",
          "substitution": { "ast": "result", "smt2": "result" },
          "predicate": { "ast": "result >= 0", "smt2": "(>= result 0)" },
          "attachment": { "kind": "return", "detail": { "result": "result" } }
        }
      ]
    },
    "status": "generated"
  }
]
```

Fixtures
- `docs/proofs/fixtures/refinement-basic.vc.json` (refinement premises only).
- `docs/proofs/fixtures/refinement-contracts-loops.vc.json` (refinements + require/ensure + loop VCs).
- `docs/proofs/fixtures/refinement-call-site.vc.json` (refinements + call-site obligations + require/ensure).

CLI Contract
- `clg build file.clear --emit-vcs out.json` writes exactly the array above.
- No solver integration in this phase; `status` is always `generated`.
- Future: `--emit-proof proofs/` adds per-VC proof files keyed by `vc_id`.

Versioning and Compatibility
- `version` is required. Consumers should handle v1 and v2 explicitly.
- v1 fields are unchanged; v2 adds the optional `refinements` object.
- Producers must bump `version` for any non-additive change.
- Consumers may ignore unknown fields, but should reject unknown `version` values.

## Example: ADT sugar
```json
[
  {
    "version": 2,
    "function": "pick",
    "vc_id": "vc:0",
    "pre": { "ast": "true", "smt2": "true" },
    "post": { "ast": "result == result", "smt2": "(= result result)" },
    "vc": { "smt2": "; Option/Result variants use (tag, payload_lo, payload_hi)\n(declare-fun cl.variant.tag (Int) Int)\n(declare-fun cl.variant.payload_lo (Int) Int)\n(declare-fun cl.variant.payload_hi (Int) Int)\n(declare-fun cl.option.mk (Int Int Int) Int)\n(=> true (= (cl.option.mk 1 (+ (let ((cl_match$0 opt)) (ite (= (cl.variant.tag cl_match$0) 1) (let ((__coalesce_tmp92 (cl.variant.payload_lo cl_match$0))) __coalesce_tmp92) 7)) 1) 0) (cl.option.mk 1 (+ (let ((cl_match$1 opt)) (ite (= (cl.variant.tag cl_match$1) 1) (let ((__coalesce_tmp92 (cl.variant.payload_lo cl_match$1))) __coalesce_tmp92) 7)) 1) 0)))" },
    "status": "generated"
  }
]
```
VC outputs declare cl.variant.tag/payload accessors and cl.option.mk/cl.result.mk so the tagged layout is explicit to solvers and tooling.
