use clg_ast::{BinOp, Expr, MatchArm, MatchPat, Stmt, UnaryOp};

pub(super) fn expr_to_source(expr: &Expr, parent_prec: u8) -> String {
    match expr {
        Expr::Int(n, _) => n.to_string(),
        Expr::Bool(b, _) => b.to_string(),
        Expr::String(s, _) => format!("\"{}\"", s),
        Expr::Var(name, _) => name.clone(),
        Expr::ArrayLit { elems, .. } => {
            let items: Vec<String> = elems.iter().map(|e| expr_to_source(e, 0)).collect();
            format!("[{}]", items.join(", "))
        }
        Expr::TupleLit { elems, .. } => {
            let items: Vec<String> = elems.iter().map(|e| expr_to_source(e, 0)).collect();
            format!("({})", items.join(", "))
        }
        Expr::StructLit { name, fields, .. } => {
            let items: Vec<String> = fields
                .iter()
                .map(|f| format!("{}: {}", f.name, expr_to_source(&f.expr, 0)))
                .collect();
            format!("{} {{ {} }}", name, items.join(", "))
        }
        Expr::FieldAccess { base, field, .. } => {
            let base_str = expr_to_source(base, precedence_unary());
            format!("{}.{}", base_str, field)
        }
        Expr::Index { base, index, .. } => {
            let base_str = expr_to_source(base, precedence_unary());
            let index_str = expr_to_source(index, 0);
            format!("{}[{}]", base_str, index_str)
        }
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
            let rendered = match op {
                BinOp::Add => format!("{} + {}", left, right),
                BinOp::Sub => format!("{} - {}", left, right),
                BinOp::Mul => format!("{} * {}", left, right),
                BinOp::Div => format!("{} / {}", left, right),
                BinOp::Shl => format!("{} << {}", left, right),
                BinOp::Shr => format!("{} >> {}", left, right),
                BinOp::BitAnd => format!("{} & {}", left, right),
                BinOp::BitXor => format!("{} ^ {}", left, right),
                BinOp::BitOr => format!("{} | {}", left, right),
                BinOp::Lt => format!("{} < {}", left, right),
                BinOp::Le => format!("{} <= {}", left, right),
                BinOp::Gt => format!("{} > {}", left, right),
                BinOp::Ge => format!("{} >= {}", left, right),
                BinOp::Eq => format!("{} == {}", left, right),
                BinOp::Neq => format!("{} != {}", left, right),
                BinOp::And => format!("{} && {}", left, right),
                BinOp::Or => format!("{} || {}", left, right),
            };
            if prec < parent_prec {
                format!("({})", rendered)
            } else {
                rendered
            }
        }
        Expr::Call { callee, args, .. } => {
            let params: Vec<String> = args.iter().map(|a| expr_to_source(a, 0)).collect();
            format!("{}({})", callee, params.join(", "))
        }
        Expr::Return { expr, .. } => format!("return {}", expr_to_source(expr, 0)),
        Expr::Try { expr, .. } => {
            let inner = expr_to_source(expr, precedence_unary());
            format!("{}?", inner)
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
                    Stmt::Expr { expr, .. } => format!("{};", expr_to_source(expr.as_ref(), 0)),
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
        Expr::Lambda { params, body, .. } => {
            let params_src = params
                .iter()
                .map(|p| format!("{}: {:?}", p.name, p.ty))
                .collect::<Vec<_>>()
                .join(", ");
            format!("({}) => {}", params_src, expr_to_source(body, 0))
        }
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
        MatchPat::Wildcard => "_".to_string(),
        MatchPat::EnumVariant {
            enum_name,
            variant,
            binders,
        } => {
            if binders.is_empty() {
                format!("{}::{}", enum_name, variant)
            } else {
                format!("{}::{}({})", enum_name, variant, binders.join(", "))
            }
        }
    };
    let body = expr_to_source(&arm.expr, 0);
    ArmSource { pat, body }
}

fn precedence_bin(op: BinOp) -> u8 {
    match op {
        BinOp::Mul | BinOp::Div => 60,
        BinOp::Add | BinOp::Sub => 50,
        BinOp::Shl | BinOp::Shr => 45,
        BinOp::BitAnd => 40,
        BinOp::BitXor => 39,
        BinOp::BitOr => 38,
        BinOp::Lt | BinOp::Le | BinOp::Gt | BinOp::Ge | BinOp::Eq | BinOp::Neq => 30,
        BinOp::And => 10,
        BinOp::Or => 5,
    }
}

fn precedence_unary() -> u8 {
    50
}
