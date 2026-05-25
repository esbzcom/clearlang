# Phase 26.1.4.4 - `std::codec` First-Production API Lock

## Scope
This lock defines the first-production `std::codec` surface for Gate B execution.

## First-Production API (Release-Enabled)

Encoder:
- `std::encoder::new() -> Encoder`
- `std::encoder::write_u64(enc: Encoder, value: U64) -> Result<Encoder, EncodeError>`
- `std::encoder::write_bool(enc: Encoder, value: Bool) -> Result<Encoder, EncodeError>`
- `std::encoder::write_bytes(enc: Encoder, value: Bytes) -> Result<Encoder, EncodeError>`
- `std::encoder::finish(enc: Encoder) -> Result<Bytes, EncodeError>`

Decoder:
- `std::decoder::new(input: Bytes) -> Decoder`
- `std::decoder::read_u64(dec: Decoder) -> Result<(Decoder, U64), DecodeError>`
- `std::decoder::read_bool(dec: Decoder) -> Result<(Decoder, Bool), DecodeError>`
- `std::decoder::read_bytes(dec: Decoder) -> Result<(Decoder, Bytes), DecodeError>`
- `std::decoder::read_fixed(dec: Decoder, len: Int) -> Result<(Decoder, Bytes), DecodeError>`
- `std::decoder::position(dec: Decoder) -> Int`
- `std::decoder::remaining(dec: Decoder) -> Int`
- `std::decoder::is_eof(dec: Decoder) -> Bool`

Error surfaces:
- `std::decode_error::{code,offset,equals}`
- `std::encode_error::{code,equals}`

## Determinism and Safety Contracts
- Encoding/decoding MUST be canonical and deterministic across supported targets.
- `read_fixed` MUST reject negative lengths and `len > remaining(dec)` with stable deterministic `DecodeError` mapping.
- `position`/`remaining` MUST be monotonic with successful reads and MUST NOT be negative.
- Decode paths MUST fail closed for truncated, oversized, and malformed payloads.

## Deferred (Non-Release) Symbols
- `std::encoder::write_u128`
- `std::encoder::write_u256`
- `std::encoder::write_string`
- `std::decoder::read_u128`
- `std::decoder::read_u256`
- `std::decoder::read_string`

## Gate B Exit for `std::codec`
`std::codec` slice is complete when:
1. API/doc/metadata surface is consistent with this lock.
2. `typed|runtime|proved` statuses are recorded in `docs/std/coverage-matrix.md`.
3. Release profile fails closed for deferred codec symbols.
4. CI has deterministic contract tests for all release-enabled codec symbols.
