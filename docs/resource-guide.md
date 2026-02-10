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

- `List<Resource>` and `Map<K, Resource>` are allowed, but ownership-extracting operations must use `*_take` APIs.
- `Set<Resource>` and other unsupported resource-container forms still raise `T806`.
- For resource collections, use:
  - `std::list::remove_take(l, i) -> (List<R>, Option<R>)`
  - `std::map::insert_take(m, k, v) -> (Map<K,R>, Option<R>)`
  - `std::map::remove_take(m, k) -> (Map<K,R>, Option<R>)`
- Borrow-read calls like `get` on resource collections are rejected with guidance to the corresponding take API.

## Diagnostics (typer codes)

- `T801` use-after-consume.
- `T802` double-consume.
- `T803` consume while borrowed.
- `T804` branch ownership mismatch.
- `T806` unsupported resource-collection form or operation.

## Quick patterns

- Transfer: `function take(consume f: File) -> File { f }`
- Borrow-only helper: `function size(file: File) -> Int { 0 }`
- Single-use pipeline: `let out = take(f); out` (passes ownership through)
