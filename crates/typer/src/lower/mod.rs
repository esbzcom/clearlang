use crate::check::{
    base_type, infer_expr_type, AliasMap, BoundsMap, FnSig as CheckFnSig, LocalBinding,
    StdTypeMap, TraitEnv, TypeDefs,
};
use anyhow::Result;
use clg_ast::{BinOp, Expr, Func, ParamKind, Span, Type};
use clg_ir::{
    BinOpIR, Function as IrFunction, GuardKind, Instr, IrType, TrapCode, Value, VariantKind,
    VariantParts,
};
use std::collections::{HashMap, HashSet};

mod array;
mod block;
mod calls;
mod collections;
mod eq;
mod intrinsics;
mod layout;
mod r#match;

use array::{
    emit_array_data_ptr, emit_array_len, emit_array_len_guard, ARRAY_HEADER_ALIGN,
    ARRAY_HEADER_DATA_OFFSET, ARRAY_HEADER_LEN_OFFSET, ARRAY_HEADER_SIZE,
};
use block::{expr_span_local, lower_block_expr};
use calls::lower_call_expr;
use eq::emit_eq_for_type;
use r#match::{lower_enum_match, lower_match_sugar};
use layout::{
    array_layout, collection_layout, std_type_info_for, struct_layout, tuple_layout,
};

type FnSig = CheckFnSig;

const COLLECTION_HEADER_SIZE: u32 = 16;
const COLLECTION_HEADER_ALIGN: u32 = 4;
const COLLECTION_LEN_OFFSET: u32 = 0;
const COLLECTION_CAP_OFFSET: u32 = 4;
const COLLECTION_FLAGS_OFFSET: u32 = 8;
const COLLECTION_DATA_OFFSET: u32 = 12;

fn ir_ty(t: Type) -> IrType {
    match t {
        Type::Int => IrType::Int,
        Type::U8 => IrType::Int,
        Type::U64 => IrType::U64,
        Type::U128 => IrType::U128,
        Type::U256 => IrType::U256,
        Type::Bool => IrType::Bool,
        Type::String => IrType::Int, // placeholder until strings have a runtime representation
        Type::Bytes => IrType::Int,
        Type::Named { .. } => IrType::Int,
        Type::Option(_) => IrType::Int,
        Type::Result(_, _) => IrType::Int,
        Type::List(_)
        | Type::Set(_)
        | Type::Map(_, _)
        | Type::Array(_, _)
        | Type::Slice(_)
        | Type::Tuple(_) => {
            IrType::Int
        }
    }
}

fn mem_ir_type(ty: &Type, aliases: &AliasMap) -> Result<IrType> {
    let resolved = base_type(ty, aliases)?;
    let ir = match resolved {
        Type::U8 => IrType::U8,
        Type::U64 => IrType::U64,
        Type::Bool => IrType::Bool,
        _ => IrType::Int,
    };
    Ok(ir)
}

fn emit_ptr_add_const(ctx: &mut LowerCtx<'_>, ptr: Value, offset: u32) -> Value {
    if offset == 0 {
        return ptr;
    }
    let off = emit_int_const(ctx, offset as i64);
    emit_ptr_add(ctx, ptr, off)
}

fn load_value_borrow(
    ctx: &mut LowerCtx<'_>,
    ty: &Type,
    ptr: Value,
    offset: u32,
) -> Result<Value> {
    if std_type_info_for(ty, ctx.aliases, ctx.std_types)?.is_some() {
        return Ok(emit_ptr_add_const(ctx, ptr, offset));
    }
    let mem_ty = mem_ir_type(ty, ctx.aliases)?;
    let dst = fresh(ctx);
    ctx.body.push(Instr::Load {
        dst,
        ptr,
        offset,
        ty: mem_ty,
    });
    Ok(dst)
}

fn load_value_copy(
    ctx: &mut LowerCtx<'_>,
    ty: &Type,
    ptr: Value,
    offset: u32,
) -> Result<Value> {
    if let Some(info) = std_type_info_for(ty, ctx.aliases, ctx.std_types)? {
        let src_ptr = emit_ptr_add_const(ctx, ptr, offset);
        let dst = emit_alloc(ctx, info.byte_len, info.align);
        let len_val = emit_int_const(ctx, info.byte_len as i64);
        emit_memcpy_bytes(ctx, src_ptr, dst, len_val)?;
        return Ok(dst);
    }
    load_value_borrow(ctx, ty, ptr, offset)
}

fn store_value(
    ctx: &mut LowerCtx<'_>,
    ty: &Type,
    ptr: Value,
    offset: u32,
    value: Value,
) -> Result<()> {
    if let Some(info) = std_type_info_for(ty, ctx.aliases, ctx.std_types)? {
        let dst_ptr = emit_ptr_add_const(ctx, ptr, offset);
        let len_val = emit_int_const(ctx, info.byte_len as i64);
        emit_memcpy_bytes(ctx, value, dst_ptr, len_val)?;
        return Ok(());
    }
    let mem_ty = mem_ir_type(ty, ctx.aliases)?;
    ctx.body.push(Instr::Store {
        ptr,
        src: value,
        offset,
        ty: mem_ty,
    });
    Ok(())
}

fn zero_value_for_type(ctx: &mut LowerCtx<'_>, ty: &Type) -> Result<Value> {
    if std_type_info_for(ty, ctx.aliases, ctx.std_types)?.is_some() {
        return Ok(emit_int_const(ctx, 0));
    }
    let mem_ty = mem_ir_type(ty, ctx.aliases)?;
    Ok(emit_zero_for_mem_ty(ctx, mem_ty))
}

pub(crate) struct LowerCtx<'a> {
    pub next: u32,
    pub env: HashMap<&'a str, Value>,
    pub type_env: HashMap<&'a str, LocalBinding>,
    pub fns: HashMap<&'a str, FnSig>,      // for call return types
    pub fn_indices: HashMap<&'a str, u32>, // for resolving callee indices (user + intrinsics)
    pub aliases: &'a AliasMap,
    pub trait_env: &'a TraitEnv<'a>,
    pub type_defs: &'a TypeDefs<'a>,
    pub std_types: &'a StdTypeMap,
    pub type_params: HashSet<String>,
    pub bounds: BoundsMap,
    pub body: Vec<Instr>,
    pub ret_ty: Type,
}

pub(crate) fn lower_func<'a>(
    f: &'a Func,
    fns: &HashMap<&'a str, FnSig>,
    fn_indices: &HashMap<&'a str, u32>,
    aliases: &'a AliasMap,
    trait_env: &'a TraitEnv<'a>,
    type_defs: &'a TypeDefs<'a>,
    std_types: &'a StdTypeMap,
) -> Result<IrFunction> {
    let mut env: HashMap<&str, Value> = HashMap::new();
    let mut type_env: HashMap<&str, LocalBinding> = HashMap::new();
    type_env.insert(
        "$return",
        LocalBinding {
            ty: f.ret.clone(),
            kind: ParamKind::Borrow,
        },
    );
    for (i, p) in f.params.iter().enumerate() {
        env.insert(p.name.as_str(), Value(i as u32));
        type_env.insert(
            p.name.as_str(),
            LocalBinding {
                ty: p.ty.clone(),
                kind: p.kind,
            },
        );
    }
    let mut ctx = LowerCtx {
        next: f.params.len() as u32,
        env,
        type_env,
        fns: fns.clone(),
        fn_indices: fn_indices.clone(),
        aliases,
        trait_env,
        type_defs,
        std_types,
        type_params: f.type_params.iter().map(|p| p.name.clone()).collect(),
        bounds: {
            let mut map: BoundsMap = HashMap::new();
            for bound in &f.where_bounds {
                map.entry(bound.param.clone())
                    .or_default()
                    .insert(bound.trait_name.clone());
            }
            map
        },
        body: Vec::new(),
        ret_ty: f.ret.clone(),
    };

    for (i, p) in f.params.iter().enumerate() {
        if let Type::Array(_, Some(len)) = base_type(&p.ty, aliases)? {
            emit_array_len_guard(&mut ctx, Value(i as u32), len);
        }
    }

    for req in &f.requires {
        let cond = lower_expr(&mut ctx, &req.expr, None)?;
        ctx.body.push(Instr::Guard {
            cond,
            trap: TrapCode::ContractViolation,
            span: Some((req.span.start as u32, req.span.end as u32)),
            detail: GuardKind::Require,
        });
    }

    let ret_val = lower_expr(&mut ctx, &f.body, Some(f.ret.clone()))?;

    if let Type::Array(_, Some(len)) = base_type(&f.ret, aliases)? {
        emit_array_len_guard(&mut ctx, ret_val, len);
    }

    if !f.ensures.is_empty() {
        ctx.env.insert("result", ret_val);
        for ens in &f.ensures {
            let cond = lower_expr(&mut ctx, &ens.expr, None)?;
            ctx.body.push(Instr::Guard {
                cond,
                trap: TrapCode::ContractViolation,
                span: Some((ens.span.start as u32, ens.span.end as u32)),
                detail: GuardKind::Ensure,
            });
        }
        ctx.env.remove("result");
    }

    ctx.body.push(Instr::Ret { val: ret_val });

    Ok(IrFunction {
        name: f.name.clone(),
        params: f.params.iter().map(|p| ir_ty(p.ty.clone())).collect(),
        ret: Some(ir_ty(f.ret.clone())),
        body: ctx.body,
    })
}

fn lower_expr<'a>(ctx: &mut LowerCtx<'a>, e: &'a Expr, expected: Option<Type>) -> Result<Value> {
    match e {
        Expr::Int(n, _) => match expected {
            Some(Type::U8) => {
                let dst = fresh(ctx);
                ctx.body.push(Instr::IConst {
                    dst,
                    ty: IrType::U8,
                    n: *n,
                });
                Ok(dst)
            }
            Some(Type::U64) => {
                let dst = fresh(ctx);
                ctx.body.push(Instr::IConst {
                    dst,
                    ty: IrType::U64,
                    n: *n,
                });
                Ok(dst)
            }
            Some(Type::U128) => {
                if *n < 0 {
                    anyhow::bail!("U128 literal must be non-negative");
                }
                let limb_lo = emit_u64_const(ctx, *n as u64);
                let limb_hi = emit_u64_const(ctx, 0);
                let dst = fresh(ctx);
                ctx.body.push(Instr::U128Init {
                    dst,
                    limb_lo,
                    limb_hi,
                });
                Ok(dst)
            }
            Some(Type::U256) => {
                if *n < 0 {
                    anyhow::bail!("U256 literal must be non-negative");
                }
                let limb0 = emit_u64_const(ctx, *n as u64);
                let limb1 = emit_u64_const(ctx, 0);
                let limb2 = emit_u64_const(ctx, 0);
                let limb3 = emit_u64_const(ctx, 0);
                let dst = fresh(ctx);
                ctx.body.push(Instr::U256Init {
                    dst,
                    limb0,
                    limb1,
                    limb2,
                    limb3,
                });
                Ok(dst)
            }
            _ => {
                let dst = fresh(ctx);
                ctx.body.push(Instr::IConst {
                    dst,
                    ty: IrType::Int,
                    n: *n,
                });
                Ok(dst)
            }
        },
        Expr::Block { block } => lower_block_expr(ctx, block, expected),
        Expr::Return { expr, .. } => {
            // For expression-bodied functions, `return e` is equivalent to `e`.
            // Lower inner expression; the enclosing function appends the Ret.
            lower_expr(ctx, expr, expected)
        }
        Expr::Bool(b, _) => {
            let dst = fresh(ctx);
            ctx.body.push(Instr::IConst {
                dst,
                ty: IrType::Bool,
                n: if *b { 1 } else { 0 },
            });
            Ok(dst)
        }
        Expr::String(s, _) => {
            let dst = fresh(ctx);
            ctx.body.push(Instr::IStringConst { dst, s: s.clone() });
            Ok(dst)
        }
        Expr::ArrayLit { elems, .. } => {
            let first = elems
                .first()
                .ok_or_else(|| anyhow::anyhow!("array literal requires at least one element"))?;
            let elem_ty = infer_expr_type(first, &ctx.type_env, &ctx.fns, ctx.trait_env, ctx.aliases, ctx.type_defs, &ctx.type_params, &ctx.bounds)?;
            let elem_ty = base_type(&elem_ty, ctx.aliases)?;
            let layout = array_layout(&elem_ty, elems.len() as u32, ctx.aliases, ctx.std_types)?;
            let data_ptr = emit_alloc(ctx, layout.size, layout.align);
            let header_ptr = emit_alloc(ctx, ARRAY_HEADER_SIZE, ARRAY_HEADER_ALIGN);
            let len_val = emit_int_const(ctx, elems.len() as i64);
            ctx.body.push(Instr::Store {
                ptr: header_ptr,
                src: len_val,
                offset: ARRAY_HEADER_LEN_OFFSET,
                ty: IrType::Int,
            });
            ctx.body.push(Instr::Store {
                ptr: header_ptr,
                src: data_ptr,
                offset: ARRAY_HEADER_DATA_OFFSET,
                ty: IrType::Int,
            });
            for (idx, elem) in elems.iter().enumerate() {
                let offset = (idx as u64)
                    .checked_mul(layout.stride as u64)
                    .ok_or_else(|| anyhow::anyhow!("array literal offset overflow"))?;
                if offset > u32::MAX as u64 {
                    anyhow::bail!("array literal offset exceeds u32 limits");
                }
                let val = lower_expr(ctx, elem, Some(elem_ty.clone()))?;
                store_value(ctx, &elem_ty, data_ptr, offset as u32, val)?;
            }
            Ok(header_ptr)
        }
        Expr::TupleLit { elems, .. } => {
            let mut elem_tys = Vec::with_capacity(elems.len());
            for elem in elems {
                let ty = infer_expr_type(elem, &ctx.type_env, &ctx.fns, ctx.trait_env, ctx.aliases, ctx.type_defs, &ctx.type_params, &ctx.bounds)?;
                elem_tys.push(base_type(&ty, ctx.aliases)?);
            }
            let layout = tuple_layout(&elem_tys, ctx.aliases, ctx.std_types)?;
            let ptr = emit_alloc(ctx, layout.size, layout.align);
            for (idx, elem) in elems.iter().enumerate() {
                let elem_ty = elem_tys
                    .get(idx)
                    .cloned()
                    .ok_or_else(|| anyhow::anyhow!("tuple element missing"))?;
                let val = lower_expr(ctx, elem, Some(elem_ty.clone()))?;
                let offset = *layout
                    .offsets
                    .get(idx)
                    .ok_or_else(|| anyhow::anyhow!("tuple offset missing"))?;
                store_value(ctx, &elem_ty, ptr, offset, val)?;
            }
            Ok(ptr)
        }
        Expr::StructLit { name: _, fields, .. } => {
            let struct_ty = infer_expr_type(
                e,
                &ctx.type_env,
                &ctx.fns,
                ctx.trait_env,
                ctx.aliases,
                ctx.type_defs,
                &ctx.type_params,
                &ctx.bounds,
            )?;
            let resolved = base_type(&struct_ty, ctx.aliases)?;
            let Type::Named { name: type_name, args } = resolved else {
                anyhow::bail!("struct literal expects a struct value");
            };
            let (decl_fields, layout) = struct_layout(
                ctx.type_defs,
                ctx.aliases,
                ctx.std_types,
                type_name.as_str(),
                &args,
            )?;
            let mut field_offsets: HashMap<&str, (u32, Type)> =
                HashMap::with_capacity(decl_fields.len());
            for (idx, field) in decl_fields.iter().enumerate() {
                let offset = *layout
                    .offsets
                    .get(idx)
                    .ok_or_else(|| anyhow::anyhow!("struct field offset missing"))?;
                field_offsets.insert(field.name.as_str(), (offset, field.ty.clone()));
            }
            let ptr = emit_alloc(ctx, layout.size, layout.align);
            for field in fields {
                let (offset, field_ty) = field_offsets
                    .get(field.name.as_str())
                    .ok_or_else(|| anyhow::anyhow!("unknown struct field `{}`", field.name))?
                    .clone();
                let val = lower_expr(ctx, &field.expr, Some(field_ty.clone()))?;
                store_value(ctx, &field_ty, ptr, offset, val)?;
            }
            Ok(ptr)
        }
        Expr::FieldAccess { base, field, .. } => {
            let base_ty = infer_expr_type(base, &ctx.type_env, &ctx.fns, ctx.trait_env, ctx.aliases, ctx.type_defs, &ctx.type_params, &ctx.bounds)?;
            let resolved = base_type(&base_ty, ctx.aliases)?;
            let Type::Named { name, args } = resolved else {
                anyhow::bail!("field access expects a struct value");
            };
            let (decl_fields, layout) =
                struct_layout(ctx.type_defs, ctx.aliases, ctx.std_types, name.as_str(), &args)?;
            let mut field_idx: Option<usize> = None;
            let mut field_ty: Option<Type> = None;
            for (idx, f) in decl_fields.iter().enumerate() {
                if f.name == *field {
                    field_idx = Some(idx);
                    field_ty = Some(f.ty.clone());
                    break;
                }
            }
            let idx = field_idx.ok_or_else(|| anyhow::anyhow!("unknown field `{}`", field))?;
            let field_ty = field_ty.ok_or_else(|| anyhow::anyhow!("field type missing"))?;
            let offset = *layout
                .offsets
                .get(idx)
                .ok_or_else(|| anyhow::anyhow!("struct field offset missing"))?;
            let base_ptr = lower_expr(ctx, base, None)?;
            load_value_borrow(ctx, &field_ty, base_ptr, offset)
        }
        Expr::Unary { .. } => {
            anyhow::bail!("unary operators are not supported in codegen yet")
        }
        Expr::Match {
            scrutinee, arms, ..
        } => {
            if let Some(val) = lower_match_sugar(ctx, scrutinee, arms, expected.clone())? {
                return Ok(val);
            }
            let scrut_ty = infer_expr_type(scrutinee, &ctx.type_env, &ctx.fns, ctx.trait_env, ctx.aliases, ctx.type_defs, &ctx.type_params, &ctx.bounds)?;
            let resolved = base_type(&scrut_ty, ctx.aliases)?;
            if let Type::Named { name, args } = resolved {
                if ctx.type_defs.enums.contains_key(name.as_str()) {
                    return lower_enum_match(ctx, scrutinee, arms, expected, name.as_str(), &args);
                }
            }
            anyhow::bail!("match expression not supported in lowering yet")
        }
        Expr::Try { expr, .. } => {
            let kind = match &ctx.ret_ty {
                Type::Option(_) => VariantKind::Option,
                Type::Result(_, _) => VariantKind::Result,
                other => {
                    return Err(anyhow::anyhow!(
                        "`?` requires Option/Result return type, found {:?}",
                        other
                    ));
                }
            };
            let variant = lower_expr(ctx, expr, None)?;
            let parts = ctx.variant_destructure(variant, kind);
            let failure_tag = emit_int_const(ctx, 0);
            let cond = fresh(ctx);
            ctx.body.push(Instr::IBin {
                dst: cond,
                op: BinOpIR::Eq,
                lhs: parts.tag,
                rhs: failure_tag,
                ty: IrType::Int,
            });
            ctx.body.push(Instr::ReturnIf { cond, ret: variant });
            Ok(parts.payload_lo)
        }
        Expr::If {
            cond,
            then_br,
            else_br,
            ..
        } => {
            let cv = lower_expr(ctx, cond, None)?;
            let tv = lower_expr(ctx, then_br, expected.clone())?;
            let ev = lower_expr(ctx, else_br, expected)?;
            let dst = fresh(ctx);
            ctx.body.push(Instr::ISelect {
                dst,
                cond: cv,
                then_v: tv,
                else_v: ev,
            });
            Ok(dst)
        }
        Expr::Var(name, _) => ctx
            .env
            .get(name.as_str())
            .copied()
            .ok_or_else(|| anyhow::anyhow!(format!("unknown variable `{}`", name))),
        Expr::Index { base, index, span } => {
            let base_ty = infer_expr_type(base, &ctx.type_env, &ctx.fns, ctx.trait_env, ctx.aliases, ctx.type_defs, &ctx.type_params, &ctx.bounds)?;
            let resolved = base_type(&base_ty, ctx.aliases)?;
            let base_ptr = lower_expr(ctx, base, None)?;
            match resolved {
                Type::Array(inner, _) | Type::Slice(inner) => {
                    let elem_ty = *inner;
                    let (_, _, stride) = collection_layout(&elem_ty, ctx.aliases, ctx.std_types)?;
                    let len_val = emit_array_len(ctx, base_ptr);
                    let data_ptr = emit_array_data_ptr(ctx, base_ptr);
                    let idx_val = match index.as_ref() {
                        Expr::Int(n, _) => emit_int_const(ctx, *n),
                        _ => lower_expr(ctx, index, None)?,
                    };
                    let zero = emit_int_const(ctx, 0);
                    let ge_zero = fresh(ctx);
                    ctx.body.push(Instr::IBin {
                        dst: ge_zero,
                        op: BinOpIR::Ge,
                        lhs: idx_val,
                        rhs: zero,
                        ty: IrType::Int,
                    });
                    let lt_len = fresh(ctx);
                    ctx.body.push(Instr::IBin {
                        dst: lt_len,
                        op: BinOpIR::Lt,
                        lhs: idx_val,
                        rhs: len_val,
                        ty: IrType::Int,
                    });
                    let ok = fresh(ctx);
                    ctx.body.push(Instr::IBin {
                        dst: ok,
                        op: BinOpIR::And,
                        lhs: ge_zero,
                        rhs: lt_len,
                        ty: IrType::Bool,
                    });
                    ctx.body.push(Instr::Guard {
                        cond: ok,
                        trap: TrapCode::ContractViolation,
                        span: Some((span.start as u32, span.end as u32)),
                        detail: GuardKind::Require,
                    });
                    let stride_val = emit_int_const(ctx, stride as i64);
                    let offset_val = fresh(ctx);
                    ctx.body.push(Instr::IBin {
                        dst: offset_val,
                        op: BinOpIR::Mul,
                        lhs: idx_val,
                        rhs: stride_val,
                        ty: IrType::Int,
                    });
                    let addr = fresh(ctx);
                    ctx.body.push(Instr::IBin {
                        dst: addr,
                        op: BinOpIR::Add,
                        lhs: data_ptr,
                        rhs: offset_val,
                        ty: IrType::Int,
                    });
                    load_value_borrow(ctx, &elem_ty, addr, 0)
                }
                Type::Tuple(elems) => {
                    let idx = match index.as_ref() {
                        Expr::Int(n, _) => *n,
                        _ => anyhow::bail!("tuple index must be a constant integer"),
                    };
                    if idx < 0 || idx as usize >= elems.len() {
                        anyhow::bail!("tuple index out of bounds in lowering");
                    }
                    let layout = tuple_layout(&elems, ctx.aliases, ctx.std_types)?;
                    let elem_ty = elems[idx as usize].clone();
                    let offset = *layout
                        .offsets
                        .get(idx as usize)
                        .ok_or_else(|| anyhow::anyhow!("tuple offset missing"))?;
                    load_value_borrow(ctx, &elem_ty, base_ptr, offset)
                }
                other => anyhow::bail!("indexing not supported for {:?}", other),
            }
        }
        Expr::Bin { op, lhs, rhs, .. } => {
            let lt = infer_expr_type(lhs, &ctx.type_env, &ctx.fns, ctx.trait_env, ctx.aliases, ctx.type_defs, &ctx.type_params, &ctx.bounds)?;
            let rt = infer_expr_type(rhs, &ctx.type_env, &ctx.fns, ctx.trait_env, ctx.aliases, ctx.type_defs, &ctx.type_params, &ctx.bounds)?;
            if matches!(op, BinOp::Eq | BinOp::Neq) {
                let std_l = std_type_info_for(&lt, ctx.aliases, ctx.std_types)?;
                let std_r = std_type_info_for(&rt, ctx.aliases, ctx.std_types)?;
                if std_l.is_some() || std_r.is_some() {
                    let lv = lower_expr(ctx, lhs, None)?;
                    let rv = lower_expr(ctx, rhs, None)?;
                    let eq_ty = if std_l.is_some() { lt.clone() } else { rt.clone() };
                    let eq_val = emit_eq_for_type(ctx, &eq_ty, lv, rv, ctx.aliases)?;
                    if matches!(op, BinOp::Neq) {
                        let zero = emit_bool_const(ctx, false);
                        let dst = fresh(ctx);
                        ctx.body.push(Instr::IBin {
                            dst,
                            op: BinOpIR::Eq,
                            lhs: eq_val,
                            rhs: zero,
                            ty: IrType::Bool,
                        });
                        return Ok(dst);
                    }
                    return Ok(eq_val);
                }
            }
            let op_type = match op {
                BinOp::And | BinOp::Or => Type::Bool,
                BinOp::Eq | BinOp::Neq => {
                    if matches!(expected, Some(Type::U64))
                        || matches!(lt, Type::U64)
                        || matches!(rt, Type::U64)
                    {
                        Type::U64
                    } else {
                        Type::Int
                    }
                }
                _ => {
                    if matches!(expected, Some(Type::U64))
                        || matches!(lt, Type::U64)
                        || matches!(rt, Type::U64)
                    {
                        Type::U64
                    } else {
                        Type::Int
                    }
                }
            };
            let operand_expected = if matches!(op_type, Type::U64) {
                Some(Type::U64)
            } else {
                None
            };
            let lv = lower_expr(ctx, lhs, operand_expected.clone())?;
            let rv = lower_expr(ctx, rhs, operand_expected)?;
            let dst = fresh(ctx);
            let irop = match op {
                BinOp::Add => BinOpIR::Add,
                BinOp::Sub => BinOpIR::Sub,
                BinOp::Mul => BinOpIR::Mul,
                BinOp::Div => BinOpIR::Div,
                BinOp::Shl => BinOpIR::Shl,
                BinOp::Shr => BinOpIR::Shr,
                BinOp::BitAnd | BinOp::And => BinOpIR::And,
                BinOp::BitOr | BinOp::Or => BinOpIR::Or,
                BinOp::BitXor => BinOpIR::Xor,
                BinOp::Lt => BinOpIR::Lt,
                BinOp::Le => BinOpIR::Le,
                BinOp::Gt => BinOpIR::Gt,
                BinOp::Ge => BinOpIR::Ge,
                BinOp::Eq => BinOpIR::Eq,
                BinOp::Neq => BinOpIR::Neq,
            };
            let ir_op_ty = match op_type {
                Type::U64 => IrType::U64,
                Type::Bool => IrType::Bool,
                _ => IrType::Int,
            };
            ctx.body.push(Instr::IBin {
                dst,
                op: irop,
                lhs: lv,
                rhs: rv,
                ty: ir_op_ty,
            });
            if matches!(op_type, Type::U64) && matches!(op, BinOp::Add | BinOp::Sub | BinOp::Mul) {
                let span = expr_span_local(e);
                emit_u64_overflow_guard(ctx, op, lv, rv, dst, span)?;
            }
            Ok(dst)
        }
        Expr::Call { callee, args, .. } => {
            lower_call_expr(ctx, e, callee.as_str(), args, expected.as_ref())
        }
    }
}

impl<'a> LowerCtx<'a> {
    fn variant_init(&mut self, tag: Value, payload_lo: Value, payload_hi: Value) -> Value {
        let dst = fresh(self);
        self.body.push(Instr::VariantInit {
            dst,
            tag,
            payload_lo,
            payload_hi,
        });
        dst
    }

    #[allow(dead_code)]
    fn variant_destructure(&mut self, variant: Value, kind: VariantKind) -> VariantParts {
        let tag = fresh(self);
        self.body.push(Instr::VariantLoadTag {
            dst: tag,
            variant,
            kind,
        });
        let payload_lo = fresh(self);
        self.body.push(Instr::VariantLoadPayloadLo {
            dst: payload_lo,
            variant,
        });
        let payload_hi = fresh(self);
        self.body.push(Instr::VariantLoadPayloadHi {
            dst: payload_hi,
            variant,
        });
        VariantParts {
            tag,
            payload_lo,
            payload_hi,
        }
    }
}

fn emit_int_const(ctx: &mut LowerCtx<'_>, n: i64) -> Value {
    let dst = fresh(ctx);
    ctx.body.push(Instr::IConst {
        dst,
        ty: IrType::Int,
        n,
    });
    dst
}

fn emit_alloc(ctx: &mut LowerCtx<'_>, size: u32, align: u32) -> Value {
    let dst = fresh(ctx);
    let align = align.max(1);
    ctx.body.push(Instr::Alloc { dst, size, align });
    dst
}

fn emit_alloc_dyn(ctx: &mut LowerCtx<'_>, size: Value, align: u32) -> Value {
    let dst = fresh(ctx);
    let align = align.max(1);
    ctx.body.push(Instr::AllocDyn { dst, size, align });
    dst
}

fn emit_ptr_add(ctx: &mut LowerCtx<'_>, ptr: Value, offset: Value) -> Value {
    let dst = fresh(ctx);
    ctx.body.push(Instr::IBin {
        dst,
        op: BinOpIR::Add,
        lhs: ptr,
        rhs: offset,
        ty: IrType::Int,
    });
    dst
}

fn emit_load_i32(ctx: &mut LowerCtx<'_>, ptr: Value, offset: u32) -> Value {
    let dst = fresh(ctx);
    ctx.body.push(Instr::Load {
        dst,
        ptr,
        offset,
        ty: IrType::Int,
    });
    dst
}

fn emit_store_i32(ctx: &mut LowerCtx<'_>, ptr: Value, offset: u32, src: Value) {
    ctx.body.push(Instr::Store {
        ptr,
        src,
        offset,
        ty: IrType::Int,
    });
}

fn emit_memcpy_bytes(
    ctx: &mut LowerCtx<'_>,
    src_ptr: Value,
    dst_ptr: Value,
    byte_len: Value,
) -> Result<()> {
    let idx = fresh(ctx);
    ctx.body.push(Instr::IConst {
        dst: idx,
        ty: IrType::Int,
        n: 0,
    });
    let one = emit_int_const(ctx, 1);

    ctx.body.push(Instr::BlockBegin);
    ctx.body.push(Instr::LoopBegin);
    let cond = fresh(ctx);
    ctx.body.push(Instr::IBin {
        dst: cond,
        op: BinOpIR::Lt,
        lhs: idx,
        rhs: byte_len,
        ty: IrType::Int,
    });
    ctx.body.push(Instr::BrIfEqz { cond, depth: 1 });

    let src_addr = emit_ptr_add(ctx, src_ptr, idx);
    let dst_addr = emit_ptr_add(ctx, dst_ptr, idx);
    let byte = fresh(ctx);
    ctx.body.push(Instr::Load {
        dst: byte,
        ptr: src_addr,
        offset: 0,
        ty: IrType::U8,
    });
    ctx.body.push(Instr::Store {
        ptr: dst_addr,
        src: byte,
        offset: 0,
        ty: IrType::U8,
    });

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
    Ok(())
}

fn emit_zero_for_mem_ty(ctx: &mut LowerCtx<'_>, mem_ty: IrType) -> Value {
    match mem_ty {
        IrType::U64 => emit_u64_const(ctx, 0),
        IrType::U8 => {
            let dst = fresh(ctx);
            ctx.body.push(Instr::IConst {
                dst,
                ty: IrType::U8,
                n: 0,
            });
            dst
        }
        IrType::Bool => emit_bool_const(ctx, false),
        _ => emit_int_const(ctx, 0),
    }
}

fn emit_bool_const(ctx: &mut LowerCtx<'_>, value: bool) -> Value {
    let dst = fresh(ctx);
    ctx.body.push(Instr::IConst {
        dst,
        ty: IrType::Bool,
        n: if value { 1 } else { 0 },
    });
    dst
}

fn emit_u64_const(ctx: &mut LowerCtx<'_>, n: u64) -> Value {
    let dst = fresh(ctx);
    ctx.body.push(Instr::IConst {
        dst,
        ty: IrType::U64,
        n: n as i64,
    });
    dst
}

fn emit_u64_bin(ctx: &mut LowerCtx<'_>, op: BinOp, lhs: Value, rhs: Value) -> Value {
    let dst = fresh(ctx);
    let ir_op = match op {
        BinOp::Add => BinOpIR::Add,
        BinOp::Sub => BinOpIR::Sub,
        BinOp::Mul => BinOpIR::Mul,
        BinOp::Div => BinOpIR::Div,
        _ => BinOpIR::Add,
    };
    ctx.body.push(Instr::IBin {
        dst,
        op: ir_op,
        lhs,
        rhs,
        ty: IrType::U64,
    });
    dst
}

fn mem_layout_for_ir(ty: IrType) -> (u32, u32) {
    match ty {
        IrType::U8 => (1, 1),
        IrType::U64 => (8, 8),
        _ => (4, 4),
    }
}

fn emit_u64_overflow_flag(
    ctx: &mut LowerCtx<'_>,
    op: &BinOp,
    lhs: Value,
    rhs: Value,
    dst: Value,
) -> Result<Option<Value>> {
    let overflow = match op {
        BinOp::Add => {
            let overflow = fresh(ctx);
            ctx.body.push(Instr::IBin {
                dst: overflow,
                op: BinOpIR::Lt,
                lhs: dst,
                rhs: lhs,
                ty: IrType::U64,
            });
            overflow
        }
        BinOp::Sub => {
            let overflow = fresh(ctx);
            ctx.body.push(Instr::IBin {
                dst: overflow,
                op: BinOpIR::Lt,
                lhs,
                rhs,
                ty: IrType::U64,
            });
            overflow
        }
        BinOp::Mul => {
            let zero = emit_u64_const(ctx, 0);
            let rhs_is_zero = fresh(ctx);
            ctx.body.push(Instr::IBin {
                dst: rhs_is_zero,
                op: BinOpIR::Eq,
                lhs: rhs,
                rhs: zero,
                ty: IrType::U64,
            });
            let one = emit_u64_const(ctx, 1);
            let rhs_nonzero = fresh(ctx);
            ctx.body.push(Instr::ISelect {
                dst: rhs_nonzero,
                cond: rhs_is_zero,
                then_v: one,
                else_v: rhs,
            });
            let div = fresh(ctx);
            ctx.body.push(Instr::IBin {
                dst: div,
                op: BinOpIR::Div,
                lhs: dst,
                rhs: rhs_nonzero,
                ty: IrType::U64,
            });
            let div_eq = fresh(ctx);
            ctx.body.push(Instr::IBin {
                dst: div_eq,
                op: BinOpIR::Eq,
                lhs: div,
                rhs: lhs,
                ty: IrType::U64,
            });
            let ok = fresh(ctx);
            ctx.body.push(Instr::IBin {
                dst: ok,
                op: BinOpIR::Or,
                lhs: rhs_is_zero,
                rhs: div_eq,
                ty: IrType::Bool,
            });
            let zero = emit_bool_const(ctx, false);
            let overflow = fresh(ctx);
            ctx.body.push(Instr::IBin {
                dst: overflow,
                op: BinOpIR::Eq,
                lhs: ok,
                rhs: zero,
                ty: IrType::Bool,
            });
            overflow
        }
        _ => return Ok(None),
    };
    Ok(Some(overflow))
}

fn emit_u64_overflow_guard(
    ctx: &mut LowerCtx<'_>,
    op: &BinOp,
    lhs: Value,
    rhs: Value,
    dst: Value,
    span: Span,
) -> Result<()> {
    let Some(overflow) = emit_u64_overflow_flag(ctx, op, lhs, rhs, dst)? else {
        return Ok(());
    };
    let zero = emit_bool_const(ctx, false);
    let ok = fresh(ctx);
    ctx.body.push(Instr::IBin {
        dst: ok,
        op: BinOpIR::Eq,
        lhs: overflow,
        rhs: zero,
        ty: IrType::Bool,
    });

    ctx.body.push(Instr::Guard {
        cond: ok,
        trap: TrapCode::Overflow,
        span: Some((span.start as u32, span.end as u32)),
        detail: GuardKind::Require,
    });
    Ok(())
}

fn fresh(ctx: &mut LowerCtx<'_>) -> Value {
    let v = Value(ctx.next);
    ctx.next += 1;
    v
}
