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
- `canonical_function`: string - optional canonical unshortened function name when `function` was shortened for mangling/debug limits
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
- `assurance`: object - explicit assurance-tier metadata (`L0`-`L3`)
  - `tier`: string - current VC tier (`L0` or `L1` in current implementation)
  - `label`: string - tier label (`assumed`, `checked core`, `verified module`, `verified package profile`)
  - `levels`: object - stable tier label map
    - `L0`: `assumed`
    - `L1`: `checked core`
    - `L2`: `verified module`
    - `L3`: `verified package profile`
- `assurance_claim`: object - explicit trust-claim boundary marker for this artifact
  - `compiler_mode`: string (`permissive|standard|strict|unknown`)
  - `non_strict_evidence_only`: boolean (`true` in permissive/standard; `false` in strict)
  - `release_grade_trust`: boolean (`true` only in strict mode)
- `assumptions`: object - optional assumption boundaries (v2+)
  - `items`: array of assumption objects (may be empty)
- `positions`: object - optional source mapping
  - `file`: string (path)
  - `pre_start`: number (byte offset)
  - `pre_end`: number (byte offset)
  - `post_start`: number
  - `post_end`: number
- `diagnostics`: object - optional machine-readable diagnostics payload
  - `repair_hints`: object - optional minimal-repair suggestions for failed VC handling
    - `on_status`: string (`failed` in current implementation)
    - `items`: array of repair hint objects (may be empty)
  - `failure_slice`: object - optional failed-VC slice mapped to source spans
    - `on_status`: string (`failed`)
    - `vc_id`: string (same as parent VC id)
    - `clause_kind`: string (`ensure|require|invariant|variant|vc`)
    - `focus_span`: span object (`file`, `start`, `end`, `role`)
    - `related_spans`: array of span objects (may include pre/post spans)
  - `counterexample`: object - optional counterexample report payload
    - `on_status`: string (`failed`)
    - `state`: string (`solver_unavailable` in current implementation)
    - `format`: string (`clg.counterexample.v1`)
    - `reason`: string (why no concrete model is attached)
    - `focus_span`: span object (`file`, `start`, `end`, `role`)
    - `bindings`: array of binding objects (`symbol`, `value`)
  - `proof_context`: object - optional AI-oriented proof context bundle
    - `on_status`: string (`failed`)
    - `format`: string (`clg.proof_context.v1`)
    - `vc`: object - embedded VC context (`function`, `vc_id`, `status`, `pre`, `post`, `clause_kind`, `vc`)
    - `assumptions`: object - embedded assumption payload (`items`)
    - `model_snippet`: object - model envelope (`state`, `reason`, `bindings`)
    - `span_map`: object - source span map (`focus_role`, optional `pre`/`post` offsets)
- `refinements`: object - optional refinement premises (v2+)
  - `premises`: array of refinement premise objects (may be empty)
- `extra`: object - reserved for tool metadata

Repair Hint
- `kind`: string (`contract.ensure` | `contract.require` | `loop.invariant` | `loop.variant_nonneg` | `loop.variant_decrease`)
- `message`: string (short actionable guidance)
- `minimal_clause`: string (copyable clause candidate)

Span Object
- `file`: string (path)
- `start`: number (byte offset)
- `end`: number (byte offset)
- `role`: string (`pre|post`)

Counterexample Binding
- `symbol`: string (identifier/function symbol from VC goal expression)
- `value`: JSON value (`null` until solver/model integration is attached)

Proof Context Span
- `start`: number (byte offset)
- `end`: number (byte offset)

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

Assumption Boundary
- `id`: string (stable machine id, for example `unsigned.int_model`)
- `category`: string (`unsigned|bitwise|crypto|primitive|external`)
- `status`: string (`assumed` for current Phase 18.0.6.1 scope)
- `message`: string (human-readable boundary summary)
- `symbols`: array of strings (exact touched symbols, such as unsigned types, operators, or intrinsic names)
- `intrinsic_levels`: optional array (currently emitted for `crypto.uninterpreted`)
  - `intrinsic`: string (exact intrinsic symbol from `symbols`)
  - `tier`: string (currently `L0`)
  - `label`: string (currently `assumed`)

Current assumption IDs
- `unsigned.int_model`: unsigned values use SMT `Int` modeling with bounded-domain guards where available; overflow/bit-precise semantics are assumed.
- `bitwise.uninterpreted`: bitwise/shift operators and bitwise-sensitive `std::u64` intrinsics are encoded as uninterpreted SMT functions.
- `crypto.uninterpreted`: crypto/constant-time intrinsics are encoded as uninterpreted SMT functions; emitted items include deterministic per-intrinsic `intrinsic_levels` metadata.
- `primitive.unproved`: called std primitive intrinsics are outside current formal proof coverage and are treated as assumed dependencies.
- `external.dependency`: externally imported package dependencies are treated as assumed boundaries.

Assurance tier mapping (current)
- VCs with one or more assumption boundaries are emitted as `L0` (`assumed`).
- VCs without assumption boundaries are emitted as `L1` (`checked core`).
- `L2` and `L3` labels are emitted in `assurance.levels` for deterministic policy tooling; these levels are not claimed by current VC emission.

Notes
- `refinements.premises` are trace data; `pre`/`post` and `vc.smt2` include the substituted predicates.
- `assumptions.items` enumerates model boundaries required for that VC; omission means no selected boundary was detected for that VC.
- `assumptions.items[].intrinsic_levels` is emitted only for crypto boundaries and is ordered exactly like `symbols` for deterministic tool consumption.
- `assurance` is always emitted so release/policy tooling can consume tier labels deterministically.
- `assurance_claim.non_strict_evidence_only=true` explicitly marks non-strict outputs as developer evidence and not release-grade trust claims.
- `diagnostics.repair_hints` is advisory guidance for failed-VC workflows; it does not alter VC semantics or assurance tiers.
- `diagnostics.failure_slice` maps each VC obligation to exact source spans so failure reporting can point directly to user code.
- `diagnostics.counterexample` carries the model-report envelope; in current implementation `state` is `solver_unavailable` and `bindings[*].value` is `null` until external solver/model attachment is provided.
- `diagnostics.proof_context` bundles VC payload + assumptions + model snippet + span map for AI/tooling consumption; it is deterministic metadata and does not affect proof outcomes.
- Strict compiler mode (`--compiler-mode strict`) validates assumption labels (`C031`) and then fails closed on any remaining assumption boundary (`C033`), enforcing the verified-by-construction strict language profile.
- Consumers that do not understand refinements can ignore `refinements` and rely on `vc.smt2`.
- Coverage status for language features/intrinsics is tracked separately in `docs/proofs/proof-coverage-matrix.md` and `docs/proofs/proof-coverage-matrix.json`.
- `canonical_function` appears only when `CLG_MANGLE_MAX_LEN` shortening changed the emitted function symbol.

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
- `docs/proofs/fixtures/linear-collections-branch.vc.json` (prototype linear branch VC for ownership-sensitive collection flow).
- `docs/proofs/fixtures/linear-collections-branch-inline.vc.json` (prototype linear branch VC when owner flows through an inline call expression).
- `docs/proofs/fixtures/linear-collections-loop.vc.json` (prototype linear loop VC plus loop invariant/variant obligations).

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
