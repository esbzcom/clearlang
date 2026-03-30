use crate::builtins::builtin_sigs;
use clg_ast::{BinOp, Block, Expr, MatchArm, MatchPat, Stmt, Type, UnaryOp};
use std::collections::HashSet;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum SmtHelper {
    VariantAccessors,
    OptionCtor,
    ResultCtor,
    BitwiseOps,
}

#[derive(Default)]
pub(super) struct SmtEncoder {
    helpers: HashSet<SmtHelper>,
    fresh: usize,
    builtin_calls: HashSet<String>,
}

impl SmtEncoder {
    pub(super) fn encode(&mut self, expr: &Expr) -> String {
        self.encode_inner(expr)
    }

    fn encode_inner(&mut self, expr: &Expr) -> String {
        match expr {
            Expr::Int(n, _) => n.to_string(),
            Expr::Bool(b, _) => b.to_string(),
            Expr::String(s, _) => format!("\"{}\"", s),
            Expr::Var(name, _) => name.clone(),
            Expr::ArrayLit { .. }
            | Expr::TupleLit { .. }
            | Expr::StructLit { .. }
            | Expr::FieldAccess { .. }
            | Expr::Index { .. } => "0".to_string(),
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
                BinOp::Shl => {
                    self.helpers.insert(SmtHelper::BitwiseOps);
                    format!(
                        "(clg.shl {} {})",
                        self.encode_inner(lhs),
                        self.encode_inner(rhs)
                    )
                }
                BinOp::Shr => {
                    self.helpers.insert(SmtHelper::BitwiseOps);
                    format!(
                        "(clg.shr {} {})",
                        self.encode_inner(lhs),
                        self.encode_inner(rhs)
                    )
                }
                BinOp::BitAnd => {
                    self.helpers.insert(SmtHelper::BitwiseOps);
                    format!(
                        "(clg.bit_and {} {})",
                        self.encode_inner(lhs),
                        self.encode_inner(rhs)
                    )
                }
                BinOp::BitXor => {
                    self.helpers.insert(SmtHelper::BitwiseOps);
                    format!(
                        "(clg.bit_xor {} {})",
                        self.encode_inner(lhs),
                        self.encode_inner(rhs)
                    )
                }
                BinOp::BitOr => {
                    self.helpers.insert(SmtHelper::BitwiseOps);
                    format!(
                        "(clg.bit_or {} {})",
                        self.encode_inner(lhs),
                        self.encode_inner(rhs)
                    )
                }
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
            Expr::Block { block } => self.encode_block(block),
            Expr::Lambda { .. } => "0".to_string(),
        }
    }

    fn encode_block(&mut self, block: &Block) -> String {
        let mut bindings: Vec<(String, String)> = Vec::new();
        for stmt in &block.statements {
            match stmt {
                Stmt::Let { name, expr, .. } => {
                    bindings.push((name.clone(), self.encode_inner(expr)));
                }
                Stmt::Expr { expr, .. } => {
                    let _ = self.encode_inner(expr);
                }
                Stmt::While {
                    cond,
                    invariant,
                    variant,
                    body,
                    ..
                } => {
                    let _ = self.encode_inner(cond);
                    let _ = self.encode_inner(invariant);
                    if let Some(variant_expr) = variant {
                        let _ = self.encode_inner(variant_expr);
                    }
                    let _ = self.encode_block(body);
                }
            }
        }

        let mut body = block
            .tail
            .as_ref()
            .map(|tail| self.encode_inner(tail.as_ref()))
            .unwrap_or_else(|| "0".to_string());
        for (name, value) in bindings.into_iter().rev() {
            body = format!("(let (({} {})) {})", name, value, body);
        }
        body
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
                self.record_builtin_call(callee);
                let parts: Vec<String> = args.iter().map(|a| self.encode_inner(a)).collect();
                let smt_callee = smt_symbol(callee);
                if parts.is_empty() {
                    format!("({})", smt_callee)
                } else {
                    format!("({} {})", smt_callee, parts.join(" "))
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
            return "0".to_string();
        }
        if !arms.iter().all(|arm| {
            matches!(
                arm.pat,
                MatchPat::Some(_) | MatchPat::None | MatchPat::Ok(_) | MatchPat::Err(_)
            )
        }) {
            return "0".to_string();
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
            MatchPat::Wildcard | MatchPat::EnumVariant { .. } => 0,
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
            MatchPat::None | MatchPat::Wildcard | MatchPat::EnumVariant { .. } => body,
        }
    }

    fn fresh_sym(&mut self, prefix: &str) -> String {
        let sym = format!("{}${}", prefix, self.fresh);
        self.fresh += 1;
        sym
    }

    pub(super) fn wrap_vc(&self, body: &str) -> String {
        self.wrap_vc_with_extra(body, None)
    }

    pub(super) fn wrap_vc_with_extra(&self, body: &str, extra: Option<&str>) -> String {
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
        if self.helpers.is_empty() && self.builtin_calls.is_empty() {
            return String::new();
        }
        let mut lines: Vec<String> = Vec::new();
        if self.helpers.contains(&SmtHelper::VariantAccessors) {
            lines.push("; Option/Result variants use (tag, payload_lo, payload_hi)".to_string());
            lines.push("(declare-fun cl.variant.tag (Int) Int)".to_string());
            lines.push("(declare-fun cl.variant.payload_lo (Int) Int)".to_string());
            lines.push("(declare-fun cl.variant.payload_hi (Int) Int)".to_string());
        }
        if self.helpers.contains(&SmtHelper::OptionCtor) {
            lines.push("(declare-fun cl.option.mk (Int Int Int) Int)".to_string());
        }
        if self.helpers.contains(&SmtHelper::ResultCtor) {
            lines.push("(declare-fun cl.result.mk (Int Int Int) Int)".to_string());
        }
        if self.helpers.contains(&SmtHelper::BitwiseOps) {
            lines.push("; Bitwise ops are modeled as uninterpreted functions.".to_string());
            lines.push("(declare-fun clg.bit_and (Int Int) Int)".to_string());
            lines.push("(declare-fun clg.bit_or (Int Int) Int)".to_string());
            lines.push("(declare-fun clg.bit_xor (Int Int) Int)".to_string());
            lines.push("(declare-fun clg.shl (Int Int) Int)".to_string());
            lines.push("(declare-fun clg.shr (Int Int) Int)".to_string());
        }
        let builtin_lines = self.builtin_prelude();
        if !builtin_lines.is_empty() {
            lines.push("; Builtin intrinsics are modeled as uninterpreted functions.".to_string());
            lines.extend(builtin_lines);
        }
        lines.join("\n")
    }

    fn record_builtin_call(&mut self, callee: &str) {
        if is_builtin_name(callee) {
            self.builtin_calls.insert(callee.to_string());
        }
    }

    fn builtin_prelude(&self) -> Vec<String> {
        if self.builtin_calls.is_empty() {
            return Vec::new();
        }
        let mut lines = Vec::new();
        for (name, params, ret, _) in builtin_sigs() {
            if !self.builtin_calls.contains(&name) {
                continue;
            }
            let args: Vec<&str> = params
                .iter()
                .map(|param| smt_sort_for_builtin(&param.ty))
                .collect();
            let ret_sort = smt_sort_for_builtin(&ret);
            let smt_name = smt_symbol(&name);
            lines.push(format!(
                "(declare-fun {} ({}) {})",
                smt_name,
                args.join(" "),
                ret_sort
            ));
        }
        lines
    }
}

fn smt_symbol(name: &str) -> String {
    if is_simple_smt_symbol(name) {
        name.to_string()
    } else {
        format!("|{}|", name)
    }
}

fn is_simple_smt_symbol(name: &str) -> bool {
    let mut chars = name.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    if first.is_ascii_digit() || !is_smt_symbol_char(first) {
        return false;
    }
    chars.all(is_smt_symbol_char)
}

fn is_smt_symbol_char(c: char) -> bool {
    c.is_ascii_alphanumeric()
        || matches!(
            c,
            '_' | '-'
                | '+'
                | '*'
                | '/'
                | '='
                | '%'
                | '?'
                | '!'
                | '.'
                | '$'
                | '<'
                | '>'
                | '~'
                | '@'
                | '^'
                | '&'
        )
}

fn is_builtin_name(callee: &str) -> bool {
    builtin_sigs()
        .iter()
        .any(|(name, _params, _ret, _)| name == callee)
}

fn smt_sort_for_builtin(ty: &Type) -> &'static str {
    match ty {
        Type::Int | Type::U8 | Type::U64 | Type::U128 | Type::U256 => "Int",
        Type::Bool => "Bool",
        Type::String | Type::Bytes => "String",
        Type::Option(_)
        | Type::Result(_, _)
        | Type::List(_)
        | Type::Set(_)
        | Type::Map(_, _)
        | Type::Array(_, _)
        | Type::Slice(_)
        | Type::Tuple(_)
        | Type::Fn { .. }
        | Type::Named { .. } => "Int",
    }
}
