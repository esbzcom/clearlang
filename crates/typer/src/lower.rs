use crate::check::FnSig as CheckFnSig;
use crate::guards::guard_kind_for_callee;
use anyhow::Result;
use clg_ast::{BinOp, Expr, Func, Type};
use clg_ir::{BinOpIR, Function as IrFunction, GuardKind, Instr, IrType, TrapCode, Value};
use std::collections::HashMap;

type FnSig<'a> = CheckFnSig<'a>;

fn ir_ty(t: Type) -> IrType {
    match t {
        Type::Int => IrType::Int,
        Type::Bool => IrType::Bool,
        Type::String => IrType::Int, // placeholder until strings have a runtime representation
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
        Expr::Match { .. } => {
            anyhow::bail!("match expression not supported in lowering yet")
        }
        Expr::Try { span, .. } => {
            anyhow::bail!(
                "`?` is not supported in codegen yet ({}..{})",
                span.start,
                span.end
            )
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
        Expr::Call { callee, args, .. } => {
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
            // If callee is known (user or intrinsic), emit a Call with its index
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
}
fn fresh(ctx: &mut LowerCtx<'_>) -> Value {
    let v = Value(ctx.next);
    ctx.next += 1;
    v
}
