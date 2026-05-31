# Std Coverage Matrix (Phase 26)

Status keys:
- `typed`: typer + lowering coverage
- `runtime`: codegen/runtime execution coverage
- `proved`: theorem-grade proof coverage in release policy

Allowed values:
- `yes`
- `no`
- `deferred`

## `std::str`

| Symbol | typed | runtime | proved | Notes |
|---|---|---|---|---|
| `std::str::len` | yes | yes | no | Release-enabled API; proof coverage pending Gate B/C integration. |
| `std::str::is_empty` | yes | yes | no | Lowered via `len == 0`; proof status follows `len`. |
| `std::str::equals` | yes | yes | no | Alias of `std::str::eq`; theorem-grade closure pending. |
| `std::str::concat` | yes | yes | no | Runtime string model coverage exists; proof closure pending. |
| `std::str::starts_with` | yes | yes | no | Implemented intrinsic; proof modeling pending. |
| `std::str::ends_with` | yes | yes | no | Implemented intrinsic; proof modeling pending. |
| `std::str::contains` | yes | yes | no | Implemented intrinsic; proof modeling pending. |
| `std::str::to_bytes` | yes | yes | no | Lowered to `std::bytes::from_string`; proof closure pending. |
| `std::str::slice` | no | no | deferred | Deferred from first-production release cut. |
| `std::str::trim` | no | no | deferred | Deferred from first-production release cut. |
| `std::str_pattern::matches` | yes | yes | no | Stable `matches(pattern, input)` contract lowered to deterministic `std::str::contains(input, pattern)` runtime behavior. |

## `std::bytes`

| Symbol | typed | runtime | proved | Notes |
|---|---|---|---|---|
| `std::bytes::len` | yes | yes | no | Release-enabled API; proof closure pending. |
| `std::bytes::is_empty` | yes | yes | no | Lowered via `len == 0`; proof status follows `len`. |
| `std::bytes::concat` | yes | yes | no | Release-enabled API; deterministic runtime behavior required. |
| `std::bytes::equals` | yes | yes | no | Alias of `std::bytes::eq`; release-enabled equality. |
| `std::bytes::equals_ct` | yes | yes | no | Alias of `std::bytes::eq_ct`; constant-time contract pending proof closure. |
| `std::bytes::from_string` | yes | yes | no | Release-enabled UTF-8 byte-identity conversion. |
| `std::bytes::slice` | no | no | deferred | Deferred from first-production release cut. |
| `std::bytes::to_string` | no | no | deferred | Deferred from first-production release cut. |
| `std::bytes::to_hex` | no | no | deferred | Deferred from first-production release cut. |
| `std::bytes::from_hex` | no | no | deferred | Deferred from first-production release cut. |
| `std::bytes_error::code` | no | no | deferred | Deferred with error-producing helpers. |
| `std::bytes_error::offset` | no | no | deferred | Deferred with error-producing helpers. |
| `std::bytes_error::equals` | no | no | deferred | Deferred with error-producing helpers. |

## `std::codec`

| Symbol | typed | runtime | proved | Notes |
|---|---|---|---|---|
| `std::encoder::new` | no | no | no | First-production target; implementation pending. |
| `std::encoder::write_u64` | no | no | no | First-production target; implementation pending. |
| `std::encoder::write_bool` | no | no | no | First-production target; implementation pending. |
| `std::encoder::write_bytes` | no | no | no | First-production target; implementation pending. |
| `std::encoder::finish` | no | no | no | First-production target; implementation pending. |
| `std::decoder::new` | no | no | no | First-production target; implementation pending. |
| `std::decoder::read_u64` | no | no | no | First-production target; implementation pending. |
| `std::decoder::read_bool` | no | no | no | First-production target; implementation pending. |
| `std::decoder::read_bytes` | no | no | no | First-production target; implementation pending. |
| `std::decoder::read_fixed` | no | no | no | First-production target; implementation pending. |
| `std::decoder::position` | no | no | no | First-production target; implementation pending. |
| `std::decoder::remaining` | no | no | no | First-production target; implementation pending. |
| `std::decoder::is_eof` | no | no | no | First-production target; implementation pending. |
| `std::decode_error::code` | no | no | no | First-production target; implementation pending. |
| `std::decode_error::offset` | no | no | no | First-production target; implementation pending. |
| `std::decode_error::equals` | no | no | no | First-production target; implementation pending. |
| `std::encode_error::code` | no | no | no | First-production target; implementation pending. |
| `std::encode_error::equals` | no | no | no | First-production target; implementation pending. |
| `std::encoder::write_u128` | no | no | deferred | Deferred from first-production cut. |
| `std::encoder::write_u256` | no | no | deferred | Deferred from first-production cut. |
| `std::encoder::write_string` | no | no | deferred | Deferred from first-production cut. |
| `std::decoder::read_u128` | no | no | deferred | Deferred from first-production cut. |
| `std::decoder::read_u256` | no | no | deferred | Deferred from first-production cut. |
| `std::decoder::read_string` | no | no | deferred | Deferred from first-production cut. |

## `std::int`

| Symbol | typed | runtime | proved | Notes |
|---|---|---|---|---|
| `std::u64::add_wrapping` | yes | yes | no | Release-enabled canonical API. |
| `std::u64::sub_wrapping` | yes | yes | no | Release-enabled canonical API. |
| `std::u64::mul_wrapping` | yes | yes | no | Release-enabled canonical API. |
| `std::u64::add_saturating` | yes | yes | no | Release-enabled canonical API. |
| `std::u64::sub_saturating` | yes | yes | no | Release-enabled canonical API. |
| `std::u64::mul_saturating` | yes | yes | no | Release-enabled canonical API. |
| `std::u64::rotl` | yes | yes | no | Release-enabled canonical API. |
| `std::u64::rotr` | yes | yes | no | Release-enabled canonical API. |
| `std::u64::to_bytes_le` | yes | yes | no | Release-enabled canonical API. |
| `std::u64::to_bytes_be` | yes | yes | no | Release-enabled canonical API. |
| `std::u64::from_bytes_le` | yes | yes | no | Release-enabled canonical API. |
| `std::u64::from_bytes_be` | yes | yes | no | Release-enabled canonical API. |
| `std::u64::add_wrap` | yes | yes | no | Compatibility alias. |
| `std::u64::sub_wrap` | yes | yes | no | Compatibility alias. |
| `std::u64::mul_wrap` | yes | yes | no | Compatibility alias. |
| `std::u64::add_sat` | yes | yes | no | Compatibility alias. |
| `std::u64::sub_sat` | yes | yes | no | Compatibility alias. |
| `std::u64::mul_sat` | yes | yes | no | Compatibility alias. |
| `std::u128::from_limbs` | yes | yes | no | Release-enabled canonical API. |
| `std::u128::lo` | yes | yes | no | Release-enabled canonical API. |
| `std::u128::hi` | yes | yes | no | Release-enabled canonical API. |
| `std::u256::from_limbs` | yes | yes | no | Release-enabled canonical API. |
| `std::u256::limb0` | yes | yes | no | Release-enabled canonical API. |
| `std::u256::limb1` | yes | yes | no | Release-enabled canonical API. |
| `std::u256::limb2` | yes | yes | no | Release-enabled canonical API. |
| `std::u256::limb3` | yes | yes | no | Release-enabled canonical API. |
| `std::u64::{add_checked,sub_checked,mul_checked,div_checked,mod_checked}` | no | no | deferred | Deferred from first-production cut. |
| `std::u64::{bit_and,bit_or,bit_xor,bit_not}` | no | no | deferred | Deferred from first-production cut. |
| `std::u64::{shl_checked,shr_checked}` | no | no | deferred | Deferred from first-production cut. |
| `std::u128::{add_checked,sub_checked,mul_checked,div_checked,mod_checked}` | no | no | deferred | Deferred from first-production cut. |
| `std::u128::{bit_and,bit_or,bit_xor,bit_not}` | no | no | deferred | Deferred from first-production cut. |
| `std::u128::{shl_checked,shr_checked,rotl,rotr}` | no | no | deferred | Deferred from first-production cut. |
| `std::u256::{add_checked,sub_checked,mul_checked,div_checked,mod_checked}` | no | no | deferred | Deferred from first-production cut. |
| `std::u256::{bit_and,bit_or,bit_xor,bit_not}` | no | no | deferred | Deferred from first-production cut. |
| `std::u256::{shl_checked,shr_checked,rotl,rotr}` | no | no | deferred | Deferred from first-production cut. |
| `std::checked::{ok,err,to_result}` | no | no | deferred | Deferred from first-production cut. |
| `std::int_error::{code,equals}` | no | no | deferred | Deferred from first-production cut. |

## `std::crypto`

| Symbol | typed | runtime | proved | Notes |
|---|---|---|---|---|
| `std::crypto::hash` | yes | yes | no | Current host-backed compatibility API. |
| `std::crypto::sha256` | yes | yes | no | Canonical typed first-production hash API. |
| `std::crypto::hmac` | yes | yes | no | Current host-backed compatibility API. |
| `std::crypto::hmac_sha256` | yes | yes | no | Canonical typed first-production HMAC API. |
| `std::crypto::verify` | yes | yes | no | Current host-backed verify API; proof boundary still open. |
| `std::crypto::verify_result::{is_valid,error_or_none,valid,invalid}` | no | no | no | Locked in 26.1.4.5; implementation pending. |
| `std::crypto::crypto_error::{code,equals}` | no | no | no | Locked in 26.1.4.5; implementation pending. |
| `std::crypto::hash256::blake2b_256` | no | no | deferred | Deferred algorithm expansion. |
| `std::crypto::algorithm::secp256k1` | no | no | deferred | Deferred algorithm expansion. |

## `std::host`

| Symbol | typed | runtime | proved | Notes |
|---|---|---|---|---|
| `std::host::storage::{contains,get,set,delete}` | no | no | no | Locked in 26.1.4.6; implementation pending. |
| `std::host::log::{info,warn,error}` | no | no | no | Locked in 26.1.4.6; implementation pending. |
| `std::host::env::{chain_id,caller,block_height,timestamp}` | no | no | no | Locked in 26.1.4.6; implementation pending. |
| `std::host::env::gas_left` | no | no | deferred | Deferred from first-production cut. |
| `std::host::host_error::{code,equals}` | no | no | no | Locked in 26.1.4.6; implementation pending. |

## `std::contract`

| Symbol | typed | runtime | proved | Notes |
|---|---|---|---|---|
| `std::contract::address::{from_bytes,to_bytes,equals}` | no | no | no | Locked in 26.1.4.7; implementation pending. |
| `std::contract::amount::{from_u64,value,add_checked,sub_checked,is_zero}` | no | no | no | Locked in 26.1.4.7; implementation pending. |
| `std::contract::event::{new,topic,payload}` | no | no | no | Locked in 26.1.4.7; implementation pending. |
| `std::contract::contract_error::{code,equals}` | no | no | no | Locked in 26.1.4.7; implementation pending. |

## `std::unit`

| Symbol | typed | runtime | proved | Notes |
|---|---|---|---|---|
| `std::unit::assert_true` | yes | yes | no | Gate D baseline assertion API. |
| `std::unit::assert_false` | yes | yes | no | Gate F deterministic assertion extension. |
| `std::unit::assert_eq_int` | yes | yes | no | Gate D baseline assertion API. |
| `std::unit::assert_eq_bool` | yes | yes | no | Gate D baseline assertion API. |
| `std::unit::assert_eq_u64` | yes | yes | no | Gate F deterministic assertion extension. |
| `std::unit::fail` | yes | yes | no | Gate D baseline assertion API. |
| `std::unit::{assert_eq_bytes,assert_eq_bytes_ct,assert_not_zero_u64}` | no | no | deferred | Deferred assertion expansion. |
| `std::unit::assert_error::{code,equals}` | no | no | deferred | Deferred typed assertion-error surface. |

## `std::chain::<target>`

| Symbol | typed | runtime | proved | Notes |
|---|---|---|---|---|
| `std::chain::<target>::*` | no | no | deferred | Deferred-by-default namespace family. |
| `std::eth::{from_bytes,from_array}` | yes | yes | no | Current compatibility target namespace. |
| `std::solana::{from_bytes,from_array}` | yes | yes | no | Current compatibility target namespace. |
| `std::cosmos::{from_bytes,from_array}` | yes | yes | no | Current compatibility target namespace. |

## `std::dynamic`

| Symbol | typed | runtime | proved | Notes |
|---|---|---|---|---|
| `std::dynamic::package_ref::{new,name,version,digest}` | no | no | deferred | Deferred post-first-production. |
| `std::dynamic::resolver_policy::{strict,allow_cached_only,require_signature,require_trust_anchor}` | no | no | deferred | Deferred post-first-production. |
| `std::dynamic::{resolve,verify}` | no | no | deferred | Deferred post-first-production. |
| `std::dynamic::link_error::{code,equals}` | no | no | deferred | Deferred post-first-production. |

## `std::list`

| Symbol | typed | runtime | proved | Notes |
|---|---|---|---|---|
| `std::list::{new,len,is_empty,get}` | yes | yes | yes | Read-only list proof rows closed in `26.3.1` and `26.3.2` with zero-assumption VC evidence plus index-safety/get-preservation SMT axioms. |
| `std::list::{push,pop}` | yes | yes | yes | Append/pop proof rows closed in `26.3.3` with zero-assumption VC evidence (`push => len + 1`, `pop Some <=> len > 0`, `pop None <=> len == 0`). |
| `std::list::{insert,remove,remove_take}` | yes | yes | yes | Indexed mutation proof rows closed in `26.3.4` with zero-assumption VC evidence plus SMT shape/order invariants and deterministic out-of-range runtime guard semantics. |
| `std::list::{insert_checked,remove_checked}` | yes | yes | yes | Compatibility rows closed in `26.3.6` with zero-assumption VC evidence (`Result` tag/payload SMT compatibility axioms) plus runtime conformance tests for in-range `Ok` and out-of-range `Err` non-trapping behavior. |
| `std::list::{can_mut,push_mut,insert_mut,remove_mut,pop_mut}` | yes | yes | no | Current guarded mutable compatibility surface. |
| `std::list` future additions | no | no | deferred | Additive-only after conformance lock. |

## `std::set`

| Symbol | typed | runtime | proved | Notes |
|---|---|---|---|---|
| `std::set::{new,is_empty,insert,remove}` | yes | yes | no | Release-enabled set baseline (non-cardinality). |
| `std::set::len` | yes | yes | yes | Cardinality VC proof row closed in `26.2.5` with assumption-free evidence. |
| `std::set::contains` | yes | yes | yes | Finite-set membership proof row closed in `26.2.2` with zero-assumption VC evidence. |
| `std::set::subset` | yes | yes | yes | Finite-set subset proof row closed in `26.2.2` with zero-assumption VC evidence. |
| `std::set::{can_mut,insert_mut,remove_mut}` | yes | yes | no | Current guarded mutable compatibility surface. |
| `std::set::{union,intersect,diff}` | yes | yes | yes | Implemented in Gate C step `26.2.4`; assumption-free VC evidence added for set algebra closure. |

## `std::map`

| Symbol | typed | runtime | proved | Notes |
|---|---|---|---|---|
| `std::map::{new,len,is_empty,contains,get}` | yes | yes | yes | Read-only map proof rows closed in `26.4.1` with zero-assumption VC evidence for key-presence and `get` tag consistency plus deterministic `is_empty` runtime conformance. |
| `std::map::{insert,insert_take}` | yes | yes | yes | Overwrite/membership map proof rows closed in `26.4.2` with zero-assumption VC evidence for unchanged-key membership consistency, inserted-key value consistency, and deterministic overwrite length behavior. |
| `std::map::{remove,remove_take}` | yes | yes | yes | Removal/take map proof rows closed in `26.4.3` with zero-assumption VC evidence for present/absent-key membership behavior, remove length delta, and `remove_take` map/value projection consistency. |
| `std::map::{can_mut,insert_mut,remove_mut}` | yes | yes | no | Current guarded mutable compatibility surface; `insert_mut/remove_mut` alias base operations, and `can_mut` remains compatibility-oriented until ownership/uniqueness model closure. |
| `std::map::{keys,values,entries,iter,iter_keys,iter_values}` | no | no | deferred | Release-disabled until canonical ordering/conformance lock in `docs/design/phase-26.4.7-map-iteration-ordering-lock.md` plus runtime/coverage evidence lands. |
