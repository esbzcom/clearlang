# ClearLang Resource Guide

This guide summarizes how resources work in Phase 8: declaration, ownership/borrows, drops, and diagnostics.

## Declaring resources

- Syntax: `resource Name { fields... drop { ... } }`
- Fields are plain types; the `drop { ... }` block is required (empty block is allowed).
- Each declaration creates a nominal `Type::Named { name: "Name", args: [] }`.

## Function parameters and ownership

- Parameters are `borrow` by default; use `consume` to transfer ownership into the callee.
- A consumed parameter must be consumed exactly once (returned, passed to another `consume`, or explicitly dropped).
- Borrowed parameters cannot be consumed; they stay live for the whole function body.

## Blocks, returns, and locals

- Locals of resource type must also be consumed exactly once before block end.
- Returning a resource counts as consuming that binding.
- Borrow tracking is block-scoped and merges across `if`/`match`; mismatched ownership states cause `T804`.

## Collections

- Storing resources in standard `List`/`Set`/`Map` (including nested) is rejected with `T806`.
- A future phase will cover linear-aware collections; current containers are value-only.

## Diagnostics (typer codes)

- `T801` use-after-consume.
- `T802` double-consume.
- `T803` consume while borrowed.
- `T804` branch ownership mismatch.
- `T806` resource inside collections.

## Quick patterns

- Transfer: `function take(consume f: File) -> File { f }`
- Borrow-only helper: `function size(file: File) -> Int { 0 }`
- Single-use pipeline: `let out = take(f); out` (passes ownership through)
