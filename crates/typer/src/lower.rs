use crate::check::{
    base_type, infer_expr_type, substitute_type, AliasMap, BoundsMap, FnSig as CheckFnSig,
    LocalBinding, TraitEnv, TypeDefs, TypeSubst,
};
use crate::guards::guard_kind_for_callee;
use anyhow::Result;
use clg_ast::{
    BinOp, Block, Expr, Func, MatchArm, MatchPat, ParamKind, Span, Stmt, StructField, Type,
    TypeParam,
};
use clg_ir::{
    BinOpIR, Function as IrFunction, GuardKind, Instr, IrType, TrapCode, Value, VariantKind,
    VariantParts,
};
use std::collections::{HashMap, HashSet};

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
        Type::List(_) | Type::Set(_) | Type::Map(_, _) | Type::Array(_, _) | Type::Tuple(_) => {
            IrType::Int
        }
    }
}

#[derive(Clone, Copy)]
struct ArrayLayout {
    stride: u32,
    size: u32,
    align: u32,
}

struct TupleLayout {
    offsets: Vec<u32>,
    size: u32,
    align: u32,
}

fn align_up(value: u32, align: u32) -> Result<u32> {
    if align <= 1 {
        return Ok(value);
    }
    let value = value as u64;
    let align = align as u64;
    let aligned = (value + (align - 1)) / align * align;
    if aligned > u32::MAX as u64 {
        anyhow::bail!("layout size exceeds u32 limits");
    }
    Ok(aligned as u32)
}

fn layout_for_type(ty: &Type, aliases: &AliasMap) -> Result<(u32, u32)> {
    let resolved = base_type(ty, aliases)?;
    match resolved {
        Type::Int | Type::Bool => Ok((4, 4)),
        Type::U8 => Ok((1, 1)),
        Type::U64 => Ok((8, 8)),
        Type::U128 | Type::U256 => Ok((4, 4)),
        Type::String | Type::Bytes => Ok((4, 4)),
        Type::Named { .. } => Ok((4, 4)),
        Type::Option(_) | Type::Result(_, _) => Ok((4, 4)),
        Type::List(_) | Type::Set(_) | Type::Map(_, _) => Ok((4, 4)),
        Type::Array(_, _) | Type::Tuple(_) => Ok((4, 4)),
    }
}

fn array_layout(inner: &Type, len: u32, aliases: &AliasMap) -> Result<ArrayLayout> {
    let (elem_size, elem_align) = layout_for_type(inner, aliases)?;
    let stride = align_up(elem_size, elem_align)?;
    let total = (stride as u64) * (len as u64);
    if total > u32::MAX as u64 {
        anyhow::bail!("array allocation size exceeds u32 limits");
    }
    Ok(ArrayLayout {
        stride,
        size: total as u32,
        align: elem_align,
    })
}

fn tuple_layout(elems: &[Type], aliases: &AliasMap) -> Result<TupleLayout> {
    let mut offsets = Vec::with_capacity(elems.len());
    let mut offset: u32 = 0;
    let mut max_align: u32 = 1;
    for elem in elems {
        let (size, align) = layout_for_type(elem, aliases)?;
        max_align = max_align.max(align);
        offset = align_up(offset, align)?;
        offsets.push(offset);
        offset = offset
            .checked_add(size)
            .ok_or_else(|| anyhow::anyhow!("tuple size overflow"))?;
    }
    let size = align_up(offset, max_align)?;
    Ok(TupleLayout {
        offsets,
        size,
        align: max_align,
    })
}

fn collection_layout(elem_ty: &Type, aliases: &AliasMap) -> Result<(u32, u32, u32)> {
    let (size, align) = layout_for_type(elem_ty, aliases)?;
    let stride = align_up(size, align)?;
    Ok((size, align, stride))
}

fn map_entry_layout(
    key_ty: &Type,
    val_ty: &Type,
    aliases: &AliasMap,
) -> Result<(u32, u32, u32, u32)> {
    let tuple = tuple_layout(&[key_ty.clone(), val_ty.clone()], aliases)?;
    let key_offset = *tuple
        .offsets
        .get(0)
        .ok_or_else(|| anyhow::anyhow!("map key offset missing"))?;
    let val_offset = *tuple
        .offsets
        .get(1)
        .ok_or_else(|| anyhow::anyhow!("map value offset missing"))?;
    Ok((tuple.size, tuple.align, key_offset, val_offset))
}

fn build_type_param_subst(params: &[TypeParam], args: &[Type]) -> Result<TypeSubst> {
    if params.len() != args.len() {
        anyhow::bail!(
            "type argument count mismatch: expected {}, found {}",
            params.len(),
            args.len()
        );
    }
    let mut subst: TypeSubst = HashMap::with_capacity(params.len());
    for (param, arg) in params.iter().zip(args.iter()) {
        subst.insert(param.name.clone(), arg.clone());
    }
    Ok(subst)
}

fn struct_layout<'a>(
    type_defs: &'a TypeDefs,
    aliases: &AliasMap,
    name: &str,
    args: &[Type],
) -> Result<(Vec<StructField>, TupleLayout)> {
    let info = type_defs
        .structs
        .get(name)
        .ok_or_else(|| anyhow::anyhow!("unknown struct `{}`", name))?;
    let subst = build_type_param_subst(&info.decl.type_params, args)?;
    let mut fields = Vec::with_capacity(info.decl.fields.len());
    let mut field_tys = Vec::with_capacity(info.decl.fields.len());
    for field in &info.decl.fields {
        let mut cloned = field.clone();
        cloned.ty = substitute_type(&cloned.ty, &subst);
        fields.push(cloned);
        field_tys.push(base_type(&fields.last().unwrap().ty, aliases)?);
    }
    let layout = tuple_layout(&field_tys, aliases)?;
    Ok((fields, layout))
}

fn enum_variant_info<'a>(
    type_defs: &'a TypeDefs,
    enum_name: &str,
    args: &[Type],
    variant_name: &str,
) -> Result<(usize, Vec<Type>, usize)> {
    let info = type_defs
        .enums
        .get(enum_name)
        .ok_or_else(|| anyhow::anyhow!("unknown enum `{}`", enum_name))?;
    let subst = build_type_param_subst(&info.decl.type_params, args)?;
    let mut idx = None;
    for (i, variant) in info.decl.variants.iter().enumerate() {
        if variant.name == variant_name {
            idx = Some(i);
            break;
        }
    }
    let index = idx.ok_or_else(|| {
        anyhow::anyhow!("unknown enum variant `{}::{}`", enum_name, variant_name)
    })?;
    let variant = &info.decl.variants[index];
    let mut fields = Vec::with_capacity(variant.fields.len());
    for ty in &variant.fields {
        fields.push(substitute_type(ty, &subst));
    }
    Ok((index, fields, info.decl.variants.len()))
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

pub(crate) struct LowerCtx<'a> {
    pub next: u32,
    pub env: HashMap<&'a str, Value>,
    pub type_env: HashMap<&'a str, LocalBinding>,
    pub fns: HashMap<&'a str, FnSig>,      // for call return types
    pub fn_indices: HashMap<&'a str, u32>, // for resolving callee indices (user + intrinsics)
    pub aliases: &'a AliasMap,
    pub trait_env: &'a TraitEnv<'a>,
    pub type_defs: &'a TypeDefs<'a>,
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
            let layout = array_layout(&elem_ty, elems.len() as u32, ctx.aliases)?;
            let ptr = emit_alloc(ctx, layout.size, layout.align);
            let mem_ty = mem_ir_type(&elem_ty, ctx.aliases)?;
            for (idx, elem) in elems.iter().enumerate() {
                let offset = (idx as u64)
                    .checked_mul(layout.stride as u64)
                    .ok_or_else(|| anyhow::anyhow!("array literal offset overflow"))?;
                if offset > u32::MAX as u64 {
                    anyhow::bail!("array literal offset exceeds u32 limits");
                }
                let val = lower_expr(ctx, elem, Some(elem_ty.clone()))?;
                ctx.body.push(Instr::Store {
                    ptr,
                    src: val,
                    offset: offset as u32,
                    ty: mem_ty,
                });
            }
            Ok(ptr)
        }
        Expr::TupleLit { elems, .. } => {
            let mut elem_tys = Vec::with_capacity(elems.len());
            for elem in elems {
                let ty = infer_expr_type(elem, &ctx.type_env, &ctx.fns, ctx.trait_env, ctx.aliases, ctx.type_defs, &ctx.type_params, &ctx.bounds)?;
                elem_tys.push(base_type(&ty, ctx.aliases)?);
            }
            let layout = tuple_layout(&elem_tys, ctx.aliases)?;
            let ptr = emit_alloc(ctx, layout.size, layout.align);
            for (idx, elem) in elems.iter().enumerate() {
                let elem_ty = elem_tys
                    .get(idx)
                    .cloned()
                    .ok_or_else(|| anyhow::anyhow!("tuple element missing"))?;
                let mem_ty = mem_ir_type(&elem_ty, ctx.aliases)?;
                let val = lower_expr(ctx, elem, Some(elem_ty))?;
                let offset = *layout
                    .offsets
                    .get(idx)
                    .ok_or_else(|| anyhow::anyhow!("tuple offset missing"))?;
                ctx.body.push(Instr::Store {
                    ptr,
                    src: val,
                    offset,
                    ty: mem_ty,
                });
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
            let (decl_fields, layout) =
                struct_layout(ctx.type_defs, ctx.aliases, type_name.as_str(), &args)?;
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
                let mem_ty = mem_ir_type(&field_ty, ctx.aliases)?;
                ctx.body.push(Instr::Store {
                    ptr,
                    src: val,
                    offset,
                    ty: mem_ty,
                });
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
                struct_layout(ctx.type_defs, ctx.aliases, name.as_str(), &args)?;
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
            let dst = fresh(ctx);
            let mem_ty = mem_ir_type(&field_ty, ctx.aliases)?;
            ctx.body.push(Instr::Load {
                dst,
                ptr: base_ptr,
                offset,
                ty: mem_ty,
            });
            Ok(dst)
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
                Type::Array(inner, len) => {
                    let elem_ty = *inner;
                    let layout = array_layout(&elem_ty, len, ctx.aliases)?;
                    let mem_ty = mem_ir_type(&elem_ty, ctx.aliases)?;
                    if let Expr::Int(idx, _) = index.as_ref() {
                        if *idx < 0 || (*idx as u64) >= len as u64 {
                            anyhow::bail!("array index out of bounds in lowering");
                        }
                        let offset = (*idx as u64)
                            .checked_mul(layout.stride as u64)
                            .ok_or_else(|| anyhow::anyhow!("array index offset overflow"))?;
                        if offset > u32::MAX as u64 {
                            anyhow::bail!("array index offset exceeds u32 limits");
                        }
                        let dst = fresh(ctx);
                        ctx.body.push(Instr::Load {
                            dst,
                            ptr: base_ptr,
                            offset: offset as u32,
                            ty: mem_ty,
                        });
                        Ok(dst)
                    } else {
                        let idx_val = lower_expr(ctx, index, None)?;
                        let zero = emit_int_const(ctx, 0);
                        let ge_zero = fresh(ctx);
                        ctx.body.push(Instr::IBin {
                            dst: ge_zero,
                            op: BinOpIR::Ge,
                            lhs: idx_val,
                            rhs: zero,
                            ty: IrType::Int,
                        });
                        let len_val = emit_int_const(ctx, len as i64);
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
                        let stride_val = emit_int_const(ctx, layout.stride as i64);
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
                            lhs: base_ptr,
                            rhs: offset_val,
                            ty: IrType::Int,
                        });
                        let dst = fresh(ctx);
                        ctx.body.push(Instr::Load {
                            dst,
                            ptr: addr,
                            offset: 0,
                            ty: mem_ty,
                        });
                        Ok(dst)
                    }
                }
                Type::Tuple(elems) => {
                    let idx = match index.as_ref() {
                        Expr::Int(n, _) => *n,
                        _ => anyhow::bail!("tuple index must be a constant integer"),
                    };
                    if idx < 0 || idx as usize >= elems.len() {
                        anyhow::bail!("tuple index out of bounds in lowering");
                    }
                    let layout = tuple_layout(&elems, ctx.aliases)?;
                    let elem_ty = elems[idx as usize].clone();
                    let mem_ty = mem_ir_type(&elem_ty, ctx.aliases)?;
                    let offset = *layout
                        .offsets
                        .get(idx as usize)
                        .ok_or_else(|| anyhow::anyhow!("tuple offset missing"))?;
                    let dst = fresh(ctx);
                    ctx.body.push(Instr::Load {
                        dst,
                        ptr: base_ptr,
                        offset,
                        ty: mem_ty,
                    });
                    Ok(dst)
                }
                other => anyhow::bail!("indexing not supported for {:?}", other),
            }
        }
        Expr::Bin { op, lhs, rhs, .. } => {
            let lt = infer_expr_type(lhs, &ctx.type_env, &ctx.fns, ctx.trait_env, ctx.aliases, ctx.type_defs, &ctx.type_params, &ctx.bounds)?;
            let rt = infer_expr_type(rhs, &ctx.type_env, &ctx.fns, ctx.trait_env, ctx.aliases, ctx.type_defs, &ctx.type_params, &ctx.bounds)?;
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
            if let Some(val) =
                lower_collection_call(ctx, e, callee.as_str(), args, expected.as_ref())?
            {
                return Ok(val);
            }
            match callee.as_str() {
            "U8" => {
                if args.len() != 1 {
                    anyhow::bail!("`U8` expects exactly one argument");
                }
                lower_expr(ctx, &args[0], Some(Type::U8))
            }
            "U64" => {
                if args.len() != 1 {
                    anyhow::bail!("`U64` expects exactly one argument");
                }
                lower_expr(ctx, &args[0], Some(Type::U64))
            }
            "U128" => {
                if args.len() != 1 {
                    anyhow::bail!("`U128` expects exactly one argument");
                }
                let arg_ty = infer_expr_type(&args[0], &ctx.type_env, &ctx.fns, ctx.trait_env, ctx.aliases, ctx.type_defs, &ctx.type_params, &ctx.bounds)?;
                if matches!(arg_ty, Type::U128) {
                    return lower_expr(ctx, &args[0], Some(Type::U128));
                }
                let limb_lo = lower_expr(ctx, &args[0], Some(Type::U64))?;
                let limb_hi = emit_u64_const(ctx, 0);
                let dst = fresh(ctx);
                ctx.body.push(Instr::U128Init {
                    dst,
                    limb_lo,
                    limb_hi,
                });
                Ok(dst)
            }
            "U256" => {
                if args.len() != 1 {
                    anyhow::bail!("`U256` expects exactly one argument");
                }
                let arg_ty = infer_expr_type(&args[0], &ctx.type_env, &ctx.fns, ctx.trait_env, ctx.aliases, ctx.type_defs, &ctx.type_params, &ctx.bounds)?;
                if matches!(arg_ty, Type::U256) {
                    return lower_expr(ctx, &args[0], Some(Type::U256));
                }
                let limb0 = lower_expr(ctx, &args[0], Some(Type::U64))?;
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
            "std::u64::add_wrap" => lower_u64_wrap(ctx, BinOp::Add, args),
            "std::u64::sub_wrap" => lower_u64_wrap(ctx, BinOp::Sub, args),
            "std::u64::mul_wrap" => lower_u64_wrap(ctx, BinOp::Mul, args),
            "std::u64::add_sat" => lower_u64_sat(ctx, BinOp::Add, args),
            "std::u64::sub_sat" => lower_u64_sat(ctx, BinOp::Sub, args),
            "std::u64::mul_sat" => lower_u64_sat(ctx, BinOp::Mul, args),
            "std::u128::from_limbs" => lower_u128_from_limbs(ctx, args),
            "std::u128::lo" => lower_u128_load(ctx, args, 0),
            "std::u128::hi" => lower_u128_load(ctx, args, 1),
            "std::u256::from_limbs" => lower_u256_from_limbs(ctx, args),
            "std::u256::limb0" => lower_u256_load(ctx, args, 0),
            "std::u256::limb1" => lower_u256_load(ctx, args, 1),
            "std::u256::limb2" => lower_u256_load(ctx, args, 2),
            "std::u256::limb3" => lower_u256_load(ctx, args, 3),
            "Some" => {
                if args.len() != 1 {
                    anyhow::bail!("`Some` expects exactly one argument");
                }
                let payload = lower_expr(ctx, &args[0], None)?;
                let tag = emit_int_const(ctx, 1);
                let zero = emit_int_const(ctx, 0);
                Ok(ctx.variant_init(tag, payload, zero))
            }
            "None" => {
                if !args.is_empty() {
                    anyhow::bail!("`None` does not take arguments");
                }
                let tag = emit_int_const(ctx, 0);
                let zero = emit_int_const(ctx, 0);
                Ok(ctx.variant_init(tag, zero, zero))
            }
            "Ok" => {
                if args.len() > 1 {
                    anyhow::bail!("`Ok` expects at most one argument");
                }
                let payload = if let Some(arg) = args.first() {
                    lower_expr(ctx, arg, None)?
                } else {
                    emit_int_const(ctx, 0)
                };
                let tag = emit_int_const(ctx, 1);
                let zero = emit_int_const(ctx, 0);
                Ok(ctx.variant_init(tag, payload, zero))
            }
            "Err" => {
                if args.len() > 1 {
                    anyhow::bail!("`Err` expects at most one argument");
                }
                let payload = if let Some(arg) = args.first() {
                    lower_expr(ctx, arg, None)?
                } else {
                    emit_int_const(ctx, 0)
                };
                let tag = emit_int_const(ctx, 0);
                let zero = emit_int_const(ctx, 0);
                Ok(ctx.variant_init(tag, payload, zero))
            }
            _ => {
                if let Some((enum_name, variant_name)) = callee.split_once("::") {
                    if ctx.type_defs.enums.contains_key(enum_name) {
                        let call_ty = infer_expr_type(
                            e,
                            &ctx.type_env,
                            &ctx.fns,
                            ctx.trait_env,
                            ctx.aliases,
                            ctx.type_defs,
                            &ctx.type_params,
                            &ctx.bounds,
                        )?;
                        let resolved = base_type(&call_ty, ctx.aliases)?;
                        let enum_args = match resolved {
                            Type::Named { args, .. } => args,
                            _ => Vec::new(),
                        };
                        if let Some(val) =
                            lower_enum_constructor(ctx, enum_name, variant_name, args, &enum_args)?
                        {
                            return Ok(val);
                        }
                    }
                }
                if guard_kind_for_callee(callee.as_str()).is_some() {
                    let dst = fresh(ctx);
                    ctx.body.push(Instr::IConst {
                        dst,
                        ty: IrType::Bool,
                        n: 1,
                    });
                    return Ok(dst);
                }
                let sig = ctx
                    .fns
                    .get(callee.as_str())
                    .cloned()
                    .ok_or_else(|| anyhow::anyhow!(format!("unknown function `{}`", callee)))?;
                let argv: Result<Vec<_>> = args
                    .iter()
                    .enumerate()
                    .map(|(i, a)| {
                        let expected_ty = sig.params.get(i).map(|p| p.ty.clone());
                        lower_expr(ctx, a, expected_ty)
                    })
                    .collect();
                let argv = argv?;
                if let Some(idx) = ctx.fn_indices.get(callee.as_str()).copied() {
                    let dst = fresh(ctx);
                    ctx.body.push(Instr::Call {
                        dst: Some(dst),
                        callee: idx,
                        args: argv,
                    });
                    Ok(dst)
                } else {
                    anyhow::bail!(format!("unknown function `{}`", callee))
                }
            }
            }
        },
    }
}

fn lower_block_expr<'a>(
    ctx: &mut LowerCtx<'a>,
    block: &'a Block,
    expected: Option<Type>,
) -> Result<Value> {
    let mut inserted: Vec<ScopeEntry<'a>> = Vec::new();
    let result = (|| -> Result<Value> {
        lower_block_statements(ctx, block, &mut inserted)?;
        let tail = block
            .tail
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("block expressions require a tail value"))?;
        lower_expr(ctx, tail.as_ref(), expected)
    })();
    restore_scope(ctx, inserted);
    result
}

fn lower_block_stmt<'a>(ctx: &mut LowerCtx<'a>, block: &'a Block) -> Result<()> {
    let mut inserted: Vec<ScopeEntry<'a>> = Vec::new();
    let result = (|| -> Result<()> {
        lower_block_statements(ctx, block, &mut inserted)?;
        if let Some(tail) = &block.tail {
            let _ = lower_expr(ctx, tail.as_ref(), None)?;
        }
        Ok(())
    })();
    restore_scope(ctx, inserted);
    result
}

fn lower_block_statements<'a>(
    ctx: &mut LowerCtx<'a>,
    block: &'a Block,
    inserted: &mut Vec<ScopeEntry<'a>>,
) -> Result<()> {
    for stmt in &block.statements {
        match stmt {
            Stmt::Let { name, expr, .. } => {
                let val = lower_expr(ctx, expr.as_ref(), None)?;
                let ty = infer_expr_type(expr.as_ref(), &ctx.type_env, &ctx.fns, ctx.trait_env, ctx.aliases, ctx.type_defs, &ctx.type_params, &ctx.bounds)?;
                let key = name.as_str();
                let prev = ctx.env.insert(key, val);
                let prev_ty = ctx.type_env.insert(
                    key,
                    LocalBinding {
                        ty,
                        kind: ParamKind::Borrow,
                    },
                );
                inserted.push(ScopeEntry {
                    name: key,
                    prev_val: prev,
                    prev_ty,
                });
            }
            Stmt::Expr { expr, .. } => {
                let _ = lower_expr(ctx, expr.as_ref(), None)?;
            }
            Stmt::While {
                cond,
                invariant,
                variant,
                body,
                span,
            } => {
                lower_while_stmt(
                    ctx,
                    cond.as_ref(),
                    invariant.as_ref(),
                    variant.as_ref().map(|v| v.as_ref()),
                    body.as_ref(),
                    *span,
                )?;
            }
        }
    }
    Ok(())
}

struct ScopeEntry<'a> {
    name: &'a str,
    prev_val: Option<Value>,
    prev_ty: Option<LocalBinding>,
}

fn restore_scope<'a>(ctx: &mut LowerCtx<'a>, inserted: Vec<ScopeEntry<'a>>) {
    for entry in inserted.into_iter().rev() {
        match entry.prev_val {
            Some(val) => {
                ctx.env.insert(entry.name, val);
            }
            None => {
                ctx.env.remove(entry.name);
            }
        }
        match entry.prev_ty {
            Some(ty) => {
                ctx.type_env.insert(entry.name, ty);
            }
            None => {
                ctx.type_env.remove(entry.name);
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn lower_while_stmt<'a>(
    ctx: &mut LowerCtx<'a>,
    cond: &'a Expr,
    invariant: &'a Expr,
    variant: Option<&'a Expr>,
    body: &'a Block,
    _span: Span,
) -> Result<()> {
    ctx.body.push(Instr::BlockBegin);
    ctx.body.push(Instr::LoopBegin);

    let inv_span = expr_span_local(invariant);
    let inv_head = lower_expr(ctx, invariant, None)?;
    push_guard(ctx, inv_head, inv_span, GuardKind::LoopInvariant);

    let cond_val = lower_expr(ctx, cond, None)?;
    ctx.body.push(Instr::BrIfEqz {
        cond: cond_val,
        depth: 1,
    });

    let mut before_variant: Option<(Value, Span)> = None;
    if let Some(var_expr) = variant {
        let v_before = lower_expr(ctx, var_expr, None)?;
        let v_span = expr_span_local(var_expr);
        push_non_negative_guard(ctx, v_before, v_span);
        before_variant = Some((v_before, v_span));
    }

    lower_block_stmt(ctx, body)?;

    let inv_tail = lower_expr(ctx, invariant, None)?;
    push_guard(ctx, inv_tail, inv_span, GuardKind::LoopInvariant);

    if let Some((v_before, v_span)) = before_variant {
        let v_after = lower_expr(ctx, variant.expect("variant expression lost"), None)?;
        push_non_negative_guard(ctx, v_after, v_span);
        let progress = fresh(ctx);
        ctx.body.push(Instr::IBin {
            dst: progress,
            op: BinOpIR::Lt,
            lhs: v_after,
            rhs: v_before,
            ty: IrType::Int,
        });
        push_guard(ctx, progress, v_span, GuardKind::LoopVariantProgress);
    }

    ctx.body.push(Instr::Br { depth: 0 });
    ctx.body.push(Instr::LoopEnd);
    ctx.body.push(Instr::BlockEnd);
    Ok(())
}

fn push_guard(ctx: &mut LowerCtx<'_>, cond: Value, span: Span, detail: GuardKind) {
    ctx.body.push(Instr::Guard {
        cond,
        trap: TrapCode::ContractViolation,
        span: Some((span.start as u32, span.end as u32)),
        detail,
    });
}

fn push_non_negative_guard(ctx: &mut LowerCtx<'_>, value: Value, span: Span) {
    let zero = emit_int_const(ctx, 0);
    let ok = fresh(ctx);
    ctx.body.push(Instr::IBin {
        dst: ok,
        op: BinOpIR::Ge,
        lhs: value,
        rhs: zero,
        ty: IrType::Int,
    });
    push_guard(ctx, ok, span, GuardKind::LoopVariant);
}

fn expr_span_local(e: &Expr) -> Span {
    match e {
        Expr::Int(_, sp) | Expr::Bool(_, sp) | Expr::String(_, sp) | Expr::Var(_, sp) => *sp,
        Expr::ArrayLit { span, .. }
        | Expr::TupleLit { span, .. }
        | Expr::StructLit { span, .. }
        | Expr::FieldAccess { span, .. }
        | Expr::Index { span, .. } => *span,
        Expr::Bin { span, .. }
        | Expr::Call { span, .. }
        | Expr::Match { span, .. }
        | Expr::Return { span, .. }
        | Expr::If { span, .. }
        | Expr::Unary { span, .. }
        | Expr::Try { span, .. } => *span,
        Expr::Block { block } => block.span,
    }
}

fn lower_match_sugar<'a>(
    ctx: &mut LowerCtx<'a>,
    scrutinee: &'a Expr,
    arms: &'a [MatchArm],
    expected: Option<Type>,
) -> Result<Option<Value>> {
    if arms.len() != 2 {
        return Ok(None);
    }

    let success = &arms[0];
    let failure = &arms[1];

    let (success_tag, binder_name, kind): (i32, Option<&str>, VariantKind) =
        match (&success.pat, &failure.pat) {
            (MatchPat::Some(name), MatchPat::None) => (1, Some(name.as_str()), VariantKind::Option),
            (MatchPat::Ok(name), MatchPat::Err(_)) => (1, Some(name.as_str()), VariantKind::Result),
            (MatchPat::Err(name), MatchPat::Ok(_)) => (0, Some(name.as_str()), VariantKind::Result),
            _ => return Ok(None),
        };

    let variant = lower_expr(ctx, scrutinee, None)?;
    let parts = ctx.variant_destructure(variant, kind);
    let success_tag_val = emit_int_const(ctx, success_tag as i64);
    let cond = fresh(ctx);
    ctx.body.push(Instr::IBin {
        dst: cond,
        op: BinOpIR::Eq,
        lhs: parts.tag,
        rhs: success_tag_val,
        ty: IrType::Int,
    });

    let scrut_ty = infer_expr_type(scrutinee, &ctx.type_env, &ctx.fns, ctx.trait_env, ctx.aliases, ctx.type_defs, &ctx.type_params, &ctx.bounds)?;
    let binder_ty = match scrut_ty {
        Type::Option(inner) => *inner,
        Type::Result(ok, err) => {
            if matches!(success.pat, MatchPat::Err(_)) {
                *err
            } else {
                *ok
            }
        }
        _ => Type::Int,
    };

    let (binder_name_opt, previous) = if let Some(name) = binder_name {
        let prev = ctx.env.insert(name, parts.payload_lo);
        let prev_ty = ctx.type_env.insert(
            name,
            LocalBinding {
                ty: binder_ty,
                kind: ParamKind::Borrow,
            },
        );
        (Some(name), (prev, prev_ty))
    } else {
        (None, (None, None))
    };

    let success_val = lower_expr(ctx, &success.expr, expected.clone())?;

    if let Some(name) = binder_name_opt {
        let (prev, prev_ty) = previous;
        if let Some(prev) = prev {
            ctx.env.insert(name, prev);
        } else {
            ctx.env.remove(name);
        }
        if let Some(prev_ty) = prev_ty {
            ctx.type_env.insert(name, prev_ty);
        } else {
            ctx.type_env.remove(name);
        }
    }

    let failure_val = lower_expr(ctx, &failure.expr, expected)?;
    let dst = fresh(ctx);
    ctx.body.push(Instr::ISelect {
        dst,
        cond,
        then_v: success_val,
        else_v: failure_val,
    });
    Ok(Some(dst))
}

fn lower_enum_constructor<'a>(
    ctx: &mut LowerCtx<'a>,
    enum_name: &str,
    variant_name: &str,
    args: &'a [Expr],
    enum_args: &[Type],
) -> Result<Option<Value>> {
    if !ctx.type_defs.enums.contains_key(enum_name) {
        return Ok(None);
    }
    let (index, fields, _count) =
        enum_variant_info(ctx.type_defs, enum_name, enum_args, variant_name)?;
    if args.len() != fields.len() {
        anyhow::bail!(
            "`{}::{}` expects {} argument(s)",
            enum_name,
            variant_name,
            fields.len()
        );
    }
    let tag = emit_int_const(ctx, index as i64);
    let zero = emit_int_const(ctx, 0);
    let payload_lo = match fields.len() {
        0 => zero,
        1 => lower_expr(ctx, &args[0], Some(fields[0].clone()))?,
        _ => {
            let mut field_bases = Vec::with_capacity(fields.len());
            for ty in &fields {
                field_bases.push(base_type(ty, ctx.aliases)?);
            }
            let layout = tuple_layout(&field_bases, ctx.aliases)?;
            let ptr = emit_alloc(ctx, layout.size, layout.align);
            for (idx, arg) in args.iter().enumerate() {
                let expected = fields
                    .get(idx)
                    .cloned()
                    .ok_or_else(|| anyhow::anyhow!("enum variant field missing"))?;
                let val = lower_expr(ctx, arg, Some(expected.clone()))?;
                let mem_ty = mem_ir_type(&expected, ctx.aliases)?;
                let offset = *layout
                    .offsets
                    .get(idx)
                    .ok_or_else(|| anyhow::anyhow!("enum variant offset missing"))?;
                ctx.body.push(Instr::Store {
                    ptr,
                    src: val,
                    offset,
                    ty: mem_ty,
                });
            }
            ptr
        }
    };
    Ok(Some(ctx.variant_init(tag, payload_lo, zero)))
}

fn lower_enum_match<'a>(
    ctx: &mut LowerCtx<'a>,
    scrutinee: &'a Expr,
    arms: &'a [MatchArm],
    expected: Option<Type>,
    enum_name: &str,
    enum_args: &[Type],
) -> Result<Value> {
    let scrut_ty = infer_expr_type(scrutinee, &ctx.type_env, &ctx.fns, ctx.trait_env, ctx.aliases, ctx.type_defs, &ctx.type_params, &ctx.bounds)?;
    let resolved = base_type(&scrut_ty, ctx.aliases)?;
    let Type::Named { name, .. } = resolved else {
        anyhow::bail!("enum match expects enum scrutinee");
    };
    let info = ctx
        .type_defs
        .enums
        .get(enum_name)
        .ok_or_else(|| anyhow::anyhow!("unknown enum `{}`", name))?;
    let variant_count = info.decl.variants.len() as u32;
    let variant = lower_expr(ctx, scrutinee, None)?;
    let parts = ctx.variant_destructure(variant, VariantKind::Enum { max_tag: variant_count });

    let mut result_slot: Option<(Value, Type, IrType)> = None;

    ctx.body.push(Instr::BlockBegin);
    for arm in arms {
        match &arm.pat {
            MatchPat::EnumVariant {
                enum_name,
                variant,
                binders,
            } => {
                let (index, field_types, _count) = enum_variant_info(
                    ctx.type_defs,
                    enum_name.as_str(),
                    enum_args,
                    variant.as_str(),
                )?;
                ctx.body.push(Instr::BlockBegin);
                let cond = fresh(ctx);
                let tag_val = emit_int_const(ctx, index as i64);
                ctx.body.push(Instr::IBin {
                    dst: cond,
                    op: BinOpIR::Eq,
                    lhs: parts.tag,
                    rhs: tag_val,
                    ty: IrType::Int,
                });
                ctx.body.push(Instr::BrIfEqz { cond, depth: 0 });

                let mut inserted: Vec<ScopeEntry<'a>> = Vec::new();
                match field_types.len() {
                    0 => {}
                    1 => {
                        if let Some(name) = binders.first() {
                            let key = name.as_str();
                            let prev = ctx.env.insert(key, parts.payload_lo);
                            let prev_ty = ctx.type_env.insert(
                                key,
                                LocalBinding {
                                    ty: field_types[0].clone(),
                                    kind: ParamKind::Borrow,
                                },
                            );
                            inserted.push(ScopeEntry {
                                name: key,
                                prev_val: prev,
                                prev_ty,
                            });
                        }
                    }
                    _ => {
                        let mut field_bases = Vec::with_capacity(field_types.len());
                        for ty in &field_types {
                            field_bases.push(base_type(ty, ctx.aliases)?);
                        }
                        let layout = tuple_layout(&field_bases, ctx.aliases)?;
                        for (idx, name) in binders.iter().enumerate() {
                            let expected = field_types
                                .get(idx)
                                .cloned()
                                .ok_or_else(|| anyhow::anyhow!("enum binder type missing"))?;
                            let offset = *layout
                                .offsets
                                .get(idx)
                                .ok_or_else(|| anyhow::anyhow!("enum binder offset missing"))?;
                            let mem_ty = mem_ir_type(&expected, ctx.aliases)?;
                            let dst = fresh(ctx);
                            ctx.body.push(Instr::Load {
                                dst,
                                ptr: parts.payload_lo,
                                offset,
                                ty: mem_ty,
                            });
                            let key = name.as_str();
                            let prev = ctx.env.insert(key, dst);
                            let prev_ty = ctx.type_env.insert(
                                key,
                                LocalBinding {
                                    ty: expected,
                                    kind: ParamKind::Borrow,
                                },
                            );
                            inserted.push(ScopeEntry {
                                name: key,
                                prev_val: prev,
                                prev_ty,
                            });
                        }
                    }
                }

                let arm_val = lower_expr(ctx, &arm.expr, expected.clone())?;
                let arm_ty = if let Some(ty) = &expected {
                    ty.clone()
                } else {
                    infer_expr_type(
                        &arm.expr,
                        &ctx.type_env,
                        &ctx.fns, ctx.trait_env, ctx.aliases, ctx.type_defs, &ctx.type_params, &ctx.bounds,
                    )?
                };
                let mem_ty = mem_ir_type(&arm_ty, ctx.aliases)?;
                let (slot, _, slot_mem_ty) = if let Some(slot) = result_slot.clone() {
                    slot
                } else {
                    let (size, align) = mem_layout_for_ir(mem_ty);
                    let slot = emit_alloc(ctx, size, align);
                    result_slot = Some((slot, arm_ty.clone(), mem_ty));
                    (slot, arm_ty.clone(), mem_ty)
                };
                if mem_ty != slot_mem_ty {
                    anyhow::bail!("match arm lowered to mismatched runtime type");
                }
                ctx.body.push(Instr::Store {
                    ptr: slot,
                    src: arm_val,
                    offset: 0,
                    ty: mem_ty,
                });
                restore_scope(ctx, inserted);
                ctx.body.push(Instr::Br { depth: 1 });
                ctx.body.push(Instr::BlockEnd);
            }
            MatchPat::Wildcard => {
                ctx.body.push(Instr::BlockBegin);
                let arm_val = lower_expr(ctx, &arm.expr, expected.clone())?;
                let arm_ty = if let Some(ty) = &expected {
                    ty.clone()
                } else {
                    infer_expr_type(
                        &arm.expr,
                        &ctx.type_env,
                        &ctx.fns, ctx.trait_env, ctx.aliases, ctx.type_defs, &ctx.type_params, &ctx.bounds,
                    )?
                };
                let mem_ty = mem_ir_type(&arm_ty, ctx.aliases)?;
                let (slot, _, slot_mem_ty) = if let Some(slot) = result_slot.clone() {
                    slot
                } else {
                    let (size, align) = mem_layout_for_ir(mem_ty);
                    let slot = emit_alloc(ctx, size, align);
                    result_slot = Some((slot, arm_ty.clone(), mem_ty));
                    (slot, arm_ty.clone(), mem_ty)
                };
                if mem_ty != slot_mem_ty {
                    anyhow::bail!("match arm lowered to mismatched runtime type");
                }
                ctx.body.push(Instr::Store {
                    ptr: slot,
                    src: arm_val,
                    offset: 0,
                    ty: mem_ty,
                });
                ctx.body.push(Instr::Br { depth: 1 });
                ctx.body.push(Instr::BlockEnd);
            }
            _ => anyhow::bail!("unsupported match pattern in lowering"),
        }
    }
    ctx.body.push(Instr::BlockEnd);

    let (slot, arm_ty, mem_ty) = result_slot
        .ok_or_else(|| anyhow::anyhow!("match arms must not be empty"))?;
    let dst = fresh(ctx);
    ctx.body.push(Instr::Load {
        dst,
        ptr: slot,
        offset: 0,
        ty: mem_ty,
    });
    let _ = arm_ty;
    Ok(dst)
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

fn emit_collection_guard(ctx: &mut LowerCtx<'_>, cond: Value, span: Span) {
    ctx.body.push(Instr::Guard {
        cond,
        trap: TrapCode::CollectionBounds,
        span: Some((span.start as u32, span.end as u32)),
        detail: GuardKind::Require,
    });
}

fn emit_collection_len(ctx: &mut LowerCtx<'_>, ptr: Value) -> Value {
    emit_load_i32(ctx, ptr, COLLECTION_LEN_OFFSET)
}

fn emit_collection_data_ptr(ctx: &mut LowerCtx<'_>, ptr: Value) -> Value {
    emit_load_i32(ctx, ptr, COLLECTION_DATA_OFFSET)
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

fn emit_eq_for_type(
    ctx: &mut LowerCtx<'_>,
    ty: &Type,
    lhs: Value,
    rhs: Value,
    aliases: &AliasMap,
) -> Result<Value> {
    let base = base_type(ty, aliases)?;
    match base {
        Type::String => emit_intrinsic_eq(ctx, "std::str::eq", lhs, rhs),
        Type::Bytes => emit_intrinsic_eq(ctx, "std::bytes::eq", lhs, rhs),
        Type::U128 => Ok(emit_eq_u128(ctx, lhs, rhs)),
        Type::U256 => Ok(emit_eq_u256(ctx, lhs, rhs)),
        _ => {
            let ir_ty = if matches!(base, Type::U64) {
                IrType::U64
            } else {
                IrType::Int
            };
            let dst = fresh(ctx);
            ctx.body.push(Instr::IBin {
                dst,
                op: BinOpIR::Eq,
                lhs,
                rhs,
                ty: ir_ty,
            });
            Ok(dst)
        }
    }
}

fn emit_intrinsic_eq(
    ctx: &mut LowerCtx<'_>,
    callee: &str,
    lhs: Value,
    rhs: Value,
) -> Result<Value> {
    let idx = ctx
        .fn_indices
        .get(callee)
        .copied()
        .ok_or_else(|| anyhow::anyhow!("missing intrinsic `{}`", callee))?;
    let dst = fresh(ctx);
    ctx.body.push(Instr::Call {
        dst: Some(dst),
        callee: idx,
        args: vec![lhs, rhs],
    });
    Ok(dst)
}

fn emit_eq_u128(ctx: &mut LowerCtx<'_>, lhs: Value, rhs: Value) -> Value {
    let lhs_lo = fresh(ctx);
    ctx.body.push(Instr::U128LoadLimb {
        dst: lhs_lo,
        value: lhs,
        limb: 0,
    });
    let lhs_hi = fresh(ctx);
    ctx.body.push(Instr::U128LoadLimb {
        dst: lhs_hi,
        value: lhs,
        limb: 1,
    });
    let rhs_lo = fresh(ctx);
    ctx.body.push(Instr::U128LoadLimb {
        dst: rhs_lo,
        value: rhs,
        limb: 0,
    });
    let rhs_hi = fresh(ctx);
    ctx.body.push(Instr::U128LoadLimb {
        dst: rhs_hi,
        value: rhs,
        limb: 1,
    });
    let eq_lo = fresh(ctx);
    ctx.body.push(Instr::IBin {
        dst: eq_lo,
        op: BinOpIR::Eq,
        lhs: lhs_lo,
        rhs: rhs_lo,
        ty: IrType::U64,
    });
    let eq_hi = fresh(ctx);
    ctx.body.push(Instr::IBin {
        dst: eq_hi,
        op: BinOpIR::Eq,
        lhs: lhs_hi,
        rhs: rhs_hi,
        ty: IrType::U64,
    });
    let dst = fresh(ctx);
    ctx.body.push(Instr::IBin {
        dst,
        op: BinOpIR::And,
        lhs: eq_lo,
        rhs: eq_hi,
        ty: IrType::Int,
    });
    dst
}

fn emit_eq_u256(ctx: &mut LowerCtx<'_>, lhs: Value, rhs: Value) -> Value {
    let mut lhs_limbs = Vec::with_capacity(4);
    let mut rhs_limbs = Vec::with_capacity(4);
    for limb in 0..4u8 {
        let lhs_limb = fresh(ctx);
        ctx.body.push(Instr::U256LoadLimb {
            dst: lhs_limb,
            value: lhs,
            limb,
        });
        lhs_limbs.push(lhs_limb);
        let rhs_limb = fresh(ctx);
        ctx.body.push(Instr::U256LoadLimb {
            dst: rhs_limb,
            value: rhs,
            limb,
        });
        rhs_limbs.push(rhs_limb);
    }
    let mut eq_vals = Vec::with_capacity(4);
    for (lhs_limb, rhs_limb) in lhs_limbs.into_iter().zip(rhs_limbs.into_iter()) {
        let eq = fresh(ctx);
        ctx.body.push(Instr::IBin {
            dst: eq,
            op: BinOpIR::Eq,
            lhs: lhs_limb,
            rhs: rhs_limb,
            ty: IrType::U64,
        });
        eq_vals.push(eq);
    }
    let mut acc = eq_vals
        .pop()
        .expect("u256 eq must compare at least one limb");
    while let Some(eq) = eq_vals.pop() {
        let next = fresh(ctx);
        ctx.body.push(Instr::IBin {
            dst: next,
            op: BinOpIR::And,
            lhs: acc,
            rhs: eq,
            ty: IrType::Int,
        });
        acc = next;
    }
    acc
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

    let mem_ty = mem_ir_type(key_ty, aliases)?;
    let key_loaded = fresh(ctx);
    ctx.body.push(Instr::Load {
        dst: key_loaded,
        ptr: key_ptr,
        offset: 0,
        ty: mem_ty,
    });
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

fn lower_collection_call<'a>(
    ctx: &mut LowerCtx<'a>,
    call_expr: &'a Expr,
    callee: &str,
    args: &'a [Expr],
    expected: Option<&Type>,
) -> Result<Option<Value>> {
    let callee = normalize_collection_callee(callee);
    match callee {
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
            let (_size, align, stride) = collection_layout(&elem_ty, ctx.aliases)?;
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
            let (_size, _align, stride) = collection_layout(&elem_ty, ctx.aliases)?;
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
            let elem_ptr = emit_ptr_add(ctx, data_ptr, offset);
            let mem_ty = mem_ir_type(&elem_ty, ctx.aliases)?;
            let elem_val = fresh(ctx);
            ctx.body.push(Instr::Load {
                dst: elem_val,
                ptr: elem_ptr,
                offset: 0,
                ty: mem_ty,
            });
            let zero_payload = emit_zero_for_mem_ty(ctx, mem_ty);
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
            let (_size, align, stride) = collection_layout(&elem_ty, ctx.aliases)?;
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
            let mem_ty = mem_ir_type(&elem_ty, ctx.aliases)?;
            ctx.body.push(Instr::Store {
                ptr: elem_ptr,
                src: elem_val,
                offset: 0,
                ty: mem_ty,
            });
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
            let (_size, align, stride) = collection_layout(&elem_ty, ctx.aliases)?;
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
            let mem_ty = mem_ir_type(&elem_ty, ctx.aliases)?;
            ctx.body.push(Instr::Store {
                ptr: elem_ptr,
                src: elem_val,
                offset: 0,
                ty: mem_ty,
            });
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
            let (_size, align, stride) = collection_layout(&elem_ty, ctx.aliases)?;
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
            let (_size, _align, stride) = collection_layout(&elem_ty, ctx.aliases)?;
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
            let elem_ptr = emit_ptr_add(ctx, data_ptr, offset);
            let mem_ty = mem_ir_type(&elem_ty, ctx.aliases)?;
            let elem_val = fresh(ctx);
            ctx.body.push(Instr::Load {
                dst: elem_val,
                ptr: elem_ptr,
                offset: 0,
                ty: mem_ty,
            });
            let zero_payload = emit_zero_for_mem_ty(ctx, mem_ty);
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
            let (_size, align, stride) = collection_layout(&elem_ty, ctx.aliases)?;
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
            let (_size, _align, stride) = collection_layout(&elem_ty, ctx.aliases)?;
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
            let (_size, align, stride) = collection_layout(&elem_ty, ctx.aliases)?;
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
            let mem_ty = mem_ir_type(&elem_ty, ctx.aliases)?;
            ctx.body.push(Instr::Store {
                ptr: elem_ptr,
                src: elem_val,
                offset: 0,
                ty: mem_ty,
            });
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
            let (_size, align, stride) = collection_layout(&elem_ty, ctx.aliases)?;
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
                map_entry_layout(&key_ty, &val_ty, ctx.aliases)?;
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
            let (entry_size, _align, key_offset, _val_offset) =
                map_entry_layout(&key_ty, &val_ty, ctx.aliases)?;
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
            let (entry_size, _align, key_offset, val_offset) =
                map_entry_layout(&key_ty, &val_ty, ctx.aliases)?;
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
            let mem_ty = mem_ir_type(&val_ty, ctx.aliases)?;
            let val_loaded = fresh(ctx);
            ctx.body.push(Instr::Load {
                dst: val_loaded,
                ptr: val_ptr,
                offset: 0,
                ty: mem_ty,
            });
            let zero_payload = emit_zero_for_mem_ty(ctx, mem_ty);
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
                map_entry_layout(&key_ty, &val_ty, ctx.aliases)?;
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
            let val_mem_ty = mem_ir_type(&val_ty, ctx.aliases)?;
            ctx.body.push(Instr::Store {
                ptr: val_ptr,
                src: val_val,
                offset: 0,
                ty: val_mem_ty,
            });
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
            let key_mem_ty = mem_ir_type(&key_ty, ctx.aliases)?;
            ctx.body.push(Instr::Store {
                ptr: key_ptr,
                src: key_val,
                offset: 0,
                ty: key_mem_ty,
            });
            let val_ptr = if val_offset == 0 {
                base_ptr
            } else {
                let val_off_val = emit_int_const(ctx, val_offset as i64);
                emit_ptr_add(ctx, base_ptr, val_off_val)
            };
            let val_mem_ty = mem_ir_type(&val_ty, ctx.aliases)?;
            ctx.body.push(Instr::Store {
                ptr: val_ptr,
                src: val_val,
                offset: 0,
                ty: val_mem_ty,
            });
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
                map_entry_layout(&key_ty, &val_ty, ctx.aliases)?;
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

fn lower_u64_wrap<'a>(ctx: &mut LowerCtx<'a>, op: BinOp, args: &'a [Expr]) -> Result<Value> {
    if args.len() != 2 {
        anyhow::bail!("std::u64::*_wrap expects exactly two arguments");
    }
    let lhs = lower_expr(ctx, &args[0], Some(Type::U64))?;
    let rhs = lower_expr(ctx, &args[1], Some(Type::U64))?;
    Ok(emit_u64_bin(ctx, op, lhs, rhs))
}

fn lower_u64_sat<'a>(ctx: &mut LowerCtx<'a>, op: BinOp, args: &'a [Expr]) -> Result<Value> {
    if args.len() != 2 {
        anyhow::bail!("std::u64::*_sat expects exactly two arguments");
    }
    let lhs = lower_expr(ctx, &args[0], Some(Type::U64))?;
    let rhs = lower_expr(ctx, &args[1], Some(Type::U64))?;
    let raw = emit_u64_bin(ctx, op.clone(), lhs, rhs);
    let Some(overflow) = emit_u64_overflow_flag(ctx, &op, lhs, rhs, raw)? else {
        return Ok(raw);
    };
    let clamp = match op {
        BinOp::Sub => emit_u64_const(ctx, 0),
        _ => emit_u64_const(ctx, u64::MAX),
    };
    let dst = fresh(ctx);
    ctx.body.push(Instr::ISelect {
        dst,
        cond: overflow,
        then_v: clamp,
        else_v: raw,
    });
    Ok(dst)
}

fn lower_u128_from_limbs<'a>(ctx: &mut LowerCtx<'a>, args: &'a [Expr]) -> Result<Value> {
    if args.len() != 2 {
        anyhow::bail!("std::u128::from_limbs expects exactly two arguments");
    }
    let limb_lo = lower_expr(ctx, &args[0], Some(Type::U64))?;
    let limb_hi = lower_expr(ctx, &args[1], Some(Type::U64))?;
    let dst = fresh(ctx);
    ctx.body.push(Instr::U128Init {
        dst,
        limb_lo,
        limb_hi,
    });
    Ok(dst)
}

fn lower_u128_load<'a>(ctx: &mut LowerCtx<'a>, args: &'a [Expr], limb: u8) -> Result<Value> {
    if args.len() != 1 {
        anyhow::bail!("std::u128::lo/hi expects exactly one argument");
    }
    let value = lower_expr(ctx, &args[0], Some(Type::U128))?;
    let dst = fresh(ctx);
    ctx.body.push(Instr::U128LoadLimb { dst, value, limb });
    Ok(dst)
}

fn lower_u256_from_limbs<'a>(ctx: &mut LowerCtx<'a>, args: &'a [Expr]) -> Result<Value> {
    if args.len() != 4 {
        anyhow::bail!("std::u256::from_limbs expects exactly four arguments");
    }
    let limb0 = lower_expr(ctx, &args[0], Some(Type::U64))?;
    let limb1 = lower_expr(ctx, &args[1], Some(Type::U64))?;
    let limb2 = lower_expr(ctx, &args[2], Some(Type::U64))?;
    let limb3 = lower_expr(ctx, &args[3], Some(Type::U64))?;
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

fn lower_u256_load<'a>(ctx: &mut LowerCtx<'a>, args: &'a [Expr], limb: u8) -> Result<Value> {
    if args.len() != 1 {
        anyhow::bail!("std::u256::limb* expects exactly one argument");
    }
    let value = lower_expr(ctx, &args[0], Some(Type::U256))?;
    let dst = fresh(ctx);
    ctx.body.push(Instr::U256LoadLimb { dst, value, limb });
    Ok(dst)
}
