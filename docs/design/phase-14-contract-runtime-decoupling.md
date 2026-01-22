# Phase 14 Contract/Runtime Decoupling

This note defines the pure contract core and the ABI envelope used by runtime
entrypoints. It is the reference for Phase 14 implementation work.

## Goals
- Keep contract logic as a pure state transition.
- Make the runtime boundary explicit and deterministic.
- Use a stable, canonical serialization format at the ABI edge.
- Keep chain-specific types and rules out of the core.

## Non-goals
- Define chain-specific message schemas or storage formats.
- Specify the full host import list (that is handled separately).
- Replace chain packages or rework the effect system in this phase.

## Pure Core Model

The contract core is a pure state transition. It consumes bytes and returns bytes
so the ABI can stay stable while higher-level types evolve.

```text
// ABI-level type (len-prefixed byte buffer in linear memory).
type Bytes = opaque byte array

// Pure state transition used by init/handle.
// Returns a response envelope encoded as Bytes.
pure function apply(state: Bytes, msg: Bytes) -> Bytes

// Pure read-only path for queries.
// Returns a response envelope encoded as Bytes.
pure function query(state: Bytes, msg: Bytes) -> Bytes
```

Notes:
- `state` is opaque to the runtime. The contract defines its own encoding.
- The response is a serialized envelope; the runtime may decode it to update state
  and emit events.
- `query` must not change state.

## ABI Entry Points

Runtime entry points are exported as `init`, `handle`, and `query`:

```text
// All exports share the same ABI.
// Inputs are length-prefixed buffers at (state_ptr, msg_ptr).
// The return value is a pointer to a length-prefixed response.
export init(state_ptr: i32, msg_ptr: i32) -> i32
export handle(state_ptr: i32, msg_ptr: i32) -> i32
export query(state_ptr: i32, msg_ptr: i32) -> i32
```

Return layout:
- `state_ptr` and `msg_ptr` point to `[u32 len][u8 len]` in linear memory.
- `ret_ptr` points to `[u32 len][u8 len]` in linear memory.
- This mirrors the `String` layout but does not require UTF-8.

Implementation note:
- `clg build --contract` exports `apply` as `init`/`handle` and exports `query` as `query`.

## Canonical Serialization (v1)

Requests and responses are canonical CBOR (RFC 8949) maps. All map keys are
text, and canonical ordering is required. The envelope version is explicit.

Request envelope:
```text
{
  "abi_version": 1,
  "kind": "init" | "handle" | "query",
  "state": <bstr>,   // empty for init
  "msg": <bstr>
}
```

Response envelope:
```text
{
  "abi_version": 1,
  "status": "ok" | "err",
  "state": <bstr>,     // init/handle only, empty for query
  "events": [ { "kind": <tstr>, "data": <bstr> } ],
  "data": <bstr>,      // response payload on success
  "error": { "code": <tstr>, "message": <tstr>, "data": <bstr>? }
}
```

Rules:
- `query` responses must set `state` to empty and `events` to empty.
- `init` ignores the incoming `state` and treats it as empty.
- Errors must not mutate state; runtime treats `status: "err"` as no-op.

## Determinism and Extensibility

- All nondeterminism must be explicit in inputs (`state` + `msg`) or the
  serialized request envelope. The core stays pure.
- New fields can be added to the envelope in a future `abi_version`, keeping
  older fields stable and required for v1.
