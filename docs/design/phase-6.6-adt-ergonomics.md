# Phase 6.6 - ADT Ergonomics Design Note

## Overview
- **Goal**: provide lightweight surface sugar for Option/Result ergonomics (`if let`, `??`, postfix `?`) while keeping compilation proofs and diagnostics predictable.
- **Scope**: parser, typer, VC generation, CLI tooling, and IR/Wasm lowering now ship together.
- **Availability**: the sugar is enabled by default as of Phase 7.3 (no experimental switch required).

## Surface Syntax & Desugars
| Sugar | Desugar | Notes |
| --- | --- | --- |
| `if let Some(x) = opt { a } else { b }` | `match opt { Some(x) => a, None => b }` | Supports `Some`, `Ok`, `Err`. Parser enforces an explicit `else`. |
| `lhs ?? rhs` | `match lhs { Some(v) => v, None => rhs }` | Chains left-associatively; uses temporary binders that avoid name capture. |
| `expr?` | Option: unwrap or early-return `None`. Result: unwrap `Ok` or early-return `Err`. | Typer ensures operand/result alignment (`T601`-`T606`). |

## Typing Rules
- `if let` reuses match typing (T201?T205) and prevents binder shadowing.
- `??` requires an `Option<T>` left operand and a right operand of type `T`; mismatches raise `T603`.
- `expr?` validates both the operand type (`Option<T>`/`Result<T,E>`) and the surrounding return type (`T601`/`T604`).
- Constructors (`Some`, `None`, `Ok`, `Err`) enforce return-context correctness via `T607`?`T613`.
- The implicit `$return` binding lets the typer validate propagation sites.

## Effect & Purity
- All sugars are pure; their effect is the effect of the operand expression.
- Existing guard logic covers mutable intrinsics; no additional runtime guards are required.

## VC Generation
- `generate_vcs` rewrites contracts and bodies using the canonical variant layout.
- `SmtEncoder` emits helper declarations (`cl.variant.tag`, `.payload_lo`, `.payload_hi`, `cl.option.mk`, `cl.result.mk`) exactly when needed.
- `Expr::Match`/`Expr::Try` appear as `let`-bound terms that check the tag (`= 0`/`= 1`) before extracting payloads.
- Output is stable under `--emit-vcs` and validated by `crates/typer/tests/vc.rs` and `crates/cli/tests/cli_it.rs`.

## Testing Strategy
- Parser/typer suites hit positive and negative cases for each sugar.
- VC tests assert the presence of variant helpers instead of placeholder comments.
- CLI integration (`build_emits_variant_vcs_json`) covers end-to-end builds with `--emit-vcs` and inspects the JSON structure.
- Lowering tests (`crates/typer/tests/lowering_variants.rs`) confirm SSA stability for both Option and Result flows.

## Worked Examples

### `if let` / `??`
```cl
pure function default_or_zero(opt: Option<Int>) -> Int
    ensure { result >= 0 }
{ opt ?? 0 }
```
`generate_vcs` outputs:
```
; Option/Result variants use (tag, payload_lo, payload_hi)
(declare-fun cl.variant.tag (Int) Int)
(declare-fun cl.variant.payload_lo (Int) Int)
(=> true (let ((cl_match$0 opt))
             (ite (= (cl.variant.tag cl_match$0) 1)
                  (let ((__coalesce_tmp0 (cl.variant.payload_lo cl_match$0)))
                       (>= __coalesce_tmp0 0))
                  (>= 0 0))))
```

### `?` propagation
```cl
pure function pick(opt: Option<Int>) -> Option<Int>
    ensure { result == result }
{ Some((opt ?? 7) + 1) }
```
SMT output starts with the same helper declarations and then encodes the propagation:
```
(=> true (= (cl.option.mk 1 (+ (let ((cl_match$0 opt))
                                   (ite (= (cl.variant.tag cl_match$0) 1)
                                        (let ((__coalesce_tmp0 (cl.variant.payload_lo cl_match$0)))
                                             __coalesce_tmp0)
                                        7))
                             1)
                        0)
                  (cl.option.mk 1 (+ (let ((cl_match$1 opt))
                                        (ite (= (cl.variant.tag cl_match$1) 1)
                                             (let ((__coalesce_tmp0 (cl.variant.payload_lo cl_match$1)))
                                                  __coalesce_tmp0)
                                             7))
                                1)
                        0)))
```

## Status & Follow-Ups
- Lowering, runtime encoding, and SMT integration are complete.
- `--emit-vcs` now produces solver-friendly helpers for all Option/Result sugar.
- Future work focuses on pointer-backed payload metadata (string `payload_hi`) and proof packaging (Phase 7.5+).
