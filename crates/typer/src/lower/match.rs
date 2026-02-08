use crate::check::{base_type, infer_expr_type, LocalBinding};
use anyhow::Result;
use clg_ast::{Expr, MatchArm, MatchPat, ParamKind, Type};
use clg_ir::{BinOpIR, Instr, IrType, Value, VariantKind};

use super::block::{restore_scope, ScopeEntry};
use super::layout::{enum_variant_info, tuple_layout};
use super::{
    emit_alloc, emit_int_const, fresh, load_value_borrow, lower_expr, mem_ir_type,
    mem_layout_for_ir, std_type_info_for, store_value, LowerCtx,
};

pub(super) fn lower_match_expr<'a>(
    ctx: &mut LowerCtx<'a>,
    scrutinee: &'a Expr,
    arms: &'a [MatchArm],
    expected: Option<Type>,
) -> Result<Value> {
    if let Some(val) = lower_match_sugar(ctx, scrutinee, arms, expected.clone())? {
        return Ok(val);
    }
    let scrut_ty = infer_expr_type(
        scrutinee,
        &ctx.type_env,
        &ctx.fns,
        ctx.trait_env,
        ctx.aliases,
        ctx.type_defs,
        &ctx.type_params,
        &ctx.bounds,
    )?;
    let resolved = base_type(&scrut_ty, ctx.aliases)?;
    if let Type::Named { name, args } = resolved {
        if ctx.type_defs.enums.contains_key(name.as_str()) {
            return lower_enum_match(ctx, scrutinee, arms, expected, name.as_str(), &args);
        }
    }
    anyhow::bail!("match expression not supported in lowering yet")
}

pub(super) fn lower_match_sugar<'a>(
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

    let scrut_ty = infer_expr_type(
        scrutinee,
        &ctx.type_env,
        &ctx.fns,
        ctx.trait_env,
        ctx.aliases,
        ctx.type_defs,
        &ctx.type_params,
        &ctx.bounds,
    )?;
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

pub(super) fn lower_enum_constructor<'a>(
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
            let layout = tuple_layout(&field_bases, ctx.aliases, ctx.std_types)?;
            let ptr = emit_alloc(ctx, layout.size, layout.align);
            for (idx, arg) in args.iter().enumerate() {
                let expected = fields
                    .get(idx)
                    .cloned()
                    .ok_or_else(|| anyhow::anyhow!("enum variant field missing"))?;
                let val = lower_expr(ctx, arg, Some(expected.clone()))?;
                let offset = *layout
                    .offsets
                    .get(idx)
                    .ok_or_else(|| anyhow::anyhow!("enum variant offset missing"))?;
                store_value(ctx, &expected, ptr, offset, val)?;
            }
            ptr
        }
    };
    Ok(Some(ctx.variant_init(tag, payload_lo, zero)))
}

pub(super) fn lower_enum_match<'a>(
    ctx: &mut LowerCtx<'a>,
    scrutinee: &'a Expr,
    arms: &'a [MatchArm],
    expected: Option<Type>,
    enum_name: &str,
    enum_args: &[Type],
) -> Result<Value> {
    let scrut_ty = infer_expr_type(
        scrutinee,
        &ctx.type_env,
        &ctx.fns,
        ctx.trait_env,
        ctx.aliases,
        ctx.type_defs,
        &ctx.type_params,
        &ctx.bounds,
    )?;
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
    let parts = ctx.variant_destructure(
        variant,
        VariantKind::Enum {
            max_tag: variant_count,
        },
    );

    let mut result_slot: Option<(Value, Type, Option<IrType>)> = None;

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
                        let layout = tuple_layout(&field_bases, ctx.aliases, ctx.std_types)?;
                        for (idx, name) in binders.iter().enumerate() {
                            let expected = field_types
                                .get(idx)
                                .cloned()
                                .ok_or_else(|| anyhow::anyhow!("enum binder type missing"))?;
                            let offset = *layout
                                .offsets
                                .get(idx)
                                .ok_or_else(|| anyhow::anyhow!("enum binder offset missing"))?;
                            let dst = load_value_borrow(ctx, &expected, parts.payload_lo, offset)?;
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
                        &ctx.fns,
                        ctx.trait_env,
                        ctx.aliases,
                        ctx.type_defs,
                        &ctx.type_params,
                        &ctx.bounds,
                    )?
                };
                let (slot, _, slot_mem_ty) = if let Some(slot) = result_slot.clone() {
                    slot
                } else if let Some(info) = std_type_info_for(&arm_ty, ctx.aliases, ctx.std_types)? {
                    let slot = emit_alloc(ctx, info.byte_len, info.align);
                    result_slot = Some((slot, arm_ty.clone(), None));
                    (slot, arm_ty.clone(), None)
                } else {
                    let mem_ty = mem_ir_type(&arm_ty, ctx.aliases)?;
                    let (size, align) = mem_layout_for_ir(mem_ty);
                    let slot = emit_alloc(ctx, size, align);
                    result_slot = Some((slot, arm_ty.clone(), Some(mem_ty)));
                    (slot, arm_ty.clone(), Some(mem_ty))
                };
                if slot_mem_ty.is_some() {
                    let mem_ty = mem_ir_type(&arm_ty, ctx.aliases)?;
                    if Some(mem_ty) != slot_mem_ty {
                        anyhow::bail!("match arm lowered to mismatched runtime type");
                    }
                } else if std_type_info_for(&arm_ty, ctx.aliases, ctx.std_types)?.is_none() {
                    anyhow::bail!("match arm lowered to mismatched runtime type");
                }
                store_value(ctx, &arm_ty, slot, 0, arm_val)?;
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
                        &ctx.fns,
                        ctx.trait_env,
                        ctx.aliases,
                        ctx.type_defs,
                        &ctx.type_params,
                        &ctx.bounds,
                    )?
                };
                let (slot, _, slot_mem_ty) = if let Some(slot) = result_slot.clone() {
                    slot
                } else if let Some(info) = std_type_info_for(&arm_ty, ctx.aliases, ctx.std_types)? {
                    let slot = emit_alloc(ctx, info.byte_len, info.align);
                    result_slot = Some((slot, arm_ty.clone(), None));
                    (slot, arm_ty.clone(), None)
                } else {
                    let mem_ty = mem_ir_type(&arm_ty, ctx.aliases)?;
                    let (size, align) = mem_layout_for_ir(mem_ty);
                    let slot = emit_alloc(ctx, size, align);
                    result_slot = Some((slot, arm_ty.clone(), Some(mem_ty)));
                    (slot, arm_ty.clone(), Some(mem_ty))
                };
                if slot_mem_ty.is_some() {
                    let mem_ty = mem_ir_type(&arm_ty, ctx.aliases)?;
                    if Some(mem_ty) != slot_mem_ty {
                        anyhow::bail!("match arm lowered to mismatched runtime type");
                    }
                } else if std_type_info_for(&arm_ty, ctx.aliases, ctx.std_types)?.is_none() {
                    anyhow::bail!("match arm lowered to mismatched runtime type");
                }
                store_value(ctx, &arm_ty, slot, 0, arm_val)?;
                ctx.body.push(Instr::Br { depth: 1 });
                ctx.body.push(Instr::BlockEnd);
            }
            _ => anyhow::bail!("unsupported match pattern in lowering"),
        }
    }
    ctx.body.push(Instr::BlockEnd);

    let (slot, arm_ty, mem_ty) =
        result_slot.ok_or_else(|| anyhow::anyhow!("match arms must not be empty"))?;
    if let Some(mem_ty) = mem_ty {
        let dst = fresh(ctx);
        ctx.body.push(Instr::Load {
            dst,
            ptr: slot,
            offset: 0,
            ty: mem_ty,
        });
        Ok(dst)
    } else {
        let _ = arm_ty;
        Ok(slot)
    }
}
