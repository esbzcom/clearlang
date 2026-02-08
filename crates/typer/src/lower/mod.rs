use crate::check::{
    base_type, AliasMap, BoundsMap, FnSig as CheckFnSig, LocalBinding, StdTypeMap, TraitEnv,
    TypeDefs,
};
use anyhow::Result;
use clg_ast::{Expr, Func, ParamKind, Type};
use clg_ir::{
    Function as IrFunction, GuardKind, Instr, IrType, TrapCode, Value, VariantKind,
    VariantParts,
};
use std::collections::{HashMap, HashSet};

mod array;
mod atoms;
mod block;
mod binops;
mod calls;
mod collections;
mod collections_helpers;
mod collection_types;
mod collections_list;
mod collections_map;
mod collections_slice;
mod collections_set;
mod control;
mod emit;
mod eq;
mod index;
mod intrinsics;
mod layout;
mod literals;
mod r#match;
mod structs;
mod u64_ops;

use array::emit_array_len_guard;
use atoms::{lower_return_expr, lower_unary_expr, lower_var_expr};
use block::lower_block_expr;
use binops::lower_bin_expr;
use calls::lower_call_expr;
use control::{lower_if_expr, lower_try_expr};
use index::lower_index_expr;
use literals::{
    lower_array_lit, lower_bool_lit, lower_int_lit, lower_string_lit, lower_tuple_lit,
};
use r#match::lower_match_expr;
use structs::{lower_field_access, lower_struct_lit};
pub(crate) use u64_ops::{
    emit_u64_bin, emit_u64_const, emit_u64_overflow_flag, emit_u64_overflow_guard,
};
use layout::{
    std_type_info_for,
};
use emit::{
    emit_alloc, emit_int_const, emit_memcpy_bytes, emit_ptr_add, emit_zero_for_mem_ty, fresh,
};
pub(super) use emit::{
    emit_alloc_dyn, emit_bool_const, emit_load_i32, emit_store_i32, mem_layout_for_ir,
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
        Expr::Int(n, _) => lower_int_lit(ctx, *n, expected),
        Expr::Block { block } => lower_block_expr(ctx, block, expected),
        Expr::Return { expr, .. } => lower_return_expr(ctx, expr, expected),
        Expr::Bool(b, _) => lower_bool_lit(ctx, *b),
        Expr::String(s, _) => lower_string_lit(ctx, s),
        Expr::ArrayLit { elems, .. } => lower_array_lit(ctx, elems),
        Expr::TupleLit { elems, .. } => lower_tuple_lit(ctx, elems),
        Expr::StructLit { name: _, fields, .. } => lower_struct_lit(ctx, e, fields),
        Expr::FieldAccess { base, field, .. } => {
            lower_field_access(ctx, base, field.as_str())
        }
        Expr::Unary { .. } => lower_unary_expr(),
        Expr::Match {
            scrutinee, arms, ..
        } => lower_match_expr(ctx, scrutinee, arms, expected),
        Expr::Try { expr, .. } => lower_try_expr(ctx, expr),
        Expr::If {
            cond,
            then_br,
            else_br,
            ..
        } => lower_if_expr(ctx, cond, then_br, else_br, expected),
        Expr::Var(name, _) => lower_var_expr(ctx, name.as_str()),
        Expr::Index { base, index, span } => lower_index_expr(ctx, base, index, span),
        Expr::Bin { op, lhs, rhs, .. } => lower_bin_expr(ctx, e, op, lhs, rhs, expected),
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

