# Value Preservation (Subset) — Proof Sketch

Scope
- Expressions over `Int` and `Bool` using `+ - * /` and comparisons; function calls without recursion; expression-bodied functions only.
- IR instructions: `IConst`, `IBin(Add|Sub|Mul|Div)`, `Call`, `Ret`.
- Codegen target: Wasm `i32` stack machine; `Bool` encoded as `0|1`.

Judgments
- Typing: `Γ ⊢ e : τ` where `τ ∈ {Int, Bool}`.
- Evaluation (source): `⟨e, σ⟩ ⇓ v` with standard small-step semantics (deterministic); `v ∈ Z ∪ {true,false}`.
- Interpretation: `⟦true⟧=1`, `⟦false⟧=0`, `⟦n:Int⟧=n mod 2^32`.
- Codegen: `codegen(e)` yields Wasm bytes; execution `eval_wasm(codegen(e)) ⇓ n32` returns `i32`.

Theorem (Value Preservation)
If `Γ ⊢ e : τ` and `⟨e, σ⟩ ⇓ v` then executing the generated Wasm yields the integer encoding of `v`:
`eval_wasm(codegen(e)) ⇓ ⟦v⟧`.

Proof Sketch
1) By structural induction on the derivation of `⟨e, σ⟩ ⇓ v`.
   - Literals: `IConst` encodes directly; Wasm `i32.const` yields `⟦v⟧`.
   - Binops: Inductive hypotheses for operands; Wasm applies the corresponding `i32` op. For Bool comparisons, encode operands, compute relation, and normalize to `0|1`.
   - Calls: By IH on arguments and by function-level lemma below.
2) Function Lemma: For expression-bodied `function f(p:τ) -> τ' { e }` with `Γ_f ⊢ e : τ'`, compilation introduces a Wasm func whose body is `codegen(e) ; return`. By the main IH, evaluating `e` and running the Wasm body agree on `⟦v⟧`.
3) Composition: The IR lowering preserves expression structure; codegen is a homomorphism for the covered instructions.

Side Conditions
- Division uses Wasm `i32.div_s`; source semantics match two’s-complement truncation (documented). Division by zero is unspecified/traps on both sides.
- No overflow signaling: source Ints are modulo 2^32 for this subset.

Traceability
- Tests referenced: `clearlang-tests/02_arith`, `03_nested_calls`, `04_multiline_call`.
- E2E assertions compare Wasm result to interpreter result for selected inputs.

Limitations
- Control flow beyond expression form (if/else as `ISelect`) is omitted here; handled by an extension lemma.
- Strings and memory effects are outside this theorem; covered separately with runtime invariants.

