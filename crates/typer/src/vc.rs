use crate::guards::{collect_mut_calls, guard_callee_for_kind, MutCall};
use clg_ast::{BinOp, Effect, Expr, MatchArm, MatchPat, Program, Span, Stmt, UnaryOp};
use std::collections::HashSet;

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

pub fn generate_vcs(program: &Program) -> Vec<VerificationCondition> {
    let mut out = Vec::new();
    for func in &program.funcs {
        let req_exprs: Vec<Expr> = func.requires.iter().map(|c| c.expr.clone()).collect();
        let pre_expr = if req_exprs.is_empty() {
            Expr::Bool(true, Span { start: 0, end: 0 })
        } else {
            fold_conjunction(req_exprs.clone())
        };
        let pre_span = if func.requires.is_empty() {
            None
        } else {
            let start = func.requires.first().unwrap().span.start;
            let end = func.requires.last().unwrap().span.end;
            Some(Span { start, end })
        };
        let pre_ast = expr_to_source(&pre_expr, 0);

        if matches!(func.effect, Effect::None | Effect::Pure) && !func.ensures.is_empty() {
            for (idx, ensure) in func.ensures.iter().enumerate() {
                let post_ast = expr_to_source(&ensure.expr, 0);
                let substituted = substitute_result(&ensure.expr, &func.body);

                let mut encoder = SmtEncoder::default();
                let pre_smt = encoder.encode(&pre_expr);
                let post_smt = encoder.encode(&ensure.expr);
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
                        span: Some(ensure.span),
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
    }
    out.sort_by(|a, b| a.function.cmp(&b.function).then(a.vc_id.cmp(&b.vc_id)));
    out
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
        Expr::Block { block } => {
            let mut new_block = block.as_ref().clone();
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
                })
                .collect();
            new_block.tail = block
                .tail
                .as_ref()
                .map(|expr| Box::new(substitute_result(expr, replacement)));
            Expr::Block {
                block: Box::new(new_block),
            }
        }
        Expr::Match { .. } | Expr::If { .. } => expr.clone(),
    }
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
        let prelude = self.helpers_prelude();
        if prelude.is_empty() {
            body.to_string()
        } else {
            format!(
                "{}
{}",
                prelude, body
            )
        }
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
