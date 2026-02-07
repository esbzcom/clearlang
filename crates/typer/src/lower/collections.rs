use crate::check::{base_type, infer_expr_type, AliasMap};
use anyhow::Result;
use clg_ast::{Expr, Span, Type};
use clg_ir::{BinOpIR, GuardKind, Instr, IrType, TrapCode, Value};

use super::layout::{collection_layout, map_entry_layout};
use super::eq::emit_eq_for_type;
use super::{
    emit_alloc, emit_alloc_dyn, emit_array_data_ptr, emit_array_len, emit_int_const, emit_load_i32,
    emit_memcpy_bytes, emit_ptr_add, emit_store_i32, expr_span_local, fresh, load_value_borrow,
    load_value_copy, lower_expr, store_value, zero_value_for_type, LowerCtx, ARRAY_HEADER_ALIGN,
    ARRAY_HEADER_DATA_OFFSET, ARRAY_HEADER_LEN_OFFSET, ARRAY_HEADER_SIZE, COLLECTION_CAP_OFFSET,
    COLLECTION_DATA_OFFSET, COLLECTION_FLAGS_OFFSET, COLLECTION_HEADER_ALIGN, COLLECTION_HEADER_SIZE,
    COLLECTION_LEN_OFFSET,
};

fn emit_collection_guard(ctx: &mut LowerCtx<'_>, cond: Value, span: Span) {
    ctx.body.push(Instr::Guard {
        cond,
        trap: TrapCode::CollectionBounds,
        span: Some((span.start as u32, span.end as u32)),
        detail: GuardKind::Require,
    });
}

fn emit_memory_bytes(ctx: &mut LowerCtx<'_>) -> Value {
    let pages = fresh(ctx);
    ctx.body.push(Instr::MemorySize { dst: pages });
    let shift = emit_int_const(ctx, 16);
    let bytes = fresh(ctx);
    ctx.body.push(Instr::IBin {
        dst: bytes,
        op: BinOpIR::Shl,
        lhs: pages,
        rhs: shift,
        ty: IrType::Int,
    });
    let max_pages = emit_int_const(ctx, 65536);
    let is_max = fresh(ctx);
    ctx.body.push(Instr::IBin {
        dst: is_max,
        op: BinOpIR::Eq,
        lhs: pages,
        rhs: max_pages,
        ty: IrType::Int,
    });
    let max_bytes = emit_int_const(ctx, -1);
    let capped = fresh(ctx);
    ctx.body.push(Instr::ISelect {
        dst: capped,
        cond: is_max,
        then_v: max_bytes,
        else_v: bytes,
    });
    capped
}

fn emit_collection_ptr_guard(ctx: &mut LowerCtx<'_>, ptr: Value) {
    let zero = emit_int_const(ctx, 0);
    let not_zero = fresh(ctx);
    ctx.body.push(Instr::IBin {
        dst: not_zero,
        op: BinOpIR::Neq,
        lhs: ptr,
        rhs: zero,
        ty: IrType::Int,
    });
    let align_mask = emit_int_const(ctx, (COLLECTION_HEADER_ALIGN - 1) as i64);
    let masked = fresh(ctx);
    ctx.body.push(Instr::IBin {
        dst: masked,
        op: BinOpIR::And,
        lhs: ptr,
        rhs: align_mask,
        ty: IrType::Int,
    });
    let aligned = fresh(ctx);
    ctx.body.push(Instr::IBin {
        dst: aligned,
        op: BinOpIR::Eq,
        lhs: masked,
        rhs: zero,
        ty: IrType::Int,
    });

    let header_size = emit_int_const(ctx, COLLECTION_HEADER_SIZE as i64);
    let end = fresh(ctx);
    ctx.body.push(Instr::IBin {
        dst: end,
        op: BinOpIR::Add,
        lhs: ptr,
        rhs: header_size,
        ty: IrType::Int,
    });
    let no_wrap = fresh(ctx);
    ctx.body.push(Instr::IBin {
        dst: no_wrap,
        op: BinOpIR::LeU,
        lhs: ptr,
        rhs: end,
        ty: IrType::Int,
    });
    let mem_bytes = emit_memory_bytes(ctx);
    let within = fresh(ctx);
    ctx.body.push(Instr::IBin {
        dst: within,
        op: BinOpIR::LeU,
        lhs: end,
        rhs: mem_bytes,
        ty: IrType::Int,
    });

    let tmp = fresh(ctx);
    ctx.body.push(Instr::IBin {
        dst: tmp,
        op: BinOpIR::And,
        lhs: not_zero,
        rhs: aligned,
        ty: IrType::Int,
    });
    let ok = fresh(ctx);
    ctx.body.push(Instr::IBin {
        dst: ok,
        op: BinOpIR::And,
        lhs: tmp,
        rhs: no_wrap,
        ty: IrType::Int,
    });
    let ok2 = fresh(ctx);
    ctx.body.push(Instr::IBin {
        dst: ok2,
        op: BinOpIR::And,
        lhs: ok,
        rhs: within,
        ty: IrType::Int,
    });

    ctx.body.push(Instr::Guard {
        cond: ok2,
        trap: TrapCode::InvalidBuffer,
        span: None,
        detail: GuardKind::Require,
    });
}

fn emit_collection_len(ctx: &mut LowerCtx<'_>, ptr: Value) -> Value {
    emit_collection_ptr_guard(ctx, ptr);
    emit_load_i32(ctx, ptr, COLLECTION_LEN_OFFSET)
}

fn emit_collection_data_ptr(ctx: &mut LowerCtx<'_>, ptr: Value) -> Value {
    emit_collection_ptr_guard(ctx, ptr);
    emit_load_i32(ctx, ptr, COLLECTION_DATA_OFFSET)
}

fn emit_collection_cap(ctx: &mut LowerCtx<'_>, ptr: Value) -> Value {
    emit_collection_ptr_guard(ctx, ptr);
    emit_load_i32(ctx, ptr, COLLECTION_CAP_OFFSET)
}

fn emit_collection_payload_guard(
    ctx: &mut LowerCtx<'_>,
    data_ptr: Value,
    len: Value,
    cap: Value,
    stride: u32,
    align: u32,
) {
    let zero = emit_int_const(ctx, 0);
    let not_zero = fresh(ctx);
    ctx.body.push(Instr::IBin {
        dst: not_zero,
        op: BinOpIR::Neq,
        lhs: data_ptr,
        rhs: zero,
        ty: IrType::Int,
    });

    let align = align.max(1);
    let align_mask = emit_int_const(ctx, (align - 1) as i64);
    let masked = fresh(ctx);
    ctx.body.push(Instr::IBin {
        dst: masked,
        op: BinOpIR::And,
        lhs: data_ptr,
        rhs: align_mask,
        ty: IrType::Int,
    });
    let aligned = fresh(ctx);
    ctx.body.push(Instr::IBin {
        dst: aligned,
        op: BinOpIR::Eq,
        lhs: masked,
        rhs: zero,
        ty: IrType::Int,
    });

    let len_nonneg = fresh(ctx);
    ctx.body.push(Instr::IBin {
        dst: len_nonneg,
        op: BinOpIR::Ge,
        lhs: len,
        rhs: zero,
        ty: IrType::Int,
    });
    let cap_gt_zero = fresh(ctx);
    ctx.body.push(Instr::IBin {
        dst: cap_gt_zero,
        op: BinOpIR::Gt,
        lhs: cap,
        rhs: zero,
        ty: IrType::Int,
    });
    let len_le_cap = fresh(ctx);
    ctx.body.push(Instr::IBin {
        dst: len_le_cap,
        op: BinOpIR::LeU,
        lhs: len,
        rhs: cap,
        ty: IrType::Int,
    });
    let max_len = emit_int_const(ctx, (i32::MAX as i64) / stride as i64);
    let len_fits = fresh(ctx);
    ctx.body.push(Instr::IBin {
        dst: len_fits,
        op: BinOpIR::Le,
        lhs: len,
        rhs: max_len,
        ty: IrType::Int,
    });

    let stride_val = emit_int_const(ctx, stride as i64);
    let bytes = fresh(ctx);
    ctx.body.push(Instr::IBin {
        dst: bytes,
        op: BinOpIR::Mul,
        lhs: len,
        rhs: stride_val,
        ty: IrType::Int,
    });
    let end = fresh(ctx);
    ctx.body.push(Instr::IBin {
        dst: end,
        op: BinOpIR::Add,
        lhs: data_ptr,
        rhs: bytes,
        ty: IrType::Int,
    });
    let no_wrap = fresh(ctx);
    ctx.body.push(Instr::IBin {
        dst: no_wrap,
        op: BinOpIR::LeU,
        lhs: data_ptr,
        rhs: end,
        ty: IrType::Int,
    });
    let mem_bytes = emit_memory_bytes(ctx);
    let within = fresh(ctx);
    ctx.body.push(Instr::IBin {
        dst: within,
        op: BinOpIR::LeU,
        lhs: end,
        rhs: mem_bytes,
        ty: IrType::Int,
    });

    let tmp = fresh(ctx);
    ctx.body.push(Instr::IBin {
        dst: tmp,
        op: BinOpIR::And,
        lhs: not_zero,
        rhs: aligned,
        ty: IrType::Int,
    });
    let tmp2 = fresh(ctx);
    ctx.body.push(Instr::IBin {
        dst: tmp2,
        op: BinOpIR::And,
        lhs: tmp,
        rhs: len_nonneg,
        ty: IrType::Int,
    });
    let tmp3 = fresh(ctx);
    ctx.body.push(Instr::IBin {
        dst: tmp3,
        op: BinOpIR::And,
        lhs: tmp2,
        rhs: cap_gt_zero,
        ty: IrType::Int,
    });
    let tmp4 = fresh(ctx);
    ctx.body.push(Instr::IBin {
        dst: tmp4,
        op: BinOpIR::And,
        lhs: tmp3,
        rhs: len_le_cap,
        ty: IrType::Int,
    });
    let tmp5 = fresh(ctx);
    ctx.body.push(Instr::IBin {
        dst: tmp5,
        op: BinOpIR::And,
        lhs: tmp4,
        rhs: len_fits,
        ty: IrType::Int,
    });
    let tmp6 = fresh(ctx);
    ctx.body.push(Instr::IBin {
        dst: tmp6,
        op: BinOpIR::And,
        lhs: tmp5,
        rhs: no_wrap,
        ty: IrType::Int,
    });
    let ok = fresh(ctx);
    ctx.body.push(Instr::IBin {
        dst: ok,
        op: BinOpIR::And,
        lhs: tmp6,
        rhs: within,
        ty: IrType::Int,
    });

    ctx.body.push(Instr::Guard {
        cond: ok,
        trap: TrapCode::InvalidBuffer,
        span: None,
        detail: GuardKind::Require,
    });
}

fn emit_collection_header(
    ctx: &mut LowerCtx<'_>,
    len: Value,
    cap: Value,
    data_ptr: Value,
) -> Value {
    let header = emit_alloc(ctx, COLLECTION_HEADER_SIZE, COLLECTION_HEADER_ALIGN);
    let zero = emit_int_const(ctx, 0);
    emit_store_i32(ctx, header, COLLECTION_LEN_OFFSET, len);
    emit_store_i32(ctx, header, COLLECTION_CAP_OFFSET, cap);
    emit_store_i32(ctx, header, COLLECTION_FLAGS_OFFSET, zero);
    emit_store_i32(ctx, header, COLLECTION_DATA_OFFSET, data_ptr);
    header
}

fn emit_cap_from_len(ctx: &mut LowerCtx<'_>, len: Value) -> Value {
    let zero = emit_int_const(ctx, 0);
    let one = emit_int_const(ctx, 1);
    let gt_zero = fresh(ctx);
    ctx.body.push(Instr::IBin {
        dst: gt_zero,
        op: BinOpIR::Gt,
        lhs: len,
        rhs: zero,
        ty: IrType::Int,
    });
    let cap = fresh(ctx);
    ctx.body.push(Instr::ISelect {
        dst: cap,
        cond: gt_zero,
        then_v: len,
        else_v: one,
    });
    cap
}

fn emit_find_index(
    ctx: &mut LowerCtx<'_>,
    data_ptr: Value,
    len: Value,
    stride: u32,
    key_val: Value,
    key_ty: &Type,
    key_offset: u32,
    aliases: &AliasMap,
) -> Result<(Value, Value)> {
    let found = fresh(ctx);
    ctx.body.push(Instr::IConst {
        dst: found,
        ty: IrType::Bool,
        n: 0,
    });
    let found_idx = fresh(ctx);
    ctx.body.push(Instr::IConst {
        dst: found_idx,
        ty: IrType::Int,
        n: 0,
    });
    let idx = fresh(ctx);
    ctx.body.push(Instr::IConst {
        dst: idx,
        ty: IrType::Int,
        n: 0,
    });
    let stride_val = emit_int_const(ctx, stride as i64);

    ctx.body.push(Instr::BlockBegin);
    ctx.body.push(Instr::LoopBegin);
    let cond = fresh(ctx);
    ctx.body.push(Instr::IBin {
        dst: cond,
        op: BinOpIR::Lt,
        lhs: idx,
        rhs: len,
        ty: IrType::Int,
    });
    ctx.body.push(Instr::BrIfEqz { cond, depth: 1 });

    let offset = fresh(ctx);
    ctx.body.push(Instr::IBin {
        dst: offset,
        op: BinOpIR::Mul,
        lhs: idx,
        rhs: stride_val,
        ty: IrType::Int,
    });
    let base_ptr = emit_ptr_add(ctx, data_ptr, offset);
    let key_ptr = if key_offset == 0 {
        base_ptr
    } else {
        let key_off_val = emit_int_const(ctx, key_offset as i64);
        emit_ptr_add(ctx, base_ptr, key_off_val)
    };

    let key_loaded = load_value_borrow(ctx, key_ty, key_ptr, 0)?;
    let eq = emit_eq_for_type(ctx, key_ty, key_loaded, key_val, aliases)?;

    ctx.body.push(Instr::IBin {
        dst: found,
        op: BinOpIR::Or,
        lhs: found,
        rhs: eq,
        ty: IrType::Bool,
    });
    ctx.body.push(Instr::ISelect {
        dst: found_idx,
        cond: eq,
        then_v: idx,
        else_v: found_idx,
    });

    let one = emit_int_const(ctx, 1);
    ctx.body.push(Instr::IBin {
        dst: idx,
        op: BinOpIR::Add,
        lhs: idx,
        rhs: one,
        ty: IrType::Int,
    });
    ctx.body.push(Instr::Br { depth: 0 });
    ctx.body.push(Instr::LoopEnd);
    ctx.body.push(Instr::BlockEnd);

    Ok((found, found_idx))
}

fn normalize_collection_callee(callee: &str) -> &str {
    match callee {
        "std::list::push_mut" => "std::list::push",
        "std::list::insert_mut" => "std::list::insert",
        "std::list::remove_mut" => "std::list::remove",
        "std::list::pop_mut" => "std::list::pop",
        "std::set::insert_mut" => "std::set::insert",
        "std::set::remove_mut" => "std::set::remove",
        "std::map::insert_mut" => "std::map::insert",
        "std::map::remove_mut" => "std::map::remove",
        _ => callee,
    }
}

fn list_elem_type(ctx: &LowerCtx<'_>, arg: &Expr) -> Result<Type> {
    let ty = infer_expr_type(
        arg,
        &ctx.type_env,
        &ctx.fns,
        ctx.trait_env,
        ctx.aliases,
        ctx.type_defs,
        &ctx.type_params,
        &ctx.bounds,
    )?;
    let base = base_type(&ty, ctx.aliases)?;
    match base {
        Type::List(inner) => Ok(*inner),
        other => anyhow::bail!("expected List argument, found {:?}", other),
    }
}

fn slice_elem_type(ctx: &LowerCtx<'_>, arg: &Expr) -> Result<Type> {
    let ty = infer_expr_type(
        arg,
        &ctx.type_env,
        &ctx.fns,
        ctx.trait_env,
        ctx.aliases,
        ctx.type_defs,
        &ctx.type_params,
        &ctx.bounds,
    )?;
    let base = base_type(&ty, ctx.aliases)?;
    match base {
        Type::Slice(inner) => Ok(*inner),
        other => anyhow::bail!("expected Slice argument, found {:?}", other),
    }
}

fn set_elem_type(ctx: &LowerCtx<'_>, arg: &Expr) -> Result<Type> {
    let ty = infer_expr_type(
        arg,
        &ctx.type_env,
        &ctx.fns,
        ctx.trait_env,
        ctx.aliases,
        ctx.type_defs,
        &ctx.type_params,
        &ctx.bounds,
    )?;
    let base = base_type(&ty, ctx.aliases)?;
    match base {
        Type::Set(inner) => Ok(*inner),
        other => anyhow::bail!("expected Set argument, found {:?}", other),
    }
}

fn map_key_val_type(ctx: &LowerCtx<'_>, arg: &Expr) -> Result<(Type, Type)> {
    let ty = infer_expr_type(
        arg,
        &ctx.type_env,
        &ctx.fns,
        ctx.trait_env,
        ctx.aliases,
        ctx.type_defs,
        &ctx.type_params,
        &ctx.bounds,
    )?;
    let base = base_type(&ty, ctx.aliases)?;
    match base {
        Type::Map(key, val) => Ok((*key, *val)),
        other => anyhow::bail!("expected Map argument, found {:?}", other),
    }
}

pub(super) fn lower_collection_call<'a>(
    ctx: &mut LowerCtx<'a>,
    call_expr: &'a Expr,
    callee: &str,
    args: &'a [Expr],
    expected: Option<&Type>,
) -> Result<Option<Value>> {
    let callee = normalize_collection_callee(callee);
    match callee {
        "std::array::len" => {
            if args.len() != 1 {
                anyhow::bail!("`std::array::len` expects one argument");
            }
            let arr_val = lower_expr(ctx, &args[0], None)?;
            Ok(Some(emit_array_len(ctx, arr_val)))
        }
        "std::slice::len" => {
            if args.len() != 1 {
                anyhow::bail!("`std::slice::len` expects one argument");
            }
            let slice_val = lower_expr(ctx, &args[0], None)?;
            Ok(Some(emit_array_len(ctx, slice_val)))
        }
        "std::slice::from_array" => {
            if args.len() != 1 {
                anyhow::bail!("`std::slice::from_array` expects one argument");
            }
            let array_val = lower_expr(ctx, &args[0], None)?;
            let len = emit_array_len(ctx, array_val);
            let data_ptr = emit_array_data_ptr(ctx, array_val);
            let header = emit_alloc(ctx, ARRAY_HEADER_SIZE, ARRAY_HEADER_ALIGN);
            ctx.body.push(Instr::Store {
                ptr: header,
                src: len,
                offset: ARRAY_HEADER_LEN_OFFSET,
                ty: IrType::Int,
            });
            ctx.body.push(Instr::Store {
                ptr: header,
                src: data_ptr,
                offset: ARRAY_HEADER_DATA_OFFSET,
                ty: IrType::Int,
            });
            Ok(Some(header))
        }
        "std::slice::sub" => {
            if args.len() != 3 {
                anyhow::bail!("`std::slice::sub` expects three arguments");
            }
            let elem_ty = slice_elem_type(ctx, &args[0])?;
            let slice_val = lower_expr(ctx, &args[0], None)?;
            let start = lower_expr(ctx, &args[1], Some(Type::Int))?;
            let len = lower_expr(ctx, &args[2], Some(Type::Int))?;
            let base_len = emit_array_len(ctx, slice_val);
            let base_ptr = emit_array_data_ptr(ctx, slice_val);
            let zero = emit_int_const(ctx, 0);
            let start_ge_zero = fresh(ctx);
            ctx.body.push(Instr::IBin {
                dst: start_ge_zero,
                op: BinOpIR::Ge,
                lhs: start,
                rhs: zero,
                ty: IrType::Int,
            });
            let len_ge_zero = fresh(ctx);
            ctx.body.push(Instr::IBin {
                dst: len_ge_zero,
                op: BinOpIR::Ge,
                lhs: len,
                rhs: zero,
                ty: IrType::Int,
            });
            let sum = fresh(ctx);
            ctx.body.push(Instr::IBin {
                dst: sum,
                op: BinOpIR::Add,
                lhs: start,
                rhs: len,
                ty: IrType::Int,
            });
            let sum_le = fresh(ctx);
            ctx.body.push(Instr::IBin {
                dst: sum_le,
                op: BinOpIR::LeU,
                lhs: sum,
                rhs: base_len,
                ty: IrType::Int,
            });
            let ok1 = fresh(ctx);
            ctx.body.push(Instr::IBin {
                dst: ok1,
                op: BinOpIR::And,
                lhs: start_ge_zero,
                rhs: len_ge_zero,
                ty: IrType::Bool,
            });
            let ok = fresh(ctx);
            ctx.body.push(Instr::IBin {
                dst: ok,
                op: BinOpIR::And,
                lhs: ok1,
                rhs: sum_le,
                ty: IrType::Bool,
            });
            emit_collection_guard(ctx, ok, expr_span_local(&args[0]));
            let (_size, _align, stride) = collection_layout(&elem_ty, ctx.aliases, ctx.std_types)?;
            let stride_val = emit_int_const(ctx, stride as i64);
            let offset_val = fresh(ctx);
            ctx.body.push(Instr::IBin {
                dst: offset_val,
                op: BinOpIR::Mul,
                lhs: start,
                rhs: stride_val,
                ty: IrType::Int,
            });
            let data_ptr = emit_ptr_add(ctx, base_ptr, offset_val);
            let header = emit_alloc(ctx, ARRAY_HEADER_SIZE, ARRAY_HEADER_ALIGN);
            ctx.body.push(Instr::Store {
                ptr: header,
                src: len,
                offset: ARRAY_HEADER_LEN_OFFSET,
                ty: IrType::Int,
            });
            ctx.body.push(Instr::Store {
                ptr: header,
                src: data_ptr,
                offset: ARRAY_HEADER_DATA_OFFSET,
                ty: IrType::Int,
            });
            Ok(Some(header))
        }
        "std::list::new" => {
            if !args.is_empty() {
                anyhow::bail!("`std::list::new` expects no arguments");
            }
            let elem_ty = match expected {
                Some(Type::List(inner)) => *inner.clone(),
                _ => {
                    let call_ty = infer_expr_type(
                        call_expr,
                        &ctx.type_env,
                        &ctx.fns,
                        ctx.trait_env,
                        ctx.aliases,
                        ctx.type_defs,
                        &ctx.type_params,
                        &ctx.bounds,
                    )?;
                    let call_ty = base_type(&call_ty, ctx.aliases)?;
                    match call_ty {
                        Type::List(inner) => *inner,
                        other => anyhow::bail!("cannot infer list element type: {:?}", other),
                    }
                }
            };
            let (_size, align, stride) = collection_layout(&elem_ty, ctx.aliases, ctx.std_types)?;
            let len = emit_int_const(ctx, 0);
            let cap = emit_int_const(ctx, 1);
            let buf_bytes = emit_int_const(ctx, stride as i64);
            let data_ptr = emit_alloc_dyn(ctx, buf_bytes, align);
            let header = emit_collection_header(ctx, len, cap, data_ptr);
            Ok(Some(header))
        }
        "std::list::len" => {
            if args.len() != 1 {
                anyhow::bail!("`std::list::len` expects one argument");
            }
            let list_val = lower_expr(ctx, &args[0], None)?;
            Ok(Some(emit_collection_len(ctx, list_val)))
        }
        "std::list::get" => {
            if args.len() != 2 {
                anyhow::bail!("`std::list::get` expects two arguments");
            }
            let elem_ty = list_elem_type(ctx, &args[0])?;
            let list_val = lower_expr(ctx, &args[0], None)?;
            let idx = lower_expr(ctx, &args[1], Some(Type::Int))?;
            let len = emit_collection_len(ctx, list_val);
            let zero = emit_int_const(ctx, 0);
            let idx_ge_zero = fresh(ctx);
            ctx.body.push(Instr::IBin {
                dst: idx_ge_zero,
                op: BinOpIR::Ge,
                lhs: idx,
                rhs: zero,
                ty: IrType::Int,
            });
            let idx_lt_len = fresh(ctx);
            ctx.body.push(Instr::IBin {
                dst: idx_lt_len,
                op: BinOpIR::Lt,
                lhs: idx,
                rhs: len,
                ty: IrType::Int,
            });
            let ok = fresh(ctx);
            ctx.body.push(Instr::IBin {
                dst: ok,
                op: BinOpIR::And,
                lhs: idx_ge_zero,
                rhs: idx_lt_len,
                ty: IrType::Int,
            });
            let safe_idx = fresh(ctx);
            ctx.body.push(Instr::ISelect {
                dst: safe_idx,
                cond: ok,
                then_v: idx,
                else_v: zero,
            });
            let (_size, align, stride) = collection_layout(&elem_ty, ctx.aliases, ctx.std_types)?;
            let stride_val = emit_int_const(ctx, stride as i64);
            let offset = fresh(ctx);
            ctx.body.push(Instr::IBin {
                dst: offset,
                op: BinOpIR::Mul,
                lhs: safe_idx,
                rhs: stride_val,
                ty: IrType::Int,
            });
            let data_ptr = emit_collection_data_ptr(ctx, list_val);
            let cap = emit_collection_cap(ctx, list_val);
            emit_collection_payload_guard(ctx, data_ptr, len, cap, stride, align);
            let elem_ptr = emit_ptr_add(ctx, data_ptr, offset);
            let elem_val = load_value_copy(ctx, &elem_ty, elem_ptr, 0)?;
            let zero_payload = zero_value_for_type(ctx, &elem_ty)?;
            let payload = fresh(ctx);
            ctx.body.push(Instr::ISelect {
                dst: payload,
                cond: ok,
                then_v: elem_val,
                else_v: zero_payload,
            });
            let zero_hi = emit_int_const(ctx, 0);
            Ok(Some(ctx.variant_init(ok, payload, zero_hi)))
        }
        "std::list::push" => {
            if args.len() != 2 {
                anyhow::bail!("`std::list::push` expects two arguments");
            }
            let elem_ty = list_elem_type(ctx, &args[0])?;
            let list_val = lower_expr(ctx, &args[0], None)?;
            let elem_val = lower_expr(ctx, &args[1], Some(elem_ty.clone()))?;
            let len = emit_collection_len(ctx, list_val);
            let one = emit_int_const(ctx, 1);
            let new_len = fresh(ctx);
            ctx.body.push(Instr::IBin {
                dst: new_len,
                op: BinOpIR::Add,
                lhs: len,
                rhs: one,
                ty: IrType::Int,
            });
            let (_size, align, stride) = collection_layout(&elem_ty, ctx.aliases, ctx.std_types)?;
            let stride_val = emit_int_const(ctx, stride as i64);
            let new_cap = emit_cap_from_len(ctx, new_len);
            let buf_bytes = fresh(ctx);
            ctx.body.push(Instr::IBin {
                dst: buf_bytes,
                op: BinOpIR::Mul,
                lhs: new_cap,
                rhs: stride_val,
                ty: IrType::Int,
            });
            let new_data = emit_alloc_dyn(ctx, buf_bytes, align);
            let old_data = emit_collection_data_ptr(ctx, list_val);
            let cap = emit_collection_cap(ctx, list_val);
            emit_collection_payload_guard(ctx, old_data, len, cap, stride, align);
            let copy_bytes = fresh(ctx);
            ctx.body.push(Instr::IBin {
                dst: copy_bytes,
                op: BinOpIR::Mul,
                lhs: len,
                rhs: stride_val,
                ty: IrType::Int,
            });
            emit_memcpy_bytes(ctx, old_data, new_data, copy_bytes)?;
            let offset = fresh(ctx);
            ctx.body.push(Instr::IBin {
                dst: offset,
                op: BinOpIR::Mul,
                lhs: len,
                rhs: stride_val,
                ty: IrType::Int,
            });
            let elem_ptr = emit_ptr_add(ctx, new_data, offset);
            store_value(ctx, &elem_ty, elem_ptr, 0, elem_val)?;
            let header = emit_collection_header(ctx, new_len, new_cap, new_data);
            Ok(Some(header))
        }
        "std::list::insert" => {
            if args.len() != 3 {
                anyhow::bail!("`std::list::insert` expects three arguments");
            }
            let elem_ty = list_elem_type(ctx, &args[0])?;
            let list_val = lower_expr(ctx, &args[0], None)?;
            let elem_val = lower_expr(ctx, &args[1], Some(elem_ty.clone()))?;
            let idx = lower_expr(ctx, &args[2], Some(Type::Int))?;
            let len = emit_collection_len(ctx, list_val);
            let zero = emit_int_const(ctx, 0);
            let idx_ge_zero = fresh(ctx);
            ctx.body.push(Instr::IBin {
                dst: idx_ge_zero,
                op: BinOpIR::Ge,
                lhs: idx,
                rhs: zero,
                ty: IrType::Int,
            });
            let idx_le_len = fresh(ctx);
            ctx.body.push(Instr::IBin {
                dst: idx_le_len,
                op: BinOpIR::Le,
                lhs: idx,
                rhs: len,
                ty: IrType::Int,
            });
            let ok = fresh(ctx);
            ctx.body.push(Instr::IBin {
                dst: ok,
                op: BinOpIR::And,
                lhs: idx_ge_zero,
                rhs: idx_le_len,
                ty: IrType::Int,
            });
            emit_collection_guard(ctx, ok, expr_span_local(&args[2]));
            let one = emit_int_const(ctx, 1);
            let new_len = fresh(ctx);
            ctx.body.push(Instr::IBin {
                dst: new_len,
                op: BinOpIR::Add,
                lhs: len,
                rhs: one,
                ty: IrType::Int,
            });
            let (_size, align, stride) = collection_layout(&elem_ty, ctx.aliases, ctx.std_types)?;
            let stride_val = emit_int_const(ctx, stride as i64);
            let new_cap = emit_cap_from_len(ctx, new_len);
            let buf_bytes = fresh(ctx);
            ctx.body.push(Instr::IBin {
                dst: buf_bytes,
                op: BinOpIR::Mul,
                lhs: new_cap,
                rhs: stride_val,
                ty: IrType::Int,
            });
            let new_data = emit_alloc_dyn(ctx, buf_bytes, align);
            let old_data = emit_collection_data_ptr(ctx, list_val);
            let cap = emit_collection_cap(ctx, list_val);
            emit_collection_payload_guard(ctx, old_data, len, cap, stride, align);
            let bytes_before = fresh(ctx);
            ctx.body.push(Instr::IBin {
                dst: bytes_before,
                op: BinOpIR::Mul,
                lhs: idx,
                rhs: stride_val,
                ty: IrType::Int,
            });
            emit_memcpy_bytes(ctx, old_data, new_data, bytes_before)?;
            let offset = fresh(ctx);
            ctx.body.push(Instr::IBin {
                dst: offset,
                op: BinOpIR::Mul,
                lhs: idx,
                rhs: stride_val,
                ty: IrType::Int,
            });
            let elem_ptr = emit_ptr_add(ctx, new_data, offset);
            store_value(ctx, &elem_ty, elem_ptr, 0, elem_val)?;
            let idx_plus_one = fresh(ctx);
            ctx.body.push(Instr::IBin {
                dst: idx_plus_one,
                op: BinOpIR::Add,
                lhs: idx,
                rhs: one,
                ty: IrType::Int,
            });
            let remaining = fresh(ctx);
            ctx.body.push(Instr::IBin {
                dst: remaining,
                op: BinOpIR::Sub,
                lhs: len,
                rhs: idx,
                ty: IrType::Int,
            });
            let bytes_after = fresh(ctx);
            ctx.body.push(Instr::IBin {
                dst: bytes_after,
                op: BinOpIR::Mul,
                lhs: remaining,
                rhs: stride_val,
                ty: IrType::Int,
            });
            let src_ptr = emit_ptr_add(ctx, old_data, offset);
            let dst_offset = fresh(ctx);
            ctx.body.push(Instr::IBin {
                dst: dst_offset,
                op: BinOpIR::Mul,
                lhs: idx_plus_one,
                rhs: stride_val,
                ty: IrType::Int,
            });
            let dst_ptr = emit_ptr_add(ctx, new_data, dst_offset);
            emit_memcpy_bytes(ctx, src_ptr, dst_ptr, bytes_after)?;
            let header = emit_collection_header(ctx, new_len, new_cap, new_data);
            Ok(Some(header))
        }
        "std::list::remove" => {
            if args.len() != 2 {
                anyhow::bail!("`std::list::remove` expects two arguments");
            }
            let elem_ty = list_elem_type(ctx, &args[0])?;
            let list_val = lower_expr(ctx, &args[0], None)?;
            let idx = lower_expr(ctx, &args[1], Some(Type::Int))?;
            let len = emit_collection_len(ctx, list_val);
            let zero = emit_int_const(ctx, 0);
            let idx_ge_zero = fresh(ctx);
            ctx.body.push(Instr::IBin {
                dst: idx_ge_zero,
                op: BinOpIR::Ge,
                lhs: idx,
                rhs: zero,
                ty: IrType::Int,
            });
            let idx_lt_len = fresh(ctx);
            ctx.body.push(Instr::IBin {
                dst: idx_lt_len,
                op: BinOpIR::Lt,
                lhs: idx,
                rhs: len,
                ty: IrType::Int,
            });
            let ok = fresh(ctx);
            ctx.body.push(Instr::IBin {
                dst: ok,
                op: BinOpIR::And,
                lhs: idx_ge_zero,
                rhs: idx_lt_len,
                ty: IrType::Int,
            });
            emit_collection_guard(ctx, ok, expr_span_local(&args[1]));
            let one = emit_int_const(ctx, 1);
            let new_len = fresh(ctx);
            ctx.body.push(Instr::IBin {
                dst: new_len,
                op: BinOpIR::Sub,
                lhs: len,
                rhs: one,
                ty: IrType::Int,
            });
            let (_size, align, stride) = collection_layout(&elem_ty, ctx.aliases, ctx.std_types)?;
            let stride_val = emit_int_const(ctx, stride as i64);
            let new_cap = emit_cap_from_len(ctx, new_len);
            let buf_bytes = fresh(ctx);
            ctx.body.push(Instr::IBin {
                dst: buf_bytes,
                op: BinOpIR::Mul,
                lhs: new_cap,
                rhs: stride_val,
                ty: IrType::Int,
            });
            let new_data = emit_alloc_dyn(ctx, buf_bytes, align);
            let old_data = emit_collection_data_ptr(ctx, list_val);
            let cap = emit_collection_cap(ctx, list_val);
            emit_collection_payload_guard(ctx, old_data, len, cap, stride, align);
            let bytes_before = fresh(ctx);
            ctx.body.push(Instr::IBin {
                dst: bytes_before,
                op: BinOpIR::Mul,
                lhs: idx,
                rhs: stride_val,
                ty: IrType::Int,
            });
            emit_memcpy_bytes(ctx, old_data, new_data, bytes_before)?;
            let idx_plus_one = fresh(ctx);
            ctx.body.push(Instr::IBin {
                dst: idx_plus_one,
                op: BinOpIR::Add,
                lhs: idx,
                rhs: one,
                ty: IrType::Int,
            });
            let remaining = fresh(ctx);
            ctx.body.push(Instr::IBin {
                dst: remaining,
                op: BinOpIR::Sub,
                lhs: len,
                rhs: idx_plus_one,
                ty: IrType::Int,
            });
            let bytes_after = fresh(ctx);
            ctx.body.push(Instr::IBin {
                dst: bytes_after,
                op: BinOpIR::Mul,
                lhs: remaining,
                rhs: stride_val,
                ty: IrType::Int,
            });
            let src_offset = fresh(ctx);
            ctx.body.push(Instr::IBin {
                dst: src_offset,
                op: BinOpIR::Mul,
                lhs: idx_plus_one,
                rhs: stride_val,
                ty: IrType::Int,
            });
            let src_ptr = emit_ptr_add(ctx, old_data, src_offset);
            let dst_offset = fresh(ctx);
            ctx.body.push(Instr::IBin {
                dst: dst_offset,
                op: BinOpIR::Mul,
                lhs: idx,
                rhs: stride_val,
                ty: IrType::Int,
            });
            let dst_ptr = emit_ptr_add(ctx, new_data, dst_offset);
            emit_memcpy_bytes(ctx, src_ptr, dst_ptr, bytes_after)?;
            let header = emit_collection_header(ctx, new_len, new_cap, new_data);
            Ok(Some(header))
        }
        "std::list::pop" => {
            if args.len() != 1 {
                anyhow::bail!("`std::list::pop` expects one argument");
            }
            let elem_ty = list_elem_type(ctx, &args[0])?;
            let list_val = lower_expr(ctx, &args[0], None)?;
            let len = emit_collection_len(ctx, list_val);
            let zero = emit_int_const(ctx, 0);
            let ok = fresh(ctx);
            ctx.body.push(Instr::IBin {
                dst: ok,
                op: BinOpIR::Gt,
                lhs: len,
                rhs: zero,
                ty: IrType::Int,
            });
            let one = emit_int_const(ctx, 1);
            let idx = fresh(ctx);
            ctx.body.push(Instr::IBin {
                dst: idx,
                op: BinOpIR::Sub,
                lhs: len,
                rhs: one,
                ty: IrType::Int,
            });
            let safe_idx = fresh(ctx);
            ctx.body.push(Instr::ISelect {
                dst: safe_idx,
                cond: ok,
                then_v: idx,
                else_v: zero,
            });
            let (_size, align, stride) = collection_layout(&elem_ty, ctx.aliases, ctx.std_types)?;
            let stride_val = emit_int_const(ctx, stride as i64);
            let offset = fresh(ctx);
            ctx.body.push(Instr::IBin {
                dst: offset,
                op: BinOpIR::Mul,
                lhs: safe_idx,
                rhs: stride_val,
                ty: IrType::Int,
            });
            let data_ptr = emit_collection_data_ptr(ctx, list_val);
            let cap = emit_collection_cap(ctx, list_val);
            emit_collection_payload_guard(ctx, data_ptr, len, cap, stride, align);
            let elem_ptr = emit_ptr_add(ctx, data_ptr, offset);
            let elem_val = load_value_copy(ctx, &elem_ty, elem_ptr, 0)?;
            let zero_payload = zero_value_for_type(ctx, &elem_ty)?;
            let payload = fresh(ctx);
            ctx.body.push(Instr::ISelect {
                dst: payload,
                cond: ok,
                then_v: elem_val,
                else_v: zero_payload,
            });
            let zero_hi = emit_int_const(ctx, 0);
            Ok(Some(ctx.variant_init(ok, payload, zero_hi)))
        }
        "std::set::new" => {
            if !args.is_empty() {
                anyhow::bail!("`std::set::new` expects no arguments");
            }
            let elem_ty = match expected {
                Some(Type::Set(inner)) => *inner.clone(),
                _ => {
                    let call_ty = infer_expr_type(
                        call_expr,
                        &ctx.type_env,
                        &ctx.fns,
                        ctx.trait_env,
                        ctx.aliases,
                        ctx.type_defs,
                        &ctx.type_params,
                        &ctx.bounds,
                    )?;
                    let call_ty = base_type(&call_ty, ctx.aliases)?;
                    match call_ty {
                        Type::Set(inner) => *inner,
                        other => anyhow::bail!("cannot infer set element type: {:?}", other),
                    }
                }
            };
            let (_size, align, stride) = collection_layout(&elem_ty, ctx.aliases, ctx.std_types)?;
            let len = emit_int_const(ctx, 0);
            let cap = emit_int_const(ctx, 1);
            let buf_bytes = emit_int_const(ctx, stride as i64);
            let data_ptr = emit_alloc_dyn(ctx, buf_bytes, align);
            let header = emit_collection_header(ctx, len, cap, data_ptr);
            Ok(Some(header))
        }
        "std::set::len" => {
            if args.len() != 1 {
                anyhow::bail!("`std::set::len` expects one argument");
            }
            let set_val = lower_expr(ctx, &args[0], None)?;
            Ok(Some(emit_collection_len(ctx, set_val)))
        }
        "std::set::contains" => {
            if args.len() != 2 {
                anyhow::bail!("`std::set::contains` expects two arguments");
            }
            let elem_ty = set_elem_type(ctx, &args[0])?;
            let set_val = lower_expr(ctx, &args[0], None)?;
            let elem_val = lower_expr(ctx, &args[1], Some(elem_ty.clone()))?;
            let len = emit_collection_len(ctx, set_val);
            let data_ptr = emit_collection_data_ptr(ctx, set_val);
            let (_size, align, stride) = collection_layout(&elem_ty, ctx.aliases, ctx.std_types)?;
            let cap = emit_collection_cap(ctx, set_val);
            emit_collection_payload_guard(ctx, data_ptr, len, cap, stride, align);
            let (found, _idx) =
                emit_find_index(ctx, data_ptr, len, stride, elem_val, &elem_ty, 0, ctx.aliases)?;
            Ok(Some(found))
        }
        "std::set::insert" => {
            if args.len() != 2 {
                anyhow::bail!("`std::set::insert` expects two arguments");
            }
            let elem_ty = set_elem_type(ctx, &args[0])?;
            let set_val = lower_expr(ctx, &args[0], None)?;
            let elem_val = lower_expr(ctx, &args[1], Some(elem_ty.clone()))?;
            let len = emit_collection_len(ctx, set_val);
            let data_ptr = emit_collection_data_ptr(ctx, set_val);
            let (_size, align, stride) = collection_layout(&elem_ty, ctx.aliases, ctx.std_types)?;
            let cap = emit_collection_cap(ctx, set_val);
            emit_collection_payload_guard(ctx, data_ptr, len, cap, stride, align);
            let (found, _idx) =
                emit_find_index(ctx, data_ptr, len, stride, elem_val, &elem_ty, 0, ctx.aliases)?;
            let one = emit_int_const(ctx, 1);
            let len_plus_one = fresh(ctx);
            ctx.body.push(Instr::IBin {
                dst: len_plus_one,
                op: BinOpIR::Add,
                lhs: len,
                rhs: one,
                ty: IrType::Int,
            });
            let new_len = fresh(ctx);
            ctx.body.push(Instr::ISelect {
                dst: new_len,
                cond: found,
                then_v: len,
                else_v: len_plus_one,
            });
            let stride_val = emit_int_const(ctx, stride as i64);
            let new_cap = emit_cap_from_len(ctx, new_len);
            let buf_bytes = fresh(ctx);
            ctx.body.push(Instr::IBin {
                dst: buf_bytes,
                op: BinOpIR::Mul,
                lhs: new_cap,
                rhs: stride_val,
                ty: IrType::Int,
            });
            let new_data = emit_alloc_dyn(ctx, buf_bytes, align);
            let copy_bytes = fresh(ctx);
            ctx.body.push(Instr::IBin {
                dst: copy_bytes,
                op: BinOpIR::Mul,
                lhs: len,
                rhs: stride_val,
                ty: IrType::Int,
            });
            emit_memcpy_bytes(ctx, data_ptr, new_data, copy_bytes)?;
            ctx.body.push(Instr::BlockBegin);
            ctx.body.push(Instr::BrIf { cond: found, depth: 0 });
            let offset = fresh(ctx);
            ctx.body.push(Instr::IBin {
                dst: offset,
                op: BinOpIR::Mul,
                lhs: len,
                rhs: stride_val,
                ty: IrType::Int,
            });
            let elem_ptr = emit_ptr_add(ctx, new_data, offset);
            store_value(ctx, &elem_ty, elem_ptr, 0, elem_val)?;
            ctx.body.push(Instr::BlockEnd);
            let header = emit_collection_header(ctx, new_len, new_cap, new_data);
            Ok(Some(header))
        }
        "std::set::remove" => {
            if args.len() != 2 {
                anyhow::bail!("`std::set::remove` expects two arguments");
            }
            let elem_ty = set_elem_type(ctx, &args[0])?;
            let set_val = lower_expr(ctx, &args[0], None)?;
            let elem_val = lower_expr(ctx, &args[1], Some(elem_ty.clone()))?;
            let len = emit_collection_len(ctx, set_val);
            let data_ptr = emit_collection_data_ptr(ctx, set_val);
            let (_size, align, stride) = collection_layout(&elem_ty, ctx.aliases, ctx.std_types)?;
            let cap = emit_collection_cap(ctx, set_val);
            emit_collection_payload_guard(ctx, data_ptr, len, cap, stride, align);
            let (found, found_idx) =
                emit_find_index(ctx, data_ptr, len, stride, elem_val, &elem_ty, 0, ctx.aliases)?;
            let one = emit_int_const(ctx, 1);
            let len_minus_one = fresh(ctx);
            ctx.body.push(Instr::IBin {
                dst: len_minus_one,
                op: BinOpIR::Sub,
                lhs: len,
                rhs: one,
                ty: IrType::Int,
            });
            let new_len = fresh(ctx);
            ctx.body.push(Instr::ISelect {
                dst: new_len,
                cond: found,
                then_v: len_minus_one,
                else_v: len,
            });
            let stride_val = emit_int_const(ctx, stride as i64);
            let new_cap = emit_cap_from_len(ctx, new_len);
            let buf_bytes = fresh(ctx);
            ctx.body.push(Instr::IBin {
                dst: buf_bytes,
                op: BinOpIR::Mul,
                lhs: new_cap,
                rhs: stride_val,
                ty: IrType::Int,
            });
            let new_data = emit_alloc_dyn(ctx, buf_bytes, align);
            ctx.body.push(Instr::BlockBegin);
            ctx.body.push(Instr::BlockBegin);
            ctx.body.push(Instr::BrIfEqz { cond: found, depth: 0 });
            let bytes_before = fresh(ctx);
            ctx.body.push(Instr::IBin {
                dst: bytes_before,
                op: BinOpIR::Mul,
                lhs: found_idx,
                rhs: stride_val,
                ty: IrType::Int,
            });
            emit_memcpy_bytes(ctx, data_ptr, new_data, bytes_before)?;
            let idx_plus_one = fresh(ctx);
            ctx.body.push(Instr::IBin {
                dst: idx_plus_one,
                op: BinOpIR::Add,
                lhs: found_idx,
                rhs: one,
                ty: IrType::Int,
            });
            let remaining = fresh(ctx);
            ctx.body.push(Instr::IBin {
                dst: remaining,
                op: BinOpIR::Sub,
                lhs: len,
                rhs: idx_plus_one,
                ty: IrType::Int,
            });
            let bytes_after = fresh(ctx);
            ctx.body.push(Instr::IBin {
                dst: bytes_after,
                op: BinOpIR::Mul,
                lhs: remaining,
                rhs: stride_val,
                ty: IrType::Int,
            });
            let src_offset = fresh(ctx);
            ctx.body.push(Instr::IBin {
                dst: src_offset,
                op: BinOpIR::Mul,
                lhs: idx_plus_one,
                rhs: stride_val,
                ty: IrType::Int,
            });
            let src_ptr = emit_ptr_add(ctx, data_ptr, src_offset);
            let dst_offset = fresh(ctx);
            ctx.body.push(Instr::IBin {
                dst: dst_offset,
                op: BinOpIR::Mul,
                lhs: found_idx,
                rhs: stride_val,
                ty: IrType::Int,
            });
            let dst_ptr = emit_ptr_add(ctx, new_data, dst_offset);
            emit_memcpy_bytes(ctx, src_ptr, dst_ptr, bytes_after)?;
            ctx.body.push(Instr::Br { depth: 1 });
            ctx.body.push(Instr::BlockEnd);
            let copy_bytes = fresh(ctx);
            ctx.body.push(Instr::IBin {
                dst: copy_bytes,
                op: BinOpIR::Mul,
                lhs: len,
                rhs: stride_val,
                ty: IrType::Int,
            });
            emit_memcpy_bytes(ctx, data_ptr, new_data, copy_bytes)?;
            ctx.body.push(Instr::BlockEnd);
            let header = emit_collection_header(ctx, new_len, new_cap, new_data);
            Ok(Some(header))
        }
        "std::map::new" => {
            if !args.is_empty() {
                anyhow::bail!("`std::map::new` expects no arguments");
            }
            let (key_ty, val_ty) = match expected {
                Some(Type::Map(key, val)) => (*key.clone(), *val.clone()),
                _ => {
                    let call_ty = infer_expr_type(
                        call_expr,
                        &ctx.type_env,
                        &ctx.fns,
                        ctx.trait_env,
                        ctx.aliases,
                        ctx.type_defs,
                        &ctx.type_params,
                        &ctx.bounds,
                    )?;
                    let call_ty = base_type(&call_ty, ctx.aliases)?;
                    match call_ty {
                        Type::Map(key, val) => (*key, *val),
                        other => anyhow::bail!("cannot infer map type: {:?}", other),
                    }
                }
            };
            let (entry_size, entry_align, _key_offset, _val_offset) =
                map_entry_layout(&key_ty, &val_ty, ctx.aliases, ctx.std_types)?;
            let len = emit_int_const(ctx, 0);
            let cap = emit_int_const(ctx, 1);
            let buf_bytes = emit_int_const(ctx, entry_size as i64);
            let data_ptr = emit_alloc_dyn(ctx, buf_bytes, entry_align);
            let header = emit_collection_header(ctx, len, cap, data_ptr);
            Ok(Some(header))
        }
        "std::map::len" => {
            if args.len() != 1 {
                anyhow::bail!("`std::map::len` expects one argument");
            }
            let map_val = lower_expr(ctx, &args[0], None)?;
            Ok(Some(emit_collection_len(ctx, map_val)))
        }
        "std::map::contains" => {
            if args.len() != 2 {
                anyhow::bail!("`std::map::contains` expects two arguments");
            }
            let (key_ty, val_ty) = map_key_val_type(ctx, &args[0])?;
            let map_val = lower_expr(ctx, &args[0], None)?;
            let key_val = lower_expr(ctx, &args[1], Some(key_ty.clone()))?;
            let len = emit_collection_len(ctx, map_val);
            let data_ptr = emit_collection_data_ptr(ctx, map_val);
            let (entry_size, entry_align, key_offset, _val_offset) =
                map_entry_layout(&key_ty, &val_ty, ctx.aliases, ctx.std_types)?;
            let cap = emit_collection_cap(ctx, map_val);
            emit_collection_payload_guard(ctx, data_ptr, len, cap, entry_size, entry_align);
            let (found, _idx) = emit_find_index(
                ctx,
                data_ptr,
                len,
                entry_size,
                key_val,
                &key_ty,
                key_offset,
                ctx.aliases,
            )?;
            Ok(Some(found))
        }
        "std::map::get" => {
            if args.len() != 2 {
                anyhow::bail!("`std::map::get` expects two arguments");
            }
            let (key_ty, val_ty) = map_key_val_type(ctx, &args[0])?;
            let map_val = lower_expr(ctx, &args[0], None)?;
            let key_val = lower_expr(ctx, &args[1], Some(key_ty.clone()))?;
            let len = emit_collection_len(ctx, map_val);
            let data_ptr = emit_collection_data_ptr(ctx, map_val);
            let (entry_size, entry_align, key_offset, val_offset) =
                map_entry_layout(&key_ty, &val_ty, ctx.aliases, ctx.std_types)?;
            let cap = emit_collection_cap(ctx, map_val);
            emit_collection_payload_guard(ctx, data_ptr, len, cap, entry_size, entry_align);
            let (found, found_idx) = emit_find_index(
                ctx,
                data_ptr,
                len,
                entry_size,
                key_val,
                &key_ty,
                key_offset,
                ctx.aliases,
            )?;
            let zero = emit_int_const(ctx, 0);
            let safe_idx = fresh(ctx);
            ctx.body.push(Instr::ISelect {
                dst: safe_idx,
                cond: found,
                then_v: found_idx,
                else_v: zero,
            });
            let stride_val = emit_int_const(ctx, entry_size as i64);
            let offset = fresh(ctx);
            ctx.body.push(Instr::IBin {
                dst: offset,
                op: BinOpIR::Mul,
                lhs: safe_idx,
                rhs: stride_val,
                ty: IrType::Int,
            });
            let base_ptr = emit_ptr_add(ctx, data_ptr, offset);
            let val_ptr = if val_offset == 0 {
                base_ptr
            } else {
                let val_off_val = emit_int_const(ctx, val_offset as i64);
                emit_ptr_add(ctx, base_ptr, val_off_val)
            };
            let val_loaded = load_value_copy(ctx, &val_ty, val_ptr, 0)?;
            let zero_payload = zero_value_for_type(ctx, &val_ty)?;
            let payload = fresh(ctx);
            ctx.body.push(Instr::ISelect {
                dst: payload,
                cond: found,
                then_v: val_loaded,
                else_v: zero_payload,
            });
            let zero_hi = emit_int_const(ctx, 0);
            Ok(Some(ctx.variant_init(found, payload, zero_hi)))
        }
        "std::map::insert" => {
            if args.len() != 3 {
                anyhow::bail!("`std::map::insert` expects three arguments");
            }
            let (key_ty, val_ty) = map_key_val_type(ctx, &args[0])?;
            let map_val = lower_expr(ctx, &args[0], None)?;
            let key_val = lower_expr(ctx, &args[1], Some(key_ty.clone()))?;
            let val_val = lower_expr(ctx, &args[2], Some(val_ty.clone()))?;
            let len = emit_collection_len(ctx, map_val);
            let data_ptr = emit_collection_data_ptr(ctx, map_val);
            let (entry_size, entry_align, key_offset, val_offset) =
                map_entry_layout(&key_ty, &val_ty, ctx.aliases, ctx.std_types)?;
            let cap = emit_collection_cap(ctx, map_val);
            emit_collection_payload_guard(ctx, data_ptr, len, cap, entry_size, entry_align);
            let (found, found_idx) = emit_find_index(
                ctx,
                data_ptr,
                len,
                entry_size,
                key_val,
                &key_ty,
                key_offset,
                ctx.aliases,
            )?;
            let one = emit_int_const(ctx, 1);
            let len_plus_one = fresh(ctx);
            ctx.body.push(Instr::IBin {
                dst: len_plus_one,
                op: BinOpIR::Add,
                lhs: len,
                rhs: one,
                ty: IrType::Int,
            });
            let new_len = fresh(ctx);
            ctx.body.push(Instr::ISelect {
                dst: new_len,
                cond: found,
                then_v: len,
                else_v: len_plus_one,
            });
            let stride_val = emit_int_const(ctx, entry_size as i64);
            let new_cap = emit_cap_from_len(ctx, new_len);
            let buf_bytes = fresh(ctx);
            ctx.body.push(Instr::IBin {
                dst: buf_bytes,
                op: BinOpIR::Mul,
                lhs: new_cap,
                rhs: stride_val,
                ty: IrType::Int,
            });
            let new_data = emit_alloc_dyn(ctx, buf_bytes, entry_align);
            let copy_bytes = fresh(ctx);
            ctx.body.push(Instr::IBin {
                dst: copy_bytes,
                op: BinOpIR::Mul,
                lhs: len,
                rhs: stride_val,
                ty: IrType::Int,
            });
            emit_memcpy_bytes(ctx, data_ptr, new_data, copy_bytes)?;
            ctx.body.push(Instr::BlockBegin);
            ctx.body.push(Instr::BlockBegin);
            ctx.body.push(Instr::BrIfEqz { cond: found, depth: 0 });
            let found_offset = fresh(ctx);
            ctx.body.push(Instr::IBin {
                dst: found_offset,
                op: BinOpIR::Mul,
                lhs: found_idx,
                rhs: stride_val,
                ty: IrType::Int,
            });
            let base_ptr = emit_ptr_add(ctx, new_data, found_offset);
            let val_ptr = if val_offset == 0 {
                base_ptr
            } else {
                let val_off_val = emit_int_const(ctx, val_offset as i64);
                emit_ptr_add(ctx, base_ptr, val_off_val)
            };
            store_value(ctx, &val_ty, val_ptr, 0, val_val)?;
            ctx.body.push(Instr::Br { depth: 1 });
            ctx.body.push(Instr::BlockEnd);
            let offset = fresh(ctx);
            ctx.body.push(Instr::IBin {
                dst: offset,
                op: BinOpIR::Mul,
                lhs: len,
                rhs: stride_val,
                ty: IrType::Int,
            });
            let base_ptr = emit_ptr_add(ctx, new_data, offset);
            let key_ptr = if key_offset == 0 {
                base_ptr
            } else {
                let key_off_val = emit_int_const(ctx, key_offset as i64);
                emit_ptr_add(ctx, base_ptr, key_off_val)
            };
            store_value(ctx, &key_ty, key_ptr, 0, key_val)?;
            let val_ptr = if val_offset == 0 {
                base_ptr
            } else {
                let val_off_val = emit_int_const(ctx, val_offset as i64);
                emit_ptr_add(ctx, base_ptr, val_off_val)
            };
            store_value(ctx, &val_ty, val_ptr, 0, val_val)?;
            ctx.body.push(Instr::BlockEnd);
            let header = emit_collection_header(ctx, new_len, new_cap, new_data);
            Ok(Some(header))
        }
        "std::map::remove" => {
            if args.len() != 2 {
                anyhow::bail!("`std::map::remove` expects two arguments");
            }
            let (key_ty, val_ty) = map_key_val_type(ctx, &args[0])?;
            let map_val = lower_expr(ctx, &args[0], None)?;
            let key_val = lower_expr(ctx, &args[1], Some(key_ty.clone()))?;
            let len = emit_collection_len(ctx, map_val);
            let data_ptr = emit_collection_data_ptr(ctx, map_val);
            let (entry_size, entry_align, key_offset, _val_offset) =
                map_entry_layout(&key_ty, &val_ty, ctx.aliases, ctx.std_types)?;
            let cap = emit_collection_cap(ctx, map_val);
            emit_collection_payload_guard(ctx, data_ptr, len, cap, entry_size, entry_align);
            let (found, found_idx) = emit_find_index(
                ctx,
                data_ptr,
                len,
                entry_size,
                key_val,
                &key_ty,
                key_offset,
                ctx.aliases,
            )?;
            let one = emit_int_const(ctx, 1);
            let len_minus_one = fresh(ctx);
            ctx.body.push(Instr::IBin {
                dst: len_minus_one,
                op: BinOpIR::Sub,
                lhs: len,
                rhs: one,
                ty: IrType::Int,
            });
            let new_len = fresh(ctx);
            ctx.body.push(Instr::ISelect {
                dst: new_len,
                cond: found,
                then_v: len_minus_one,
                else_v: len,
            });
            let stride_val = emit_int_const(ctx, entry_size as i64);
            let new_cap = emit_cap_from_len(ctx, new_len);
            let buf_bytes = fresh(ctx);
            ctx.body.push(Instr::IBin {
                dst: buf_bytes,
                op: BinOpIR::Mul,
                lhs: new_cap,
                rhs: stride_val,
                ty: IrType::Int,
            });
            let new_data = emit_alloc_dyn(ctx, buf_bytes, entry_align);
            ctx.body.push(Instr::BlockBegin);
            ctx.body.push(Instr::BlockBegin);
            ctx.body.push(Instr::BrIfEqz { cond: found, depth: 0 });
            let bytes_before = fresh(ctx);
            ctx.body.push(Instr::IBin {
                dst: bytes_before,
                op: BinOpIR::Mul,
                lhs: found_idx,
                rhs: stride_val,
                ty: IrType::Int,
            });
            emit_memcpy_bytes(ctx, data_ptr, new_data, bytes_before)?;
            let idx_plus_one = fresh(ctx);
            ctx.body.push(Instr::IBin {
                dst: idx_plus_one,
                op: BinOpIR::Add,
                lhs: found_idx,
                rhs: one,
                ty: IrType::Int,
            });
            let remaining = fresh(ctx);
            ctx.body.push(Instr::IBin {
                dst: remaining,
                op: BinOpIR::Sub,
                lhs: len,
                rhs: idx_plus_one,
                ty: IrType::Int,
            });
            let bytes_after = fresh(ctx);
            ctx.body.push(Instr::IBin {
                dst: bytes_after,
                op: BinOpIR::Mul,
                lhs: remaining,
                rhs: stride_val,
                ty: IrType::Int,
            });
            let src_offset = fresh(ctx);
            ctx.body.push(Instr::IBin {
                dst: src_offset,
                op: BinOpIR::Mul,
                lhs: idx_plus_one,
                rhs: stride_val,
                ty: IrType::Int,
            });
            let src_ptr = emit_ptr_add(ctx, data_ptr, src_offset);
            let dst_offset = fresh(ctx);
            ctx.body.push(Instr::IBin {
                dst: dst_offset,
                op: BinOpIR::Mul,
                lhs: found_idx,
                rhs: stride_val,
                ty: IrType::Int,
            });
            let dst_ptr = emit_ptr_add(ctx, new_data, dst_offset);
            emit_memcpy_bytes(ctx, src_ptr, dst_ptr, bytes_after)?;
            ctx.body.push(Instr::Br { depth: 1 });
            ctx.body.push(Instr::BlockEnd);
            let copy_bytes = fresh(ctx);
            ctx.body.push(Instr::IBin {
                dst: copy_bytes,
                op: BinOpIR::Mul,
                lhs: len,
                rhs: stride_val,
                ty: IrType::Int,
            });
            emit_memcpy_bytes(ctx, data_ptr, new_data, copy_bytes)?;
            ctx.body.push(Instr::BlockEnd);
            let header = emit_collection_header(ctx, new_len, new_cap, new_data);
            Ok(Some(header))
        }
        _ => Ok(None),
    }
}

