use anyhow::Result;
use std::collections::HashMap;
use lumi_ast::{BinOp, Expr, Func, Type};
use lumi_ir::{BinOpIR, Function as IrFunction, Instr, IrType, Value};

type FnSig<'a> = (&'a [lumi_ast::Param], Type);

fn ir_ty(t: Type) -> IrType {
    match t {
        Type::Int => IrType::Int,
        Type::Bool => IrType::Bool,
        Type::String => IrType::Int, // placeholder until strings have a runtime representation
        Type::Option(_) => IrType::Int,
        Type::Result(_, _) => IrType::Int,
    }
}

pub(crate) struct LowerCtx<'a> {
    pub next: u32,
    pub env: HashMap<&'a str, Value>,
    pub fns: HashMap<&'a str, FnSig<'a>>, // for call return types
    pub body: Vec<Instr>,
}

pub(crate) fn lower_func<'a>(f: &'a Func, fns: &HashMap<&'a str, FnSig<'a>>) -> Result<IrFunction> {
    let mut env: HashMap<&str, Value> = HashMap::new();
    for (i, p) in f.params.iter().enumerate() {
        env.insert(p.name.as_str(), Value(i as u32));
    }
    let mut ctx = LowerCtx {
        next: f.params.len() as u32,
        env,
        fns: fns.clone(),
        body: Vec::new(),
    };

    let ret_val = lower_expr(&mut ctx, &f.body)?;
    ctx.body.push(Instr::Ret { val: ret_val });

    Ok(IrFunction {
        name: f.name.clone(),
        params: f.params.iter().map(|p| ir_ty(p.ty)).collect(),
        ret: Some(ir_ty(f.ret)),
        body: ctx.body,
    })
}

fn lower_expr<'a>(ctx: &mut LowerCtx<'a>, e: &'a Expr) -> Result<Value> {
    match e {
        Expr::Int(n, _) => {
            let dst = fresh(ctx);
            ctx.body.push(Instr::IConst { dst, ty: IrType::Int, n: *n });
            Ok(dst)
        }
        Expr::Bool(b, _) => {
            let dst = fresh(ctx);
            ctx.body.push(Instr::IConst { dst, ty: IrType::Bool, n: if *b { 1 } else { 0 } });
            Ok(dst)
        }
        Expr::String(_, _) => {
            let dst = fresh(ctx);
            ctx.body.push(Instr::IConst { dst, ty: IrType::Int, n: 0 });
            Ok(dst)
        }
        Expr::Match { .. } => {
            anyhow::bail!("match expression not supported in lowering yet")
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
            };
            ctx.body.push(Instr::IBin { dst, op: irop, lhs: lv, rhs: rv });
            Ok(dst)
        }
        Expr::Call { callee, args, .. } => {
            let argv: Result<Vec<_>> = args.iter().map(|a| lower_expr(ctx, a)).collect();
            let argv = argv?;
            let (_params, _ret_ty) = ctx
                .fns
                .get(callee.as_str())
                .copied()
                .ok_or_else(|| anyhow::anyhow!(format!("unknown function `{}`", callee)))?;
            let dst = fresh(ctx);
            ctx.body.push(Instr::Call { dst: Some(dst), callee: callee.clone(), args: argv });
            Ok(dst)
        }
    }
}

fn fresh(ctx: &mut LowerCtx<'_>) -> Value {
    let v = Value(ctx.next);
    ctx.next += 1;
    v
}
