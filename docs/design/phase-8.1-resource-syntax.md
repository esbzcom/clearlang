# Phase 8.1 – Resource Syntax & Block Expressions

## Status
- Parser/AST support for `resource` declarations and consume parameters implemented (2025-10-04).

## Goals
- Introduce `resource` definitions with explicit `drop { ... }` blocks.
- Allow block expressions (multi-statement bodies with optional early `return`).
- Extend function signatures with the `consume` modifier and plumb ownership metadata to the AST.

## Grammar Sketch
```
ResourceDecl ::= "resource" Ident GenericParams? "{" ResourceBody "}"
ResourceBody ::= ResourceField* DropBlock
ResourceField ::= Ident ":" Type ";"
DropBlock    ::= "drop" BlockExpr

BlockExpr    ::= "{" Stmt* Expr? "}"
Stmt         ::= LetStmt | ExprStmt
LetStmt      ::= "let" Pattern "=" Expr ";"
ExprStmt     ::= Expr ";"
```

### Notes
- `DropBlock` reuses `BlockExpr` so destructors share the same semantics as general expression blocks.
- Block expressions can end with an optional tail expression; missing tail implies `()`.
- `return` is permitted in any statement position inside a block expression; type checking will ensure returned value matches the enclosing function.

## AST Changes
- Add `Ast::Block { statements: Vec<Stmt>, tail: Option<Expr> }` reusable for function bodies, drop blocks, and general expressions.
- Extend resource declarations with `drop: BlockExpr` and store field list with spans for diagnostics.
- Introduce `SignatureParamKind` enum with variants `Borrow` (default) and `Consume` to tag function parameters.

## Parser Work
1. Teach the parser to treat `{ ... }` as an expression in contexts where block expressions are allowed.
2. Add a `resource` item parser that builds the new AST nodes.
3. Permit `consume` before parameter names (`fn take(consume handle: File) -> Unit`).

## Diagnostics
- Error when `drop {}` block is missing.
- Warn on unused block tail expression if the surrounding context expects `Unit` (mirrors existing expression handling).
- Provide span for each resource field and drop block for future typing errors.

## Follow-Up Hooks
- Typer will enforce linear usage in Phase 8.2 using the new AST metadata.
- Lowering should preserve `consume` flags even if runtime stubs are temporary.
- Docs to update: `docs/typing.md`, new section in `docs/ir.md` once lowering lands, CLI help for `consume` and block expressions.