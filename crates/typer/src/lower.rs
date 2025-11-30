use crate::check::FnSig as CheckFnSig;
use crate::guards::guard_kind_for_callee;
use anyhow::Result;
use clg_ast::{BinOp, Expr, Func, MatchArm, MatchPat, Stmt, Type};
use clg_ir::{
    BinOpIR, Function as IrFunction, GuardKind, Instr, IrType, TrapCode, Value, VariantKind,
    VariantParts,
};
use std::collections::HashMap;

type FnSig<'a> = CheckFnSig<'a>;

fn ir_ty(t: Type) -> IrType {
    match t {
        Type::Int => IrType::Int,
        Type::Bool => IrType::Bool,
        Type::String => IrType::Int, // placeholder until strings have a runtime representation
        Type::Resource(_) => IrType::Int,
        Type::Option(_) => IrType::Int,
        Type::Result(_, _) => IrType::Int,
        Type::List(_) | Type::Set(_) | Type::Map(_, _) => IrType::Int,
    }
}

pub(crate) struct LowerCtx<'a> {
    pub next: u32,
    pub env: HashMap<&'a str, Value>,
    pub fns: HashMap<&'a str, FnSig<'a>>, // for call return types
    pub fn_indices: HashMap<&'a str, u32>, // for resolving callee indices (user + intrinsics)
    pub body: Vec<Instr>,
    pub ret_ty: Type,
}

pub(crate) fn lower_func<'a>(
    f: &'a Func,
    fns: &HashMap<&'a str, FnSig<'a>>,
    fn_indices: &HashMap<&'a str, u32>,
) -> Result<IrFunction> {
    let mut env: HashMap<&str, Value> = HashMap::new();
    for (i, p) in f.params.iter().enumerate() {
        env.insert(p.name.as_str(), Value(i as u32));
    }
    let mut ctx = LowerCtx {
        next: f.params.len() as u32,
        env,
        fns: fns.clone(),
        fn_indices: fn_indices.clone(),
        body: Vec::new(),
        ret_ty: f.ret.clone(),
    };

    for req in &f.requires {
        let cond = lower_expr(&mut ctx, &req.expr)?;
        ctx.body.push(Instr::Guard {
            cond,
            trap: TrapCode::ContractViolation,
            span: Some((req.span.start as u32, req.span.end as u32)),
            detail: GuardKind::Require,
        });
    }

    let ret_val = lower_expr(&mut ctx, &f.body)?;

    if !f.ensures.is_empty() {
        ctx.env.insert("result", ret_val);
        for ens in &f.ensures {
            let cond = lower_expr(&mut ctx, &ens.expr)?;
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

fn lower_expr<'a>(ctx: &mut LowerCtx<'a>, e: &'a Expr) -> Result<Value> {
    match e {
        Expr::Int(n, _) => {
            let dst = fresh(ctx);
            ctx.body.push(Instr::IConst {
                dst,
                ty: IrType::Int,
                n: *n,
            });
            Ok(dst)
        }
        Expr::Block { block } => {
            let mut inserted: Vec<(&str, Option<Value>)> = Vec::new();
            let result = (|| -> Result<Value> {
                for stmt in &block.statements {
                    match stmt {
                        Stmt::Let { name, expr, .. } => {
                            let val = lower_expr(ctx, expr.as_ref())?;
                            let key = name.as_str();
                            let prev = ctx.env.insert(key, val);
                            inserted.push((key, prev));
                        }
                        Stmt::Expr { expr, .. } => {
                            let _ = lower_expr(ctx, expr.as_ref())?;
                        }
                        Stmt::While { span, .. } => {
                            anyhow::bail!(
                                "while loops are not lowered yet (span {}..{})",
                                span.start,
                                span.end
                            );
                        }
                    }
                }
                let tail = block
                    .tail
                    .as_ref()
                    .ok_or_else(|| anyhow::anyhow!("block expressions require a tail value"))?;
                lower_expr(ctx, tail.as_ref())
            })();
            for (key, prev) in inserted.into_iter().rev() {
                match prev {
                    Some(val) => {
                        ctx.env.insert(key, val);
                    }
                    None => {
                        ctx.env.remove(key);
                    }
                }
            }
            result
        }
        Expr::Return { expr, .. } => {
            // For expression-bodied functions, `return e` is equivalent to `e`.
            // Lower inner expression; the enclosing function appends the Ret.
            lower_expr(ctx, expr)
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
        Expr::Unary { .. } => {
            anyhow::bail!("unary operators are not supported in codegen yet")
        }
        Expr::Match {
            scrutinee, arms, ..
        } => {
            if let Some(val) = lower_match_sugar(ctx, scrutinee, arms)? {
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
            let variant = lower_expr(ctx, expr)?;
            let parts = ctx.variant_destructure(variant, kind);
            let failure_tag = emit_int_const(ctx, 0);
            let cond = fresh(ctx);
            ctx.body.push(Instr::IBin {
                dst: cond,
                op: BinOpIR::Eq,
                lhs: parts.tag,
                rhs: failure_tag,
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
            let cv = lower_expr(ctx, cond)?;
            let tv = lower_expr(ctx, then_br)?;
            let ev = lower_expr(ctx, else_br)?;
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
        Expr::Bin { op, lhs, rhs, .. } => {
            let lv = lower_expr(ctx, lhs)?;
            let rv = lower_expr(ctx, rhs)?;
            let dst = fresh(ctx);
            let irop = match op {
                BinOp::Add => BinOpIR::Add,
                BinOp::Sub => BinOpIR::Sub,
                BinOp::Mul => BinOpIR::Mul,
                BinOp::Div => BinOpIR::Div,
                BinOp::Lt => BinOpIR::Lt,
                BinOp::Le => BinOpIR::Le,
                BinOp::Gt => BinOpIR::Gt,
                BinOp::Ge => BinOpIR::Ge,
                BinOp::Eq => BinOpIR::Eq,
                BinOp::Neq => BinOpIR::Neq,
                BinOp::And => BinOpIR::And,
                BinOp::Or => BinOpIR::Or,
            };
            ctx.body.push(Instr::IBin {
                dst,
                op: irop,
                lhs: lv,
                rhs: rv,
            });
            Ok(dst)
        }
        Expr::Call { callee, args, .. } => match callee.as_str() {
            "Some" => {
                if args.len() != 1 {
                    anyhow::bail!("`Some` expects exactly one argument");
                }
                let payload = lower_expr(ctx, &args[0])?;
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
                    lower_expr(ctx, arg)?
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
                    lower_expr(ctx, arg)?
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
                let argv: Result<Vec<_>> = args.iter().map(|a| lower_expr(ctx, a)).collect();
                let argv = argv?;
                let _sig = ctx
                    .fns
                    .get(callee.as_str())
                    .cloned()
                    .ok_or_else(|| anyhow::anyhow!(format!("unknown function `{}`", callee)))?;
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
fn lower_match_sugar<'a>(
    ctx: &mut LowerCtx<'a>,
    scrutinee: &'a Expr,
    arms: &'a [MatchArm],
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

    let variant = lower_expr(ctx, scrutinee)?;
    let parts = ctx.variant_destructure(variant, kind);
    let success_tag_val = emit_int_const(ctx, success_tag as i64);
    let cond = fresh(ctx);
    ctx.body.push(Instr::IBin {
        dst: cond,
        op: BinOpIR::Eq,
        lhs: parts.tag,
        rhs: success_tag_val,
    });

    let (binder_name_opt, previous) = if let Some(name) = binder_name {
        let prev = ctx.env.insert(name, parts.payload_lo);
        (Some(name), prev)
    } else {
        (None, None)
    };

    let success_val = lower_expr(ctx, &success.expr)?;

    if let Some(name) = binder_name_opt {
        if let Some(prev) = previous {
            ctx.env.insert(name, prev);
        } else {
            ctx.env.remove(name);
        }
    }

    let failure_val = lower_expr(ctx, &failure.expr)?;
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

fn fresh(ctx: &mut LowerCtx<'_>) -> Value {
    let v = Value(ctx.next);
    ctx.next += 1;
    v
}
