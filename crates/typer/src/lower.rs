use crate::check::{
    base_type, infer_expr_type, AliasMap, FnSig as CheckFnSig, LocalBinding, TypeDefs,
};
use crate::guards::guard_kind_for_callee;
use anyhow::Result;
use clg_ast::{BinOp, Block, Expr, Func, MatchArm, MatchPat, ParamKind, Span, Stmt, Type};
use clg_ir::{
    BinOpIR, Function as IrFunction, GuardKind, Instr, IrType, TrapCode, Value, VariantKind,
    VariantParts,
};
use std::collections::HashMap;

type FnSig = CheckFnSig;

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
        Type::Resource(_) => IrType::Int,
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
        Type::Resource(_) => Ok((4, 4)),
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
    pub type_defs: &'a TypeDefs<'a>,
    pub body: Vec<Instr>,
    pub ret_ty: Type,
}

pub(crate) fn lower_func<'a>(
    f: &'a Func,
    fns: &HashMap<&'a str, FnSig>,
    fn_indices: &HashMap<&'a str, u32>,
    aliases: &'a AliasMap,
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
        type_defs,
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
            let elem_ty = infer_expr_type(first, &ctx.type_env, &ctx.fns, ctx.aliases, ctx.type_defs)?;
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
                let ty = infer_expr_type(elem, &ctx.type_env, &ctx.fns, ctx.aliases, ctx.type_defs)?;
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
        Expr::StructLit { .. } => {
            anyhow::bail!("struct literals are not supported in codegen yet")
        }
        Expr::FieldAccess { .. } => {
            anyhow::bail!("field access is not supported in codegen yet")
        }
        Expr::Unary { .. } => {
            anyhow::bail!("unary operators are not supported in codegen yet")
        }
        Expr::Match {
            scrutinee, arms, ..
        } => {
            if let Some(val) = lower_match_sugar(ctx, scrutinee, arms, expected)? {
                Ok(val)
            } else {
                anyhow::bail!("match expression not supported in lowering yet")
            }
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
            let base_ty = infer_expr_type(base, &ctx.type_env, &ctx.fns, ctx.aliases, ctx.type_defs)?;
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
            let lt = infer_expr_type(lhs, &ctx.type_env, &ctx.fns, ctx.aliases, ctx.type_defs)?;
            let rt = infer_expr_type(rhs, &ctx.type_env, &ctx.fns, ctx.aliases, ctx.type_defs)?;
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
        Expr::Call { callee, args, .. } => match callee.as_str() {
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
                let arg_ty = infer_expr_type(&args[0], &ctx.type_env, &ctx.fns, ctx.aliases, ctx.type_defs)?;
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
                let arg_ty = infer_expr_type(&args[0], &ctx.type_env, &ctx.fns, ctx.aliases, ctx.type_defs)?;
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
                let ty = infer_expr_type(expr.as_ref(), &ctx.type_env, &ctx.fns, ctx.aliases, ctx.type_defs)?;
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

    let scrut_ty = infer_expr_type(scrutinee, &ctx.type_env, &ctx.fns, ctx.aliases, ctx.type_defs)?;
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
