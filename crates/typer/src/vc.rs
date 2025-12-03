use crate::guards::{collect_mut_calls, guard_callee_for_kind, MutCall};
use clg_ast::{BinOp, Effect, Expr, MatchArm, MatchPat, Program, Span, Stmt, Type, UnaryOp};
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone)]
pub struct ContractExpr {
    pub ast: String,
    pub smt2: String,
    pub span: Option<Span>,
}

#[derive(Debug, Clone)]
pub struct VerificationCondition {
    pub function: String,
    pub vc_id: String,
    pub pre: ContractExpr,
    pub post: ContractExpr,
    pub vc_smt2: String,
    pub status: &'static str,
}

#[derive(Debug)]
struct LoopObligation<'a> {
    invariant: &'a Expr,
    variant: Option<&'a Expr>,
}

pub fn generate_vcs(program: &Program) -> Vec<VerificationCondition> {
    let alias_map = build_alias_map(program);
    let fn_aliases = build_fn_aliases(program, &alias_map);
    let mut out = Vec::new();
    for func in &program.funcs {
        let req_exprs: Vec<Expr> = func.requires.iter().map(|c| c.expr.clone()).collect();
        let mut pre_extra: Vec<Expr> = Vec::new();
        for param in &func.params {
            if let Some(name) = match_alias_name(&param.ty) {
                if let Some(alias) = alias_map.get(name) {
                    if let Some(pred) = instantiate_alias_predicate(
                        alias,
                        &Expr::Var(param.name.clone(), alias.span),
                    ) {
                        pre_extra.push(pred);
                    }
                }
            }
        }

        let mut ensures: Vec<(Expr, Span)> = func
            .ensures
            .iter()
            .map(|e| (e.expr.clone(), e.span))
            .collect();
        if let Some(name) = match_alias_name(&func.ret) {
            if let Some(alias) = alias_map.get(name) {
                if let Some(pred) =
                    instantiate_alias_predicate(alias, &Expr::Var("result".into(), alias.span))
                {
                    ensures.push((pred, alias.span));
                }
            }
        }

        // Obligations arising from alias-typed call arguments inside the body.
        let mut body_obligations: Vec<Expr> = Vec::new();
        collect_refinement_obligations(&func.body, &fn_aliases, &mut body_obligations);

        let mut all_pre = req_exprs.clone();
        all_pre.extend(pre_extra.into_iter());
        all_pre.extend(body_obligations.into_iter());
        let pre_expr = if all_pre.is_empty() {
            Expr::Bool(true, Span { start: 0, end: 0 })
        } else {
            fold_conjunction(all_pre)
        };
        let pre_span = if func.requires.is_empty() {
            None
        } else {
            let start = func.requires.first().unwrap().span.start;
            let end = func.requires.last().unwrap().span.end;
            Some(Span { start, end })
        };
        let pre_ast = expr_to_source(&pre_expr, 0);

        if matches!(func.effect, Effect::None | Effect::Pure) && !ensures.is_empty() {
            for (idx, (ensure_expr, ensure_span)) in ensures.iter().enumerate() {
                let post_ast = expr_to_source(ensure_expr, 0);
                let substituted = substitute_result(ensure_expr, &func.body);

                let mut encoder = SmtEncoder::default();
                let pre_smt = encoder.encode(&pre_expr);
                let post_smt = encoder.encode(ensure_expr);
                let substituted_smt = encoder.encode(&substituted);
                let vc_body = format!("(=> {} {})", pre_smt, substituted_smt);
                let vc_smt2 = encoder.wrap_vc(&vc_body);

                out.push(VerificationCondition {
                    function: func.name.clone(),
                    vc_id: format!("vc:{}", idx),
                    pre: ContractExpr {
                        ast: pre_ast.clone(),
                        smt2: pre_smt.clone(),
                        span: pre_span,
                    },
                    post: ContractExpr {
                        ast: post_ast,
                        smt2: post_smt,
                        span: Some(*ensure_span),
                    },
                    vc_smt2,
                    status: "generated",
                });
            }
        }

        let mut mut_calls: Vec<MutCall> = Vec::new();
        collect_mut_calls(&func.body, &mut mut_calls);
        if !mut_calls.is_empty() {
            for (idx, call) in mut_calls.into_iter().enumerate() {
                let Some(arg_name) = call.target else {
                    continue;
                };
                let guard_span = call.span;
                let guard_expr = Expr::Call {
                    callee: guard_callee_for_kind(call.kind).to_string(),
                    args: vec![Expr::Var(arg_name.clone(), guard_span)],
                    span: guard_span,
                };
                let post_ast = expr_to_source(&guard_expr, 0);

                let mut encoder = SmtEncoder::default();
                let pre_smt = encoder.encode(&pre_expr);
                let post_smt = encoder.encode(&guard_expr);
                let vc_body = format!("(=> {} {})", pre_smt, post_smt);
                let vc_smt2 = encoder.wrap_vc(&vc_body);

                out.push(VerificationCondition {
                    function: func.name.clone(),
                    vc_id: format!("mut_pre:{}:{}", call.callee, idx),
                    pre: ContractExpr {
                        ast: pre_ast.clone(),
                        smt2: pre_smt.clone(),
                        span: pre_span,
                    },
                    post: ContractExpr {
                        ast: post_ast,
                        smt2: post_smt,
                        span: Some(guard_span),
                    },
                    vc_smt2,
                    status: "generated",
                });
            }
        }

        let mut loops: Vec<LoopObligation<'_>> = Vec::new();
        collect_loops(&func.body, &mut loops);
        for (idx, loop_ob) in loops.iter().enumerate() {
            let mut enc_inv = SmtEncoder::default();
            let pre_smt = enc_inv.encode(&pre_expr);
            let inv_smt = enc_inv.encode(loop_ob.invariant);
            let vc_body = format!("(=> {} {})", pre_smt, inv_smt);
            let vc_smt2 = enc_inv.wrap_vc(&vc_body);

            out.push(VerificationCondition {
                function: func.name.clone(),
                vc_id: format!("loop:{}:invariant", idx),
                pre: ContractExpr {
                    ast: pre_ast.clone(),
                    smt2: pre_smt.clone(),
                    span: pre_span,
                },
                post: ContractExpr {
                    ast: expr_to_source(loop_ob.invariant, 0),
                    smt2: inv_smt,
                    span: Some(expr_span(loop_ob.invariant)),
                },
                vc_smt2,
                status: "generated",
            });

            if let Some(var_expr) = loop_ob.variant {
                let mut enc_var = SmtEncoder::default();
                let pre_smt = enc_var.encode(&pre_expr);
                let var_smt = enc_var.encode(var_expr);
                let post_smt = format!("(>= {} 0)", var_smt);
                let vc_body = format!("(=> {} {})", pre_smt, post_smt);
                let vc_smt2 = enc_var.wrap_vc(&vc_body);
                out.push(VerificationCondition {
                    function: func.name.clone(),
                    vc_id: format!("loop:{}:variant_nonneg", idx),
                    pre: ContractExpr {
                        ast: pre_ast.clone(),
                        smt2: pre_smt.clone(),
                        span: pre_span,
                    },
                    post: ContractExpr {
                        ast: format!("{} >= 0", expr_to_source(var_expr, 0)),
                        smt2: post_smt,
                        span: Some(expr_span(var_expr)),
                    },
                    vc_smt2,
                    status: "generated",
                });

                let mut enc_dec = SmtEncoder::default();
                let pre_smt = enc_dec.encode(&pre_expr);
                let var_before = enc_dec.encode(var_expr);
                let next_sym = format!("cl.loop.variant.next.{}", idx);
                let post_smt = format!("(< {} {})", next_sym, var_before);
                let vc_body = format!("(=> {} {})", pre_smt, post_smt);
                let extra = format!("(declare-const {} Int)", next_sym);
                let vc_smt2 = enc_dec.wrap_vc_with_extra(&vc_body, Some(&extra));
                out.push(VerificationCondition {
                    function: func.name.clone(),
                    vc_id: format!("loop:{}:variant_decrease", idx),
                    pre: ContractExpr {
                        ast: pre_ast.clone(),
                        smt2: pre_smt.clone(),
                        span: pre_span,
                    },
                    post: ContractExpr {
                        ast: format!(
                            "{}' < {}",
                            expr_to_source(var_expr, 0),
                            expr_to_source(var_expr, 0)
                        ),
                        smt2: post_smt,
                        span: Some(expr_span(var_expr)),
                    },
                    vc_smt2,
                    status: "generated",
                });
            }
        }
    }
    out.sort_by(|a, b| a.function.cmp(&b.function).then(a.vc_id.cmp(&b.vc_id)));
    out
}

fn match_alias_name(ty: &clg_ast::Type) -> Option<&str> {
    if let clg_ast::Type::Resource(name) = ty {
        Some(name.as_str())
    } else {
        None
    }
}

#[derive(Clone)]
struct AliasView<'a> {
    binder: Option<&'a str>,
    predicate: &'a Expr,
    span: Span,
}

fn build_alias_map<'a>(program: &'a Program) -> std::collections::HashMap<&'a str, AliasView<'a>> {
    program
        .refined_aliases
        .iter()
        .map(|a| {
            (
                a.name.as_str(),
                AliasView {
                    binder: a.binder.as_deref(),
                    predicate: &a.predicate,
                    span: a.span,
                },
            )
        })
        .collect()
}

fn instantiate_alias_predicate(alias: &AliasView<'_>, replacement: &Expr) -> Option<Expr> {
    let binder = alias.binder?;
    Some(substitute_binder(alias.predicate, binder, replacement))
}

fn build_fn_aliases<'a>(
    program: &'a Program,
    aliases: &'a HashMap<&'a str, AliasView<'a>>,
) -> HashMap<&'a str, Vec<Option<&'a AliasView<'a>>>> {
    let mut map: HashMap<&'a str, Vec<Option<&'a AliasView<'a>>>> = HashMap::new();
    for func in &program.funcs {
        let mut param_aliases = Vec::new();
        for param in &func.params {
            if let Type::Resource(name) = &param.ty {
                param_aliases.push(aliases.get(name.as_str()));
            } else {
                param_aliases.push(None);
            }
        }
        map.insert(func.name.as_str(), param_aliases);
    }
    map
}

fn collect_refinement_obligations<'a>(
    expr: &'a Expr,
    fn_aliases: &HashMap<&'a str, Vec<Option<&'a AliasView<'a>>>>,
    out: &mut Vec<Expr>,
) {
    match expr {
        Expr::Block { block } => {
            for stmt in &block.statements {
                match stmt {
                    Stmt::Let { expr, .. } | Stmt::Expr { expr, .. } => {
                        collect_refinement_obligations(expr.as_ref(), fn_aliases, out)
                    }
                    Stmt::While {
                        cond,
                        invariant,
                        variant,
                        body,
                        ..
                    } => {
                        collect_refinement_obligations(cond.as_ref(), fn_aliases, out);
                        collect_refinement_obligations(invariant.as_ref(), fn_aliases, out);
                        if let Some(v) = variant {
                            collect_refinement_obligations(v.as_ref(), fn_aliases, out);
                        }
                        collect_refinement_obligations(
                            &Expr::Block {
                                block: body.clone(),
                            },
                            fn_aliases,
                            out,
                        );
                    }
                }
            }
            if let Some(tail) = &block.tail {
                collect_refinement_obligations(tail.as_ref(), fn_aliases, out);
            }
        }
        Expr::If {
            cond,
            then_br,
            else_br,
            ..
        } => {
            collect_refinement_obligations(cond.as_ref(), fn_aliases, out);
            collect_refinement_obligations(then_br.as_ref(), fn_aliases, out);
            collect_refinement_obligations(else_br.as_ref(), fn_aliases, out);
        }
        Expr::Match {
            scrutinee, arms, ..
        } => {
            collect_refinement_obligations(scrutinee.as_ref(), fn_aliases, out);
            for arm in arms {
                collect_refinement_obligations(&arm.expr, fn_aliases, out);
            }
        }
        Expr::Bin { lhs, rhs, .. } => {
            collect_refinement_obligations(lhs.as_ref(), fn_aliases, out);
            collect_refinement_obligations(rhs.as_ref(), fn_aliases, out);
        }
        Expr::Unary { expr, .. } | Expr::Return { expr, .. } | Expr::Try { expr, .. } => {
            collect_refinement_obligations(expr.as_ref(), fn_aliases, out);
        }
        Expr::Call { callee, args, .. } => {
            if let Some(param_aliases) = fn_aliases.get(callee.as_str()) {
                for (arg, alias_opt) in args.iter().zip(param_aliases.iter()) {
                    if let Some(alias) = alias_opt {
                        if let Some(pred) = instantiate_alias_predicate(alias, arg) {
                            out.push(pred);
                        }
                    }
                    collect_refinement_obligations(arg, fn_aliases, out);
                }
            } else {
                for arg in args {
                    collect_refinement_obligations(arg, fn_aliases, out);
                }
            }
        }
        Expr::Int(_, _) | Expr::Bool(_, _) | Expr::String(_, _) | Expr::Var(_, _) => {}
    }
}

fn substitute_binder(expr: &Expr, binder: &str, replacement: &Expr) -> Expr {
    match expr {
        Expr::Var(name, _) if name == binder => replacement.clone(),
        Expr::Var(_, _) | Expr::Int(_, _) | Expr::Bool(_, _) | Expr::String(_, _) => expr.clone(),
        Expr::Bin { op, lhs, rhs, span } => Expr::Bin {
            op: *op,
            lhs: Box::new(substitute_binder(lhs, binder, replacement)),
            rhs: Box::new(substitute_binder(rhs, binder, replacement)),
            span: *span,
        },
        Expr::Unary { op, expr, span } => Expr::Unary {
            op: *op,
            expr: Box::new(substitute_binder(expr, binder, replacement)),
            span: *span,
        },
        Expr::Call { callee, args, span } => Expr::Call {
            callee: callee.clone(),
            args: args
                .iter()
                .map(|a| substitute_binder(a, binder, replacement))
                .collect(),
            span: *span,
        },
        Expr::Return { expr, span } => Expr::Return {
            expr: Box::new(substitute_binder(expr, binder, replacement)),
            span: *span,
        },
        Expr::Try { expr, span } => Expr::Try {
            expr: Box::new(substitute_binder(expr, binder, replacement)),
            span: *span,
        },
        Expr::Block { block } => Expr::Block {
            block: Box::new(substitute_block_binder(block, binder, replacement)),
        },
        Expr::Match { .. } | Expr::If { .. } => expr.clone(),
    }
}

fn substitute_block_binder(
    block: &clg_ast::Block,
    binder: &str,
    replacement: &Expr,
) -> clg_ast::Block {
    let mut new_block = block.clone();
    new_block.statements = block
        .statements
        .iter()
        .map(|stmt| match stmt {
            Stmt::Let { name, expr, span } => Stmt::Let {
                name: name.clone(),
                expr: Box::new(substitute_binder(expr, binder, replacement)),
                span: *span,
            },
            Stmt::Expr { expr, span } => Stmt::Expr {
                expr: Box::new(substitute_binder(expr, binder, replacement)),
                span: *span,
            },
            Stmt::While {
                cond,
                invariant,
                variant,
                body,
                span,
            } => Stmt::While {
                cond: Box::new(substitute_binder(cond, binder, replacement)),
                invariant: Box::new(substitute_binder(invariant, binder, replacement)),
                variant: variant
                    .as_ref()
                    .map(|v| Box::new(substitute_binder(v, binder, replacement))),
                body: Box::new(substitute_block_binder(body, binder, replacement)),
                span: *span,
            },
        })
        .collect();
    new_block.tail = block
        .tail
        .as_ref()
        .map(|expr| Box::new(substitute_binder(expr, binder, replacement)));
    new_block
}

fn fold_conjunction(mut exprs: Vec<Expr>) -> Expr {
    let mut iter = exprs.drain(..);
    let first = iter.next().expect("exprs not empty");
    iter.fold(first, |lhs, rhs| {
        let span = merge_span(&lhs, &rhs);
        Expr::Bin {
            op: BinOp::And,
            lhs: Box::new(lhs),
            rhs: Box::new(rhs),
            span,
        }
    })
}

fn substitute_result(expr: &Expr, replacement: &Expr) -> Expr {
    match expr {
        Expr::Var(name, _) if name == "result" => replacement.clone(),
        Expr::Var(_, _) | Expr::Int(_, _) | Expr::Bool(_, _) | Expr::String(_, _) => expr.clone(),
        Expr::Bin { op, lhs, rhs, span } => Expr::Bin {
            op: *op,
            lhs: Box::new(substitute_result(lhs, replacement)),
            rhs: Box::new(substitute_result(rhs, replacement)),
            span: *span,
        },
        Expr::Unary { op, expr, span } => Expr::Unary {
            op: *op,
            expr: Box::new(substitute_result(expr, replacement)),
            span: *span,
        },
        Expr::Call { callee, args, span } => Expr::Call {
            callee: callee.clone(),
            args: args
                .iter()
                .map(|a| substitute_result(a, replacement))
                .collect(),
            span: *span,
        },
        Expr::Return { expr, span } => Expr::Return {
            expr: Box::new(substitute_result(expr, replacement)),
            span: *span,
        },
        Expr::Try { expr, span } => Expr::Try {
            expr: Box::new(substitute_result(expr, replacement)),
            span: *span,
        },
        Expr::Block { block } => Expr::Block {
            block: Box::new(substitute_block(block, replacement)),
        },
        Expr::Match { .. } | Expr::If { .. } => expr.clone(),
    }
}

fn substitute_block(block: &clg_ast::Block, replacement: &Expr) -> clg_ast::Block {
    let mut new_block = block.clone();
    new_block.statements = block
        .statements
        .iter()
        .map(|stmt| match stmt {
            Stmt::Let { name, expr, span } => Stmt::Let {
                name: name.clone(),
                expr: Box::new(substitute_result(expr, replacement)),
                span: *span,
            },
            Stmt::Expr { expr, span } => Stmt::Expr {
                expr: Box::new(substitute_result(expr, replacement)),
                span: *span,
            },
            Stmt::While {
                cond,
                invariant,
                variant,
                body,
                span,
            } => Stmt::While {
                cond: Box::new(substitute_result(cond, replacement)),
                invariant: Box::new(substitute_result(invariant, replacement)),
                variant: variant
                    .as_ref()
                    .map(|v| Box::new(substitute_result(v, replacement))),
                body: Box::new(substitute_block(body, replacement)),
                span: *span,
            },
        })
        .collect();
    new_block.tail = block
        .tail
        .as_ref()
        .map(|expr| Box::new(substitute_result(expr, replacement)));
    new_block
}
fn expr_to_source(expr: &Expr, parent_prec: u8) -> String {
    match expr {
        Expr::Int(n, _) => n.to_string(),
        Expr::Bool(b, _) => b.to_string(),
        Expr::String(s, _) => format!("\"{}\"", s),
        Expr::Var(name, _) => name.clone(),
        Expr::Unary { op, expr, .. } => match op {
            UnaryOp::Not => {
                let inner = expr_to_source(expr, precedence_unary());
                format!("!{}", inner)
            }
        },
        Expr::Bin { op, lhs, rhs, .. } => {
            let prec = precedence_bin(*op);
            let left = expr_to_source(lhs, prec);
            let right = expr_to_source(rhs, prec + 1);
            let infix = match op {
                BinOp::Add => "+",
                BinOp::Sub => "-",
                BinOp::Mul => "*",
                BinOp::Div => "/",
                BinOp::Lt => "<",
                BinOp::Le => "<=",
                BinOp::Gt => ">",
                BinOp::Ge => ">=",
                BinOp::Eq => "==",
                BinOp::Neq => "!=",
                BinOp::And => "&&",
                BinOp::Or => "||",
            };
            let s = format!("{} {} {}", left, infix, right);
            if prec < parent_prec {
                format!("({})", s)
            } else {
                s
            }
        }
        Expr::Call { callee, args, .. } => {
            let params: Vec<String> = args.iter().map(|a| expr_to_source(a, 0)).collect();
            format!("{}({})", callee, params.join(", "))
        }
        Expr::Return { expr, .. } => format!("return {}", expr_to_source(expr, 0)),
        Expr::Try { expr, .. } => {
            let inner = expr_to_source(expr, precedence_unary());
            let rendered = format!("{}?", inner);
            if precedence_unary() < parent_prec {
                format!("({})", rendered)
            } else {
                rendered
            }
        }
        Expr::Match {
            scrutinee, arms, ..
        } => {
            if arms.len() == 2 {
                let scrutinee_src = expr_to_source(scrutinee, 0);
                let first = match_arm_to_source(&arms[0]);
                let second = match_arm_to_source(&arms[1]);
                format!(
                    "match {} {{ {} => {}, {} => {} }}",
                    scrutinee_src, first.pat, first.body, second.pat, second.body
                )
            } else {
                format!("{:?}", expr)
            }
        }
        Expr::If {
            cond,
            then_br,
            else_br,
            ..
        } => {
            let cond_src = expr_to_source(cond, 0);
            let then_src = expr_to_source(then_br, 0);
            let else_src = expr_to_source(else_br, 0);
            format!("if {} {{ {} }} else {{ {} }}", cond_src, then_src, else_src)
        }
        Expr::Block { block } => {
            let mut parts: Vec<String> = Vec::new();
            for stmt in &block.statements {
                let rendered = match stmt {
                    Stmt::Let { name, expr, .. } => {
                        format!("let {} = {};", name, expr_to_source(expr.as_ref(), 0))
                    }
                    Stmt::Expr { expr, .. } => {
                        format!("{};", expr_to_source(expr.as_ref(), 0))
                    }
                    Stmt::While {
                        cond,
                        invariant,
                        variant,
                        body,
                        ..
                    } => {
                        let cond_src = expr_to_source(cond.as_ref(), 0);
                        let inv_src = expr_to_source(invariant.as_ref(), 0);
                        let var_src = variant
                            .as_ref()
                            .map(|v| format!(" variant {{ {} }}", expr_to_source(v.as_ref(), 0)))
                            .unwrap_or_default();
                        let body_src = expr_to_source(
                            &Expr::Block {
                                block: body.clone(),
                            },
                            0,
                        );
                        format!(
                            "while {} invariant {{ {} }}{} {}",
                            cond_src, inv_src, var_src, body_src
                        )
                    }
                };
                parts.push(rendered);
            }
            if let Some(tail) = &block.tail {
                parts.push(expr_to_source(tail.as_ref(), 0));
            }
            format!("{{ {} }}", parts.join(" "))
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum SmtHelper {
    VariantAccessors,
    OptionCtor,
    ResultCtor,
}

#[derive(Default)]
struct SmtEncoder {
    helpers: HashSet<SmtHelper>,
    fresh: usize,
}

impl SmtEncoder {
    fn encode(&mut self, expr: &Expr) -> String {
        self.encode_inner(expr)
    }

    fn encode_inner(&mut self, expr: &Expr) -> String {
        match expr {
            Expr::Int(n, _) => n.to_string(),
            Expr::Bool(b, _) => b.to_string(),
            Expr::String(s, _) => format!("\"{}\"", s),
            Expr::Var(name, _) => name.clone(),
            Expr::Unary { op, expr, .. } => match op {
                UnaryOp::Not => format!("(not {})", self.encode_inner(expr)),
            },
            Expr::Bin { op, lhs, rhs, .. } => match op {
                BinOp::Add => format!("(+ {} {})", self.encode_inner(lhs), self.encode_inner(rhs)),
                BinOp::Sub => format!("(- {} {})", self.encode_inner(lhs), self.encode_inner(rhs)),
                BinOp::Mul => format!("(* {} {})", self.encode_inner(lhs), self.encode_inner(rhs)),
                BinOp::Div => format!(
                    "(div {} {})",
                    self.encode_inner(lhs),
                    self.encode_inner(rhs)
                ),
                BinOp::Lt => format!("(< {} {})", self.encode_inner(lhs), self.encode_inner(rhs)),
                BinOp::Le => format!("(<= {} {})", self.encode_inner(lhs), self.encode_inner(rhs)),
                BinOp::Gt => format!("(> {} {})", self.encode_inner(lhs), self.encode_inner(rhs)),
                BinOp::Ge => format!("(>= {} {})", self.encode_inner(lhs), self.encode_inner(rhs)),
                BinOp::Eq => format!("(= {} {})", self.encode_inner(lhs), self.encode_inner(rhs)),
                BinOp::Neq => format!(
                    "(not (= {} {}))",
                    self.encode_inner(lhs),
                    self.encode_inner(rhs)
                ),
                BinOp::And => format!(
                    "(and {} {})",
                    self.encode_inner(lhs),
                    self.encode_inner(rhs)
                ),
                BinOp::Or => format!("(or {} {})", self.encode_inner(lhs), self.encode_inner(rhs)),
            },
            Expr::Call { callee, args, .. } => self.encode_call(callee.as_str(), args),
            Expr::Return { expr, .. } => self.encode_inner(expr),
            Expr::Try { expr, .. } => self.encode_try(expr),
            Expr::Match {
                scrutinee, arms, ..
            } => self.encode_match(scrutinee, arms),
            Expr::If {
                cond,
                then_br,
                else_br,
                ..
            } => format!(
                "(ite {} {} {})",
                self.encode_inner(cond),
                self.encode_inner(then_br),
                self.encode_inner(else_br)
            ),
            Expr::Block { block } => {
                if let Some(tail) = &block.tail {
                    self.encode_inner(tail.as_ref())
                } else {
                    "0".to_string()
                }
            }
        }
    }

    fn encode_call(&mut self, callee: &str, args: &[Expr]) -> String {
        match callee {
            "Some" => {
                self.helpers.insert(SmtHelper::VariantAccessors);
                self.helpers.insert(SmtHelper::OptionCtor);
                let val = args
                    .first()
                    .map(|a| self.encode_inner(a))
                    .unwrap_or_else(|| "0".to_string());
                format!("(cl.option.mk 1 {} 0)", val)
            }
            "None" => {
                self.helpers.insert(SmtHelper::VariantAccessors);
                self.helpers.insert(SmtHelper::OptionCtor);
                "(cl.option.mk 0 0 0)".to_string()
            }
            "Ok" => {
                self.helpers.insert(SmtHelper::VariantAccessors);
                self.helpers.insert(SmtHelper::ResultCtor);
                let val = args
                    .first()
                    .map(|a| self.encode_inner(a))
                    .unwrap_or_else(|| "0".to_string());
                format!("(cl.result.mk 1 {} 0)", val)
            }
            "Err" => {
                self.helpers.insert(SmtHelper::VariantAccessors);
                self.helpers.insert(SmtHelper::ResultCtor);
                let val = args
                    .first()
                    .map(|a| self.encode_inner(a))
                    .unwrap_or_else(|| "0".to_string());
                format!("(cl.result.mk 0 {} 0)", val)
            }
            _ => {
                let parts: Vec<String> = args.iter().map(|a| self.encode_inner(a)).collect();
                if parts.is_empty() {
                    format!("({})", callee)
                } else {
                    format!("({} {})", callee, parts.join(" "))
                }
            }
        }
    }

    fn encode_try(&mut self, expr: &Expr) -> String {
        self.helpers.insert(SmtHelper::VariantAccessors);
        let inner = self.encode_inner(expr);
        let tmp = self.fresh_sym("cl_try");
        format!(
            "(let (({} {})) (cl.variant.payload_lo {}))",
            tmp, inner, tmp
        )
    }

    fn encode_match(&mut self, scrutinee: &Expr, arms: &[MatchArm]) -> String {
        if arms.len() != 2 {
            return format!("; unsupported match {:?}", arms);
        }
        self.helpers.insert(SmtHelper::VariantAccessors);
        let scrutinee_term = self.encode_inner(scrutinee);
        let tmp = self.fresh_sym("cl_match");

        let cond = self.match_condition(&tmp, &arms[0].pat);
        let then_branch = self.match_branch(&tmp, &arms[0]);
        let else_branch = self.match_branch(&tmp, &arms[1]);

        format!(
            "(let (({} {})) (ite {} {} {}))",
            tmp, scrutinee_term, cond, then_branch, else_branch
        )
    }

    fn match_condition(&mut self, scrutinee_sym: &str, pat: &MatchPat) -> String {
        let tag_value = match pat {
            MatchPat::Some(_) | MatchPat::Ok(_) => 1,
            MatchPat::None | MatchPat::Err(_) => 0,
        };
        format!("(= (cl.variant.tag {}) {})", scrutinee_sym, tag_value)
    }

    fn match_branch(&mut self, scrutinee_sym: &str, arm: &MatchArm) -> String {
        let body = self.encode_inner(&arm.expr);
        match &arm.pat {
            MatchPat::Some(name) | MatchPat::Ok(name) | MatchPat::Err(name) => {
                let payload = format!("(cl.variant.payload_lo {})", scrutinee_sym);
                format!("(let (({} {})) {})", name, payload, body)
            }
            MatchPat::None => body,
        }
    }

    fn fresh_sym(&mut self, prefix: &str) -> String {
        let sym = format!("{}${}", prefix, self.fresh);
        self.fresh += 1;
        sym
    }

    fn wrap_vc(&self, body: &str) -> String {
        self.wrap_vc_with_extra(body, None)
    }

    fn wrap_vc_with_extra(&self, body: &str, extra: Option<&str>) -> String {
        let prelude = self.helpers_prelude();
        let mut parts: Vec<String> = Vec::new();
        if !prelude.is_empty() {
            parts.push(prelude);
        }
        if let Some(extra_block) = extra {
            if !extra_block.is_empty() {
                parts.push(extra_block.to_string());
            }
        }
        parts.push(body.to_string());
        parts.join("\n")
    }

    fn helpers_prelude(&self) -> String {
        if self.helpers.is_empty() {
            return String::new();
        }
        let mut lines = Vec::new();
        if self.helpers.contains(&SmtHelper::VariantAccessors) {
            lines.push("; Option/Result variants use (tag, payload_lo, payload_hi)");
            lines.push("(declare-fun cl.variant.tag (Int) Int)");
            lines.push("(declare-fun cl.variant.payload_lo (Int) Int)");
            lines.push("(declare-fun cl.variant.payload_hi (Int) Int)");
        }
        if self.helpers.contains(&SmtHelper::OptionCtor) {
            lines.push("(declare-fun cl.option.mk (Int Int Int) Int)");
        }
        if self.helpers.contains(&SmtHelper::ResultCtor) {
            lines.push("(declare-fun cl.result.mk (Int Int Int) Int)");
        }
        lines.join(
            "
",
        )
    }
}

struct ArmSource {
    pat: String,
    body: String,
}

fn match_arm_to_source(arm: &MatchArm) -> ArmSource {
    let pat = match &arm.pat {
        MatchPat::Some(name) => format!("Some({})", name),
        MatchPat::None => "None".to_string(),
        MatchPat::Ok(name) => format!("Ok({})", name),
        MatchPat::Err(name) => format!("Err({})", name),
    };
    let body = expr_to_source(&arm.expr, 0);
    ArmSource { pat, body }
}

fn collect_loops<'a>(expr: &'a Expr, out: &mut Vec<LoopObligation<'a>>) {
    match expr {
        Expr::Block { block } => collect_loops_block(block, out),
        Expr::If {
            cond,
            then_br,
            else_br,
            ..
        } => {
            collect_loops(cond, out);
            collect_loops(then_br, out);
            collect_loops(else_br, out);
        }
        Expr::Match {
            scrutinee, arms, ..
        } => {
            collect_loops(scrutinee, out);
            for arm in arms {
                collect_loops(&arm.expr, out);
            }
        }
        Expr::Bin { lhs, rhs, .. } => {
            collect_loops(lhs, out);
            collect_loops(rhs, out);
        }
        Expr::Call { args, .. } => {
            for arg in args {
                collect_loops(arg, out);
            }
        }
        Expr::Unary { expr, .. } | Expr::Return { expr, .. } | Expr::Try { expr, .. } => {
            collect_loops(expr, out);
        }
        Expr::Int(_, _) | Expr::Bool(_, _) | Expr::String(_, _) | Expr::Var(_, _) => {}
    }
}

fn collect_loops_block<'a>(block: &'a clg_ast::Block, out: &mut Vec<LoopObligation<'a>>) {
    for stmt in &block.statements {
        match stmt {
            Stmt::Let { expr, .. } | Stmt::Expr { expr, .. } => collect_loops(expr, out),
            Stmt::While {
                cond,
                invariant,
                variant,
                body,
                ..
            } => {
                out.push(LoopObligation {
                    invariant,
                    variant: variant.as_deref(),
                });
                collect_loops(cond, out);
                collect_loops(invariant, out);
                if let Some(v) = variant {
                    collect_loops(v, out);
                }
                collect_loops_block(body, out);
            }
        }
    }
    if let Some(tail) = &block.tail {
        collect_loops(tail, out);
    }
}

fn expr_span(e: &Expr) -> Span {
    match e {
        Expr::Int(_, sp) | Expr::Bool(_, sp) | Expr::String(_, sp) | Expr::Var(_, sp) => *sp,
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

fn precedence_bin(op: BinOp) -> u8 {
    match op {
        BinOp::Mul | BinOp::Div => 40,
        BinOp::Add | BinOp::Sub => 30,
        BinOp::Lt | BinOp::Le | BinOp::Gt | BinOp::Ge | BinOp::Eq | BinOp::Neq => 20,
        BinOp::And => 10,
        BinOp::Or => 5,
    }
}

fn precedence_unary() -> u8 {
    50
}

fn merge_span(lhs: &Expr, rhs: &Expr) -> Span {
    let ls = span_of(lhs);
    let rs = span_of(rhs);
    Span {
        start: ls.start,
        end: rs.end,
    }
}

fn span_of(expr: &Expr) -> Span {
    match expr {
        Expr::Int(_, sp) | Expr::Bool(_, sp) | Expr::String(_, sp) | Expr::Var(_, sp) => *sp,
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
