# Phase 17.5 - Modules and Imports

## Status
Design note only. Implementation tracked in `docs/TODO.md` under Phase 17.5.

## Goals
- Provide a minimal module system with explicit visibility for library code.
- Keep syntax simple and non-magical: no `mod` boilerplate, no wildcards.
- Make resolution deterministic and AI-friendly (stable rules, no hidden preludes).
- Preserve chain-agnostic core and keep `std`/chain packages separate.

## Design Principles Check
- Simple for users: file path == module path, `export` and `import` only.
- AI-friendly: explicit imports, no glob imports or implicit name injection.
- Provably correct: deterministic resolution avoids ambiguous bindings.
- Crypto-focused: supports clean library boundaries without complex tooling.

## Non-Goals
- Package manager or dependency registry.
- Importing from compiled artifacts (tracked in Phase 18.4).
- Re-exports in v1 (defer until a clear use case emerges).
- Field-level visibility or access control beyond top-level items.

## Module Identity and File Layout

Decision: module paths are derived from the filesystem.

Rules:
- The module root is the directory containing the entry file passed to `clg build`.
- A file `root/foo/bar.clear` defines module `foo::bar`.
- Modules can be nested arbitrarily deep; there is no hard depth limit.
- File extension is `.clear`.

Optional header:
- A `module` header is allowed only for overrides; omitted by default.
- If present, the header must match the file path (unless explicitly opting out
  for generated code). Mismatches are a build error.

## Visibility

Decision: `export` is the only visibility modifier.

Rules:
- All top-level items are private to their module unless marked `export`.
- `export` can be applied to: `function`, `struct`, `enum`, `trait`, `type`, `resource`.
- Exported items are visible to any other module.
- There is no per-field visibility in Phase 17.5.

Example:
```
export function add(a: Int, b: Int) -> Int { a + b }
function helper(x: Int) -> Int { x * 2 } // private
```

## Imports

Decision: use an explicit `import` keyword (not Rust-like `use`).

Forms:
```
import foo::bar            // bind module as `bar`
import foo::bar as baz     // bind module as `baz`
import foo::bar::{A, B}    // import explicit items into scope
```

Notes:
- No glob imports (`*`) in v1.
- No re-exports in v1 (i.e., `export import` is not allowed).
- Item imports (`{A, B}`) must be explicit and exact; missing names are errors.

Name use:
- `import foo::bar` allows `bar::Name` usage.
- `import foo::bar::{A, B}` allows direct use of `A` and `B`.

## Fully Qualified Paths

- Fully qualified paths (e.g., `foo::bar::Baz`) are always legal without an import.
- Imports are only for convenience and clarity.

## Standard Library and Chain Packages

Decision: `std` is a reserved root.

Rules:
- `std::...` is resolved by the compiler to builtin/stdlib definitions, not files.
- User code cannot define a top-level `std` module.
- Chain packages are modeled under `std::<chain>::...` and resolved by the host/
  toolchain (same rule as `std`).

## Resolution and Errors

- Resolution is deterministic and file-based.
- Cyclic imports are rejected with a clear error (phase 17.5.3).
- Ambiguous item imports (name conflicts) are rejected.

## Examples

File layout:
```
contracts/
  main.clear
  math/
    arith.clear
  util/
    bytes.clear
```

`contracts/math/arith.clear`
```
export function add(a: Int, b: Int) -> Int { a + b }
export function sub(a: Int, b: Int) -> Int { a - b }
```

`contracts/util/bytes.clear`
```
export function is_empty(b: Bytes) -> Bool {
    std::bytes::len(b) == 0
}
```

`contracts/main.clear`
```
import math::arith
import util::bytes as b

function main() -> Int {
    let x: Int = arith::add(2, 3);
    let empty: Bool = b::is_empty(std::bytes::from_string(""));
    if empty { x } else { arith::sub(x, 1) }
}
```

## Implementation Notes (for TODO breakdown)
- Parser/AST: add `import` statements and optional `module` header.
- Resolver: map module paths to file paths; enforce reserved `std` root.
- Typer: enforce visibility rules for imported names.
- Build system: expand entry file to a module graph, detect cycles, and order
  modules for type checking/lowering.
- Docs: update `docs/typing.md` and `docs/collections.md` with the new syntax.

## Open Questions
- Should `module` headers be allowed to override path-based names at all?
- Should item imports allow aliasing (e.g., `import foo::bar::{A as X}`)?
