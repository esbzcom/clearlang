use clg_ast::{BinOp, Effect, Expr, Program, Span, UnaryOp};

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
        if !matches!(func.effect, Effect::None | Effect::Pure) {
            continue;
        }
        if func.ensures.is_empty() {
            continue;
        }
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
        let pre_smt = expr_to_smt2(&pre_expr);
        for (idx, ensure) in func.ensures.iter().enumerate() {
            let post_ast = expr_to_source(&ensure.expr, 0);
            let post_smt = expr_to_smt2(&ensure.expr);
            let substituted = substitute_result(&ensure.expr, &func.body);
            let vc_smt2 = format!("(=> {} {})", pre_smt, expr_to_smt2(&substituted));
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
        Expr::Match { .. } | Expr::If { .. } => format!("{:?}", expr),
    }
}

fn expr_to_smt2(expr: &Expr) -> String {
    match expr {
        Expr::Int(n, _) => n.to_string(),
        Expr::Bool(b, _) => b.to_string(),
        Expr::String(s, _) => format!("\"{}\"", s),
        Expr::Var(name, _) => name.clone(),
        Expr::Unary { op, expr, .. } => match op {
            UnaryOp::Not => format!("(not {})", expr_to_smt2(expr)),
        },
        Expr::Bin { op, lhs, rhs, .. } => match op {
            BinOp::Add => format!("(+ {} {})", expr_to_smt2(lhs), expr_to_smt2(rhs)),
            BinOp::Sub => format!("(- {} {})", expr_to_smt2(lhs), expr_to_smt2(rhs)),
            BinOp::Mul => format!("(* {} {})", expr_to_smt2(lhs), expr_to_smt2(rhs)),
            BinOp::Div => format!("(div {} {})", expr_to_smt2(lhs), expr_to_smt2(rhs)),
            BinOp::Lt => format!("(< {} {})", expr_to_smt2(lhs), expr_to_smt2(rhs)),
            BinOp::Le => format!("(<= {} {})", expr_to_smt2(lhs), expr_to_smt2(rhs)),
            BinOp::Gt => format!("(> {} {})", expr_to_smt2(lhs), expr_to_smt2(rhs)),
            BinOp::Ge => format!("(>= {} {})", expr_to_smt2(lhs), expr_to_smt2(rhs)),
            BinOp::Eq => format!("(= {} {})", expr_to_smt2(lhs), expr_to_smt2(rhs)),
            BinOp::Neq => format!("(not (= {} {}))", expr_to_smt2(lhs), expr_to_smt2(rhs)),
            BinOp::And => format!("(and {} {})", expr_to_smt2(lhs), expr_to_smt2(rhs)),
            BinOp::Or => format!("(or {} {})", expr_to_smt2(lhs), expr_to_smt2(rhs)),
        },
        Expr::Call { callee, args, .. } => {
            let parts: Vec<String> = args.iter().map(expr_to_smt2).collect();
            if parts.is_empty() {
                format!("({})", callee)
            } else {
                format!("({} {})", callee, parts.join(" "))
            }
        }
        Expr::Return { expr, .. } => expr_to_smt2(expr),
        Expr::Match { .. } | Expr::If { .. } => format!("; unsupported expr {:?}", expr),
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
        | Expr::Unary { span, .. } => *span,
    }
}
