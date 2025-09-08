use anyhow::{Context, Result};
use lumi_ast::{BinOp, Effect, Expr, Func, Param, Program, Type, Span};
use lumi_ir::{BinOpIR, Function as IrFunction, Instr, IrType, Module, Value};
use std::collections::HashMap;

type FnSig<'a> = (&'a [Param], Type);

// ---------------- Structured Typer Errors ----------------

#[derive(Debug, Clone)]
pub struct TyperError {
    pub code: &'static str,
    pub message: String,
    pub start: usize,
    pub end: usize,
}

impl TyperError {
    fn new(code: &'static str, message: String, start: usize, end: usize) -> Self {
        TyperError { code, message, start, end }
    }

    fn unknown_function(callee: &str, span: Span) -> Self {
        Self::new(
            "T001",
            format!("at {}..{}: unknown function `{}`", span.start, span.end, callee),
            span.start,
            span.end,
        )
    }

    fn arity_mismatch(callee: &str, expected: usize, found: usize, span: Span) -> Self {
        Self::new(
            "T002",
            format!(
                "at {}..{}: arity mismatch calling `{}`: expected {}, found {}",
                span.start, span.end, callee, expected, found
            ),
            span.start,
            span.end,
        )
    }

    fn arg_type_mismatch(i: usize, callee: &str, expected: Type, found: Type, span: Span) -> Self {
        Self::new(
            "T003",
            format!(
                "at {}..{}: arg {} type mismatch calling `{}`: expected `{}`, found `{}`",
                span.start, span.end, i, callee, show_ty(expected), show_ty(found)
            ),
            span.start,
            span.end,
        )
    }

    fn return_type_mismatch(declared: Type, found: Type, span: Span) -> Self {
        Self::new(
            "T004",
            format!(
                "at {}..{}: return type mismatch: declared `{}`, found `{}`",
                span.start, span.end, show_ty(declared), show_ty(found)
            ),
            span.start,
            span.end,
        )
    }

    fn int_operand(what: &str, ty: Type, span: Option<Span>) -> Self {
        if let Some(sp) = span {
            Self::new(
                "T005",
                format!(
                    "at {}..{}: {} must be Int, found `{}`",
                    sp.start, sp.end, what, show_ty(ty)
                ),
                sp.start,
                sp.end,
            )
        } else {
            Self::new(
                "T005",
                format!("{} must be Int, found `{}`", what, show_ty(ty)),
                0,
                0,
            )
        }
    }

    fn unknown_variable(name: &str, sp: Span) -> Self {
        Self::new(
            "T006",
            format!("at {}..{}: unknown variable `{}`", sp.start, sp.end, name),
            sp.start,
            sp.end,
        )
    }

    fn duplicate_function(name: &str) -> Self {
        Self::new("T008", format!("duplicate function `{}`", name), 0, 0)
    }

    fn duplicate_parameter(name: &str) -> Self {
        Self::new("T010", format!("duplicate parameter `{}`", name), 0, 0)
    }

    fn effect_not_supported(effect: Effect) -> Self {
        let eff_str = match effect { Effect::Mut => "mut", Effect::Io => "io", _ => "" };
        Self::new(
            "T009",
            format!("effect `{}` not supported yet; use `pure` or omit", eff_str),
            0,
            0,
        )
    }
}

impl std::fmt::Display for TyperError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for TyperError {}

/// Type-check the Lumi AST and return a lowered IR module on success.
/// Phase 3.1–3.4: checks Int/Bool, variables, calls, binops, arity, and returns,
/// then lowers AST → IR using a simple SSA-like scheme.
pub fn check(ast: &Program) -> Result<Module> {
    let mut fns: HashMap<&str, FnSig> = HashMap::new();

    // Phase 4.2 — Built-in type stubs (namespaced): std::str::{len, concat, eq}
    // Keep the owning storage alive for the duration of this function
    let mut builtin_sigs: Vec<(String, Vec<Param>, Type)> = Vec::new();
    builtin_sigs.push((
        "std::str::len".to_string(),
        vec![Param { name: "s".to_string(), ty: Type::String }],
        Type::Int,
    ));
    builtin_sigs.push((
        "std::str::concat".to_string(),
        vec![
            Param { name: "a".to_string(), ty: Type::String },
            Param { name: "b".to_string(), ty: Type::String },
        ],
        Type::String,
    ));
    builtin_sigs.push((
        "std::str::eq".to_string(),
        vec![
            Param { name: "a".to_string(), ty: Type::String },
            Param { name: "b".to_string(), ty: Type::String },
        ],
        Type::Bool,
    ));
    for (name, params, ret) in &builtin_sigs {
        fns.insert(name.as_str(), (&params[..], *ret));
    }

    for f in &ast.funcs {
        if fns.insert(f.name.as_str(), (&f.params, f.ret)).is_some() {
            return Err(TyperError::duplicate_function(&f.name).into());
        }
    }

    for f in &ast.funcs {
        check_func(f, &fns).with_context(|| format!("in function `{}`", f.name))?;
    }

    // Lowering (Phase 3.4): convert each checked function into IR
    let mut module = Module::default();
    for f in &ast.funcs {
        module.funcs.push(lower_func(f, &fns)?);
    }
    Ok(module)
}

fn check_func<'a>(f: &'a Func, fns: &HashMap<&'a str, FnSig<'a>>) -> Result<()> {
    // Effects stub (Phase 3.2): accept None|Pure; reject Mut/Io for now
    match f.effect {
        Effect::None | Effect::Pure => {}
        Effect::Mut | Effect::Io => return Err(TyperError::effect_not_supported(f.effect).into()),
    }
    let mut env: HashMap<&str, Type> = HashMap::new();
    for p in &f.params {
        if env.insert(p.name.as_str(), p.ty).is_some() {
            return Err(TyperError::duplicate_parameter(&p.name).into());
        }
    }

    let body_ty = type_of(&f.body, &env, fns, 0)?;
    if body_ty != f.ret {
        // Try to use the body's span to annotate the mismatch
        let sp = match &f.body {
            Expr::Int(_, sp) | Expr::Bool(_, sp) | Expr::String(_, sp) | Expr::Var(_, sp) => *sp,
            Expr::Bin { span, .. } | Expr::Call { span, .. } => *span,
        };
        return Err(TyperError::return_type_mismatch(f.ret, body_ty, sp).into());
    }
    Ok(())
}

fn type_of<'a>(
    e: &'a Expr,
    env: &HashMap<&'a str, Type>,
    fns: &HashMap<&'a str, FnSig<'a>>,
    depth: usize,
) -> Result<Type> {
    if depth > 1024 {
        return Err(TyperError::new("T011", "type-check recursion limit exceeded".to_string(), 0, 0).into());
    }
    match e {
        Expr::Int(_, _) => Ok(Type::Int),
        Expr::Bool(_, _) => Ok(Type::Bool),
        Expr::String(_, _) => Ok(Type::String),
        Expr::Var(name, sp) => env
            .get(name.as_str())
            .copied()
            .ok_or_else(|| TyperError::unknown_variable(name, *sp).into()),
        Expr::Bin { op, lhs, rhs, span } => {
            let lt = type_of(lhs, env, fns, depth + 1)?;
            let rt = type_of(rhs, env, fns, depth + 1)?;
            ensure_int(lt, "left operand", Some(*span))?;
            ensure_int(rt, "right operand", Some(*span))?;
            match op {
                BinOp::Add | BinOp::Sub | BinOp::Mul | BinOp::Div => Ok(Type::Int),
            }
        }
        Expr::Call { callee, args, span } => {
            let (params, ret) = fns
                .get(callee.as_str())
                .copied()
                .ok_or_else(|| TyperError::unknown_function(callee, *span).into())?;
            if params.len() != args.len() {
                return Err(TyperError::arity_mismatch(callee, params.len(), args.len(), *span).into());
            }
            for (i, (p, a)) in params.iter().zip(args.iter()).enumerate() {
                let at = type_of(a, env, fns, depth + 1)?;
                if p.ty != at {
                    let sp = match a { Expr::Int(_, sp)|Expr::Bool(_, sp)|Expr::String(_, sp)|Expr::Var(_, sp)|Expr::Bin{ span: sp, .. }|Expr::Call{ span: sp, .. } => *sp };
                    return Err(TyperError::arg_type_mismatch(i, callee, p.ty, at, sp).into());
                }
            }
            Ok(ret)
        }
    }
}

fn ensure_int(ty: Type, what: &str, span: Option<Span>) -> Result<()> {
    if ty != Type::Int {
        return Err(TyperError::int_operand(what, ty, span).into());
    }
    Ok(())
}

fn show_ty(t: Type) -> &'static str {
    match t {
        Type::Int => "Int",
        Type::Bool => "Bool",
        Type::String => "String",
    }
}

// ---------------- Lowering (Phase 3.4) ----------------

fn ir_ty(t: Type) -> IrType {
    match t {
        Type::Int => IrType::Int,
        Type::Bool => IrType::Bool,
        Type::String => IrType::Int, // placeholder until strings have a runtime representation
    }
}

struct LowerCtx<'a> {
    next: u32,
    env: HashMap<&'a str, Value>,
    fns: HashMap<&'a str, FnSig<'a>>, // for call return types
    body: Vec<Instr>,
}

fn lower_func<'a>(f: &'a Func, fns: &HashMap<&'a str, FnSig<'a>>) -> Result<IrFunction> {
    // Reserve SSA ids for parameters in order
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
            // Placeholder: represent strings as 0 until runtime is implemented (Phase 5)
            let dst = fresh(ctx);
            ctx.body.push(Instr::IConst { dst, ty: IrType::Int, n: 0 });
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
            };
            ctx.body.push(Instr::IBin { dst, op: irop, lhs: lv, rhs: rv });
            Ok(dst)
        }
        Expr::Call { callee, args, .. } => {
            let argv: Result<Vec<_>> = args.iter().map(|a| lower_expr(ctx, a)).collect();
            let argv = argv?;
            // Decide whether the call yields a value based on callee's return type
            let (_params, _ret_ty) = ctx
                .fns
                .get(callee.as_str())
                .copied()
                .ok_or_else(|| anyhow::anyhow!(format!("unknown function `{}`", callee)))?;
            let dst = fresh(ctx);
            // Current language always returns a value; keep Some(dst)
            let dst_opt = Some(dst);
            ctx.body.push(Instr::Call { dst: dst_opt, callee: callee.clone(), args: argv });
            Ok(dst)
        }
    }
}

fn fresh(ctx: &mut LowerCtx<'_>) -> Value {
    let v = Value(ctx.next);
    ctx.next += 1;
    v
}
