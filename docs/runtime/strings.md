# Strings Runtime (Phase 5.6)

Goals
- Provide a minimal, deterministic runtime for `String` to support `std::str::{len, concat, eq}`.
- Keep the model simple and verifiable; make the memory layout explicit and stable.

Representation
- Value: a pointer `ptr: i32` to UTF‑8 bytes in linear memory. Length is stored inline in a fixed header.
- Layout at address `ptr`:
  - `[0..4)` u32: byte length `len`
  - `[4..4+len)` u8[len]: UTF‑8 bytes (no trailing NUL)
- Invariants:
  - `0 ≤ len ≤ memory.size*65536 - 4`
  - The byte slice `[ptr+4, ptr+4+len)` is valid UTF‑8.
  - `ptr` is 4‑byte aligned.

Operations
- `std::str::len(s: String) -> Int`: returns `load_u32(ptr)`.
- `std::str::eq(a: String, b: String) -> Bool`:
  - Compare `len_a` and `len_b`; if different, return 0.
  - Compare bytes; return 1 if equal, 0 otherwise.
- `std::str::concat(a: String, b: String) -> String`:
  - Allocate `4 + len_a + len_b` bytes using a monotonic bump pointer.
  - Store `(len_a + len_b)` at the head; copy `a` then `b` bytes after the header.

Allocation
- Use a linear bump allocator stored in a mutable global `heap_ptr: i32` initialized after static data.
- Invariants:
  - Always moves forward; never overflows memory (trap on OOM for now).
  - 4‑byte alignment preserved.

Compiler Integration
- String literals are emitted as static data segments conforming to the layout above.
- IR / Lowering:
  - String literal → i32 pointer to its data segment.
  - Calls to `std::str::{len,eq,concat}` are lowered to intrinsics resolved during codegen.
- Codegen:
  - Adds a Memory section with minimum 1 page.
  - Emits static data for literals; sets `heap_ptr` to first free address after data.
  - Defines intrinsic functions for `len`, `eq`, `concat` using Wasm `load`, `store`, and `memory.copy`.

Testing
- Unit: intrinsic functions on small samples; equality and length edge cases.
- E2E: samples invoking `std::str::{len,eq,concat}` via `clg build` + Wasmtime.

Proof Sketch
- Safety: All memory accesses use bounds derived from stored `len` and the allocation invariant; `eq` only reads in‑bounds; `concat` writes to a fresh, in‑bounds allocation.
- Determinism: No host calls; pure arithmetic and memory ops; identical inputs yield identical outputs.

## Runtime Diagnostics

- Every intrinsic writes runtime metadata before trapping so tooling can read spans:
  - `__clg_runtime_error_code`: 0 (no error), 1 (`R000` contract guard), 2 (`R001` allocator OOM), 3 (`R002` invalid UTF-8).
  - `__clg_runtime_error_start` / `__clg_runtime_error_end`: byte offsets for the originating span when available.
  - `__clg_runtime_error_detail`: extra context (0 for `require`, 1 for `ensure`; string intrinsics currently leave this as 0).
- `std::str::concat` raises `R001` when the bump allocation would exceed linear memory. Start/end capture the attempted allocation bounds.
- `std::str::{len, eq, concat}` raise `R002` when inputs fail alignment/bounds/UTF-8 checks, preventing undefined reads.
- `clg run --json-errors` surfaces these traps as `stage: "runtime"` diagnostics with stable codes to stay AI-friendly.
