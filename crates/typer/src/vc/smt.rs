use crate::builtins::all_builtin_sigs;
use clg_ast::{BinOp, Block, Expr, MatchArm, MatchPat, Stmt, Type, UnaryOp};
use std::collections::HashSet;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum SmtHelper {
    VariantAccessors,
    OptionCtor,
    ResultCtor,
    U64BitvectorBridge,
    ListAxioms,
    FiniteSetAxioms,
    MapAxioms,
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
                    self.helpers.insert(SmtHelper::U64BitvectorBridge);
                    let lhs = self.encode_inner(lhs);
                    let rhs = self.encode_inner(rhs);
                    self.encode_u64_shift(lhs, rhs, false)
                }
                BinOp::Shr => {
                    self.helpers.insert(SmtHelper::U64BitvectorBridge);
                    let lhs = self.encode_inner(lhs);
                    let rhs = self.encode_inner(rhs);
                    self.encode_u64_shift(lhs, rhs, true)
                }
                BinOp::BitAnd => {
                    self.helpers.insert(SmtHelper::U64BitvectorBridge);
                    let lhs = self.encode_inner(lhs);
                    let rhs = self.encode_inner(rhs);
                    self.encode_u64_bitwise_binop("bvand", lhs, rhs)
                }
                BinOp::BitXor => {
                    self.helpers.insert(SmtHelper::U64BitvectorBridge);
                    let lhs = self.encode_inner(lhs);
                    let rhs = self.encode_inner(rhs);
                    self.encode_u64_bitwise_binop("bvxor", lhs, rhs)
                }
                BinOp::BitOr => {
                    self.helpers.insert(SmtHelper::U64BitvectorBridge);
                    let lhs = self.encode_inner(lhs);
                    let rhs = self.encode_inner(rhs);
                    self.encode_u64_bitwise_binop("bvor", lhs, rhs)
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
            "std::u64::rotl" => {
                self.helpers.insert(SmtHelper::U64BitvectorBridge);
                if args.len() == 2 {
                    let lhs = self.encode_inner(&args[0]);
                    let rhs = self.encode_inner(&args[1]);
                    self.encode_u64_rotate(lhs, rhs, true)
                } else {
                    "0".to_string()
                }
            }
            "std::u64::rotr" => {
                self.helpers.insert(SmtHelper::U64BitvectorBridge);
                if args.len() == 2 {
                    let lhs = self.encode_inner(&args[0]);
                    let rhs = self.encode_inner(&args[1]);
                    self.encode_u64_rotate(lhs, rhs, false)
                } else {
                    "0".to_string()
                }
            }
            _ => {
                if is_finite_set_builtin(callee) {
                    self.helpers.insert(SmtHelper::FiniteSetAxioms);
                }
                if is_list_builtin(callee) {
                    self.helpers.insert(SmtHelper::ListAxioms);
                    self.helpers.insert(SmtHelper::VariantAccessors);
                    self.helpers.insert(SmtHelper::OptionCtor);
                }
                if is_map_builtin(callee) {
                    self.helpers.insert(SmtHelper::MapAxioms);
                    self.helpers.insert(SmtHelper::VariantAccessors);
                }
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
        if self.helpers.contains(&SmtHelper::U64BitvectorBridge) {
            lines.push("; U64 bitvector bridge helpers.".to_string());
            lines.push("(declare-fun clg.u64.to_int ((_ BitVec 64)) Int)".to_string());
        }
        let builtin_lines = self.builtin_prelude();
        if !builtin_lines.is_empty() {
            lines.push("; Builtin intrinsics are modeled as uninterpreted functions.".to_string());
            lines.extend(builtin_lines);
        }
        if self.helpers.contains(&SmtHelper::FiniteSetAxioms) {
            lines.push("; Finite-set axioms for std::set::* proof reasoning.".to_string());
            lines.extend(self.finite_set_axioms());
        }
        if self.helpers.contains(&SmtHelper::ListAxioms) {
            lines.push("; List axioms for std::list::* proof reasoning.".to_string());
            lines.extend(self.list_axioms());
        }
        if self.helpers.contains(&SmtHelper::MapAxioms) {
            lines.push("; Map axioms for std::map::* read-only proof reasoning.".to_string());
            lines.extend(self.map_axioms());
        }
        lines.join("\n")
    }

    fn record_builtin_call(&mut self, callee: &str) {
        if is_builtin_name(callee) {
            self.builtin_calls.insert(callee.to_string());
            if is_finite_set_builtin(callee) {
                for name in FINITE_SET_BUILTINS {
                    self.builtin_calls.insert((*name).to_string());
                }
            }
            if is_list_builtin(callee) {
                for name in LIST_BUILTINS {
                    self.builtin_calls.insert((*name).to_string());
                }
            }
            if is_map_builtin(callee) {
                for name in MAP_PROOF_BUILTINS {
                    self.builtin_calls.insert((*name).to_string());
                }
            }
        }
    }

    fn builtin_prelude(&self) -> Vec<String> {
        if self.builtin_calls.is_empty() {
            return Vec::new();
        }
        let mut lines = Vec::new();
        for (name, params, ret, _) in all_builtin_sigs() {
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

    fn finite_set_axioms(&self) -> Vec<String> {
        let contains = smt_symbol("std::set::contains");
        let subset = smt_symbol("std::set::subset");
        let union = smt_symbol("std::set::union");
        let intersect = smt_symbol("std::set::intersect");
        let diff = smt_symbol("std::set::diff");
        let len = smt_symbol("std::set::len");

        vec![
            format!(
                "(assert (forall ((a Int) (b Int)) (= ({} a b) (forall ((x Int)) (=> ({} a x) ({} b x))))))",
                subset, contains, contains
            ),
            format!(
                "(assert (forall ((a Int) (b Int) (x Int)) (= ({} ({} a b) x) (or ({} a x) ({} b x)))))",
                contains, union, contains, contains
            ),
            format!(
                "(assert (forall ((a Int) (b Int) (x Int)) (= ({} ({} a b) x) (and ({} a x) ({} b x)))))",
                contains, intersect, contains, contains
            ),
            format!(
                "(assert (forall ((a Int) (b Int) (x Int)) (= ({} ({} a b) x) (and ({} a x) (not ({} b x))))))",
                contains, diff, contains, contains
            ),
            format!(
                "(assert (forall ((a Int) (b Int)) (<= ({} ({} a b)) ({} a))))",
                len, intersect, len
            ),
            format!(
                "(assert (forall ((a Int) (b Int)) (>= ({} ({} a b)) ({} a))))",
                len, union, len
            ),
            format!(
                "(assert (forall ((a Int) (b Int)) (<= ({} ({} a b)) ({} a))))",
                len, diff, len
            ),
        ]
    }

    fn list_axioms(&self) -> Vec<String> {
        let new_list = smt_symbol("std::list::new");
        let len = smt_symbol("std::list::len");
        let is_empty = smt_symbol("std::list::is_empty");
        let get = smt_symbol("std::list::get");
        let push = smt_symbol("std::list::push");
        let pop = smt_symbol("std::list::pop");
        let insert = smt_symbol("std::list::insert");
        let insert_checked = smt_symbol("std::list::insert_checked");
        let remove = smt_symbol("std::list::remove");
        let remove_checked = smt_symbol("std::list::remove_checked");
        let remove_take = smt_symbol("std::list::remove_take");
        let remove_take_list = "cl.list.remove_take.list";
        let remove_take_value = "cl.list.remove_take.value";

        vec![
            format!("(declare-fun {} (Int) Int)", remove_take_list),
            format!("(declare-fun {} (Int) Int)", remove_take_value),
            format!("(assert (forall ((l Int)) (>= ({} l) 0)))", len),
            format!("(assert (forall ((l Int)) (= ({} l) (= ({} l) 0))))", is_empty, len),
            format!("(assert (= ({} ({})) 0))", len, new_list),
            format!("(assert ({} ({})))", is_empty, new_list),
            format!(
                "(assert (forall ((l Int) (x Int)) (= ({} ({} l x)) (+ ({} l) 1))))",
                len, push, len
            ),
            format!(
                "(assert (forall ((l Int) (i Int)) (=> (and (<= 0 i) (< i ({} l))) (= (cl.variant.tag ({} l i)) 1))))",
                len, get
            ),
            format!(
                "(assert (forall ((l Int) (i Int)) (=> (or (< i 0) (>= i ({} l))) (= (cl.variant.tag ({} l i)) 0))))",
                len, get
            ),
            format!(
                "(assert (forall ((l_before Int) (l_after Int) (i Int)) (=> (and (= l_before l_after) (<= 0 i) (< i ({} l_before))) (= ({} l_before i) ({} l_after i)))))",
                len, get, get
            ),
            format!(
                "(assert (forall ((l Int)) (=> (> ({} l) 0) (= (cl.variant.tag ({} l)) 1))))",
                len, pop
            ),
            format!(
                "(assert (forall ((l Int)) (=> (<= ({} l) 0) (= (cl.variant.tag ({} l)) 0))))",
                len, pop
            ),
            format!(
                "(assert (forall ((l Int) (x Int) (i Int)) (=> (and (<= 0 i) (<= i ({} l))) (= ({} ({} l x i)) (+ ({} l) 1)))))",
                len, len, insert, len
            ),
            format!(
                "(assert (forall ((l Int) (i Int)) (=> (and (<= 0 i) (< i ({} l))) (= ({} ({} l i)) (- ({} l) 1)))))",
                len, len, remove, len
            ),
            format!(
                "(assert (forall ((l Int) (x Int) (i Int) (j Int)) (=> (and (<= 0 i) (<= i ({} l)) (<= 0 j) (< j i)) (= ({} ({} l x i) j) ({} l j)))))",
                len, get, insert, get
            ),
            format!(
                "(assert (forall ((l Int) (x Int) (i Int)) (=> (and (<= 0 i) (<= i ({} l))) (= ({} ({} l x i) i) (cl.option.mk 1 x 0)))))",
                len, get, insert
            ),
            format!(
                "(assert (forall ((l Int) (x Int) (i Int) (j Int)) (=> (and (<= 0 i) (<= i ({} l)) (< i j) (< j ({} ({} l x i)))) (= ({} ({} l x i) j) ({} l (- j 1)))))))",
                len, len, insert, get, insert, get
            ),
            format!(
                "(assert (forall ((l Int) (i Int) (j Int)) (=> (and (<= 0 i) (< i ({} l)) (<= 0 j) (< j i)) (= ({} ({} l i) j) ({} l j)))))",
                len, get, remove, get
            ),
            format!(
                "(assert (forall ((l Int) (i Int) (j Int)) (=> (and (<= 0 i) (< i ({} l)) (<= i j) (< j ({} ({} l i)))) (= ({} ({} l i) j) ({} l (+ j 1)))))))",
                len, len, remove, get, remove, get
            ),
            format!(
                "(assert (forall ((l Int) (x Int) (i Int)) (=> (and (<= 0 i) (<= i ({} l))) (= (cl.variant.tag ({} l x i)) 1))))",
                len, insert_checked
            ),
            format!(
                "(assert (forall ((l Int) (x Int) (i Int)) (=> (or (< i 0) (> i ({} l))) (= (cl.variant.tag ({} l x i)) 0))))",
                len, insert_checked
            ),
            format!(
                "(assert (forall ((l Int) (x Int) (i Int)) (=> (and (<= 0 i) (<= i ({} l))) (= (cl.variant.payload_lo ({} l x i)) ({} l x i)))))",
                len, insert_checked, insert
            ),
            format!(
                "(assert (forall ((l Int) (x Int) (i Int)) (=> (or (< i 0) (> i ({} l))) (= (cl.variant.payload_lo ({} l x i)) 1))))",
                len, insert_checked
            ),
            format!(
                "(assert (forall ((l Int) (i Int)) (=> (and (<= 0 i) (< i ({} l))) (= (cl.variant.tag ({} l i)) 1))))",
                len, remove_checked
            ),
            format!(
                "(assert (forall ((l Int) (i Int)) (=> (or (< i 0) (>= i ({} l))) (= (cl.variant.tag ({} l i)) 0))))",
                len, remove_checked
            ),
            format!(
                "(assert (forall ((l Int) (i Int)) (=> (and (<= 0 i) (< i ({} l))) (= (cl.variant.payload_lo ({} l i)) ({} l i)))))",
                len, remove_checked, remove
            ),
            format!(
                "(assert (forall ((l Int) (i Int)) (=> (or (< i 0) (>= i ({} l))) (= (cl.variant.payload_lo ({} l i)) 1))))",
                len, remove_checked
            ),
            format!(
                "(assert (forall ((l Int) (i Int)) (=> (and (<= 0 i) (< i ({} l))) (= ({} ({} ({} l i))) (- ({} l) 1)))))",
                len, len, remove_take_list, remove_take, len
            ),
            format!(
                "(assert (forall ((l Int) (i Int)) (=> (and (<= 0 i) (< i ({} l))) (= ({} ({} l i)) ({} l i)))))",
                len, remove_take_value, remove_take, get
            ),
        ]
    }

    fn map_axioms(&self) -> Vec<String> {
        let new_map = smt_symbol("std::map::new");
        let len = smt_symbol("std::map::len");
        let is_empty = smt_symbol("std::map::is_empty");
        let contains = smt_symbol("std::map::contains");
        let get = smt_symbol("std::map::get");
        let insert = smt_symbol("std::map::insert");
        let insert_take = smt_symbol("std::map::insert_take");
        let insert_take_map = "cl.map.insert_take.map";
        let insert_take_prev = "cl.map.insert_take.prev";
        let remove = smt_symbol("std::map::remove");
        let remove_take = smt_symbol("std::map::remove_take");
        let remove_take_map = "cl.map.remove_take.map";
        let remove_take_prev = "cl.map.remove_take.prev";

        vec![
            format!("(declare-fun {} (Int) Int)", insert_take_map),
            format!("(declare-fun {} (Int) Int)", insert_take_prev),
            format!("(declare-fun {} (Int) Int)", remove_take_map),
            format!("(declare-fun {} (Int) Int)", remove_take_prev),
            format!("(assert (forall ((m Int)) (>= ({} m) 0)))", len),
            format!("(assert (forall ((m Int)) (= ({} m) (= ({} m) 0))))", is_empty, len),
            format!("(assert (= ({} ({})) 0))", len, new_map),
            format!("(assert ({} ({})))", is_empty, new_map),
            format!(
                "(assert (forall ((m Int) (k Int)) (=> ({} m k) (= (cl.variant.tag ({} m k)) 1))))",
                contains, get
            ),
            format!(
                "(assert (forall ((m Int) (k Int)) (=> (not ({} m k)) (= (cl.variant.tag ({} m k)) 0))))",
                contains, get
            ),
            format!(
                "(assert (forall ((m Int) (k Int) (v Int)) ({} ({} m k v) k)))",
                contains, insert
            ),
            format!(
                "(assert (forall ((m Int) (k Int) (v Int) (j Int)) (=> (not (= j k)) (= ({} ({} m k v) j) ({} m j)))))",
                contains, insert, contains
            ),
            format!(
                "(assert (forall ((m Int) (k Int) (v Int)) (= (cl.variant.tag ({} ({} m k v) k)) 1)))",
                get, insert
            ),
            format!(
                "(assert (forall ((m Int) (k Int) (v Int)) (= (cl.variant.payload_lo ({} ({} m k v) k)) v)))",
                get, insert
            ),
            format!(
                "(assert (forall ((m Int) (k Int) (v Int) (j Int)) (=> (not (= j k)) (= (cl.variant.tag ({} ({} m k v) j)) (cl.variant.tag ({} m j))))))",
                get, insert, get
            ),
            format!(
                "(assert (forall ((m Int) (k Int) (v Int) (j Int)) (=> (not (= j k)) (= (cl.variant.payload_lo ({} ({} m k v) j)) (cl.variant.payload_lo ({} m j))))))",
                get, insert, get
            ),
            format!(
                "(assert (forall ((m Int) (k Int) (v Int)) (= ({} ({} m k v)) (ite ({} m k) ({} m) (+ ({} m) 1)))))",
                len, insert, contains, len, len
            ),
            format!(
                "(assert (forall ((m Int) (k Int) (v Int)) (= ({} ({} m k v)) ({} m k v))))",
                insert_take_map, insert_take, insert
            ),
            format!(
                "(assert (forall ((m Int) (k Int) (v Int)) (=> ({} m k) (= (cl.variant.tag ({} ({} m k v))) 1))))",
                contains, insert_take_prev, insert_take
            ),
            format!(
                "(assert (forall ((m Int) (k Int) (v Int)) (=> (not ({} m k)) (= (cl.variant.tag ({} ({} m k v))) 0))))",
                contains, insert_take_prev, insert_take
            ),
            format!(
                "(assert (forall ((m Int) (k Int) (v Int)) (=> ({} m k) (= (cl.variant.payload_lo ({} ({} m k v))) (cl.variant.payload_lo ({} m k))))))",
                contains, insert_take_prev, insert_take, get
            ),
            format!(
                "(assert (forall ((m Int) (k Int)) (not ({} ({} m k) k))))",
                contains, remove
            ),
            format!(
                "(assert (forall ((m Int) (k Int) (j Int)) (=> (not (= j k)) (= ({} ({} m k) j) ({} m j)))))",
                contains, remove, contains
            ),
            format!(
                "(assert (forall ((m Int) (k Int)) (= (cl.variant.tag ({} ({} m k) k)) 0)))",
                get, remove
            ),
            format!(
                "(assert (forall ((m Int) (k Int) (j Int)) (=> (not (= j k)) (= (cl.variant.tag ({} ({} m k) j)) (cl.variant.tag ({} m j))))))",
                get, remove, get
            ),
            format!(
                "(assert (forall ((m Int) (k Int) (j Int)) (=> (not (= j k)) (= (cl.variant.payload_lo ({} ({} m k) j)) (cl.variant.payload_lo ({} m j))))))",
                get, remove, get
            ),
            format!(
                "(assert (forall ((m Int) (k Int)) (= ({} ({} m k)) (ite ({} m k) (- ({} m) 1) ({} m)))))",
                len, remove, contains, len, len
            ),
            format!(
                "(assert (forall ((m Int) (k Int)) (= ({} ({} m k)) ({} m k))))",
                remove_take_map, remove_take, remove
            ),
            format!(
                "(assert (forall ((m Int) (k Int)) (=> ({} m k) (= (cl.variant.tag ({} ({} m k))) 1))))",
                contains, remove_take_prev, remove_take
            ),
            format!(
                "(assert (forall ((m Int) (k Int)) (=> (not ({} m k)) (= (cl.variant.tag ({} ({} m k))) 0))))",
                contains, remove_take_prev, remove_take
            ),
            format!(
                "(assert (forall ((m Int) (k Int)) (=> ({} m k) (= (cl.variant.payload_lo ({} ({} m k))) (cl.variant.payload_lo ({} m k))))))",
                contains, remove_take_prev, remove_take, get
            ),
        ]
    }

    fn encode_u64_int_to_bv(&self, term: &str) -> String {
        format!("((_ int2bv 64) {})", term)
    }

    fn encode_u64_bv_to_int(&self, term: &str) -> String {
        format!("(clg.u64.to_int {})", term)
    }

    fn encode_u64_shift_mask(&self, rhs_int_term: &str) -> String {
        format!(
            "(bvand {} #x000000000000003f)",
            self.encode_u64_int_to_bv(rhs_int_term)
        )
    }

    fn encode_u64_bitwise_binop(&self, op: &str, lhs: String, rhs: String) -> String {
        let lhs_bv = self.encode_u64_int_to_bv(lhs.as_str());
        let rhs_bv = self.encode_u64_int_to_bv(rhs.as_str());
        self.encode_u64_bv_to_int(format!("({op} {lhs_bv} {rhs_bv})").as_str())
    }

    fn encode_u64_shift(&self, lhs: String, rhs: String, right: bool) -> String {
        let lhs_bv = self.encode_u64_int_to_bv(lhs.as_str());
        let rhs_mask = self.encode_u64_shift_mask(rhs.as_str());
        let op = if right { "bvlshr" } else { "bvshl" };
        self.encode_u64_bv_to_int(format!("({op} {lhs_bv} {rhs_mask})").as_str())
    }

    fn encode_u64_rotate(&self, lhs: String, rhs: String, left: bool) -> String {
        let lhs_bv = self.encode_u64_int_to_bv(lhs.as_str());
        let shift = self.encode_u64_shift_mask(rhs.as_str());
        let inv_shift = format!(
            "(bvand (bvsub #x0000000000000040 {}) #x000000000000003f)",
            shift
        );
        let rotated = if left {
            format!(
                "(bvor (bvshl {} {}) (bvlshr {} {}))",
                lhs_bv, shift, lhs_bv, inv_shift
            )
        } else {
            format!(
                "(bvor (bvlshr {} {}) (bvshl {} {}))",
                lhs_bv, shift, lhs_bv, inv_shift
            )
        };
        self.encode_u64_bv_to_int(rotated.as_str())
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
    all_builtin_sigs()
        .iter()
        .any(|(name, _params, _ret, _)| name == callee)
}

const FINITE_SET_BUILTINS: &[&str] = &[
    "std::set::len",
    "std::set::contains",
    "std::set::subset",
    "std::set::union",
    "std::set::intersect",
    "std::set::diff",
];

const LIST_BUILTINS: &[&str] = &[
    "std::list::new",
    "std::list::len",
    "std::list::is_empty",
    "std::list::get",
    "std::list::push",
    "std::list::pop",
    "std::list::insert",
    "std::list::insert_checked",
    "std::list::remove",
    "std::list::remove_checked",
    "std::list::remove_take",
];

const MAP_PROOF_BUILTINS: &[&str] = &[
    "std::map::new",
    "std::map::len",
    "std::map::is_empty",
    "std::map::contains",
    "std::map::get",
    "std::map::insert",
    "std::map::insert_take",
    "std::map::remove",
    "std::map::remove_take",
];

fn is_finite_set_builtin(callee: &str) -> bool {
    matches!(
        callee,
        "std::set::len"
            | "std::set::contains"
            | "std::set::subset"
            | "std::set::union"
            | "std::set::intersect"
            | "std::set::diff"
    )
}

fn is_list_builtin(callee: &str) -> bool {
    matches!(
        callee,
        "std::list::new"
            | "std::list::len"
            | "std::list::is_empty"
            | "std::list::get"
            | "std::list::push"
            | "std::list::pop"
            | "std::list::insert"
            | "std::list::insert_checked"
            | "std::list::remove"
            | "std::list::remove_checked"
            | "std::list::remove_take"
    )
}

fn is_map_builtin(callee: &str) -> bool {
    matches!(
        callee,
        "std::map::new"
            | "std::map::len"
            | "std::map::is_empty"
            | "std::map::contains"
            | "std::map::get"
            | "std::map::insert"
            | "std::map::insert_take"
            | "std::map::remove"
            | "std::map::remove_take"
    )
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
