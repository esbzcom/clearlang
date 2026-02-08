use clg_ast::{BinOp, Expr, Type, UnaryOp};
use std::collections::HashSet;

#[derive(Clone, Copy)]
struct IntBound {
    value: i64,
    inclusive: bool,
}

enum IntConstraint {
    Lower(IntBound),
    Upper(IntBound),
    Eq(i64),
    Neq(i64),
}

#[derive(Default)]
struct IntConstraintSet {
    lower: Option<IntBound>,
    upper: Option<IntBound>,
    eq: Option<i64>,
    neq: HashSet<i64>,
    unsat: bool,
}

impl IntConstraintSet {
    fn apply(&mut self, constraint: IntConstraint) {
        if self.unsat {
            return;
        }
        let ok = match constraint {
            IntConstraint::Lower(bound) => self.apply_lower(bound),
            IntConstraint::Upper(bound) => self.apply_upper(bound),
            IntConstraint::Eq(value) => self.apply_eq(value),
            IntConstraint::Neq(value) => self.apply_neq(value),
        };
        if !ok {
            self.unsat = true;
        }
    }

    fn apply_eq(&mut self, value: i64) -> bool {
        if let Some(existing) = self.eq {
            if existing != value {
                return false;
            }
        }
        if !self.satisfies_bounds(value) {
            return false;
        }
        if self.neq.contains(&value) {
            return false;
        }
        self.eq = Some(value);
        true
    }

    fn apply_neq(&mut self, value: i64) -> bool {
        if let Some(eq) = self.eq {
            if eq == value {
                return false;
            }
        }
        self.neq.insert(value);
        if let Some(only) = self.single_value() {
            if only == value {
                return false;
            }
        }
        true
    }

    fn apply_lower(&mut self, bound: IntBound) -> bool {
        if let Some(eq) = self.eq {
            if !satisfies_lower(eq, bound) {
                return false;
            }
            return true;
        }
        match self.lower {
            None => self.lower = Some(bound),
            Some(existing) => {
                if stricter_lower(bound, existing) {
                    self.lower = Some(bound);
                }
            }
        }
        if !self.bounds_allow_values() {
            return false;
        }
        if let Some(only) = self.single_value() {
            if self.neq.contains(&only) {
                return false;
            }
        }
        true
    }

    fn apply_upper(&mut self, bound: IntBound) -> bool {
        if let Some(eq) = self.eq {
            if !satisfies_upper(eq, bound) {
                return false;
            }
            return true;
        }
        match self.upper {
            None => self.upper = Some(bound),
            Some(existing) => {
                if stricter_upper(bound, existing) {
                    self.upper = Some(bound);
                }
            }
        }
        if !self.bounds_allow_values() {
            return false;
        }
        if let Some(only) = self.single_value() {
            if self.neq.contains(&only) {
                return false;
            }
        }
        true
    }

    fn satisfies_bounds(&self, value: i64) -> bool {
        if let Some(lower) = self.lower {
            if !satisfies_lower(value, lower) {
                return false;
            }
        }
        if let Some(upper) = self.upper {
            if !satisfies_upper(value, upper) {
                return false;
            }
        }
        true
    }

    fn bounds_allow_values(&self) -> bool {
        let min = match self.lower {
            None => None,
            Some(bound) => min_from_lower(bound),
        };
        if let Some(val) = min {
            let max = match self.upper {
                None => return true,
                Some(bound) => max_from_upper(bound),
            };
            if let Some(upper_val) = max {
                return val <= upper_val;
            }
            return false;
        }
        if let Some(bound) = self.upper {
            return max_from_upper(bound).is_some();
        }
        true
    }

    fn single_value(&self) -> Option<i64> {
        let lower = self.lower?;
        let upper = self.upper?;
        let min = min_from_lower(lower)?;
        let max = max_from_upper(upper)?;
        if min == max {
            Some(min)
        } else {
            None
        }
    }
}

fn stricter_lower(new: IntBound, existing: IntBound) -> bool {
    new.value > existing.value
        || (new.value == existing.value && !new.inclusive && existing.inclusive)
}

fn stricter_upper(new: IntBound, existing: IntBound) -> bool {
    new.value < existing.value
        || (new.value == existing.value && !new.inclusive && existing.inclusive)
}

fn satisfies_lower(value: i64, bound: IntBound) -> bool {
    value > bound.value || (value == bound.value && bound.inclusive)
}

fn satisfies_upper(value: i64, bound: IntBound) -> bool {
    value < bound.value || (value == bound.value && bound.inclusive)
}

fn min_from_lower(bound: IntBound) -> Option<i64> {
    if bound.inclusive {
        Some(bound.value)
    } else {
        bound.value.checked_add(1)
    }
}

fn max_from_upper(bound: IntBound) -> Option<i64> {
    if bound.inclusive {
        Some(bound.value)
    } else {
        bound.value.checked_sub(1)
    }
}

pub(super) fn predicate_is_contradiction(expr: &Expr, binder: Option<&str>, base: &Type) -> bool {
    if let Some(value) = eval_const_bool(expr) {
        return !value;
    }
    if !matches!(base, Type::Int) {
        return false;
    }
    let Some(binder) = binder else { return false };
    let mut constraints = IntConstraintSet::default();
    if collect_constraints(expr, binder, &mut constraints) {
        return constraints.unsat;
    }
    false
}

fn collect_constraints(expr: &Expr, binder: &str, constraints: &mut IntConstraintSet) -> bool {
    if constraints.unsat {
        return true;
    }
    if let Some(value) = eval_const_bool(expr) {
        if !value {
            constraints.unsat = true;
        }
        return true;
    }
    match expr {
        Expr::Bin {
            op: BinOp::And,
            lhs,
            rhs,
            ..
        } => {
            let left_ok = collect_constraints(lhs, binder, constraints);
            if constraints.unsat {
                return true;
            }
            let right_ok = collect_constraints(rhs, binder, constraints);
            if constraints.unsat {
                return true;
            }
            left_ok && right_ok
        }
        Expr::Bin { op, lhs, rhs, .. } => {
            if let Some(constraint) = comparison_constraint(*op, lhs, rhs, binder) {
                constraints.apply(constraint);
                true
            } else {
                false
            }
        }
        _ => false,
    }
}

fn comparison_constraint(op: BinOp, lhs: &Expr, rhs: &Expr, binder: &str) -> Option<IntConstraint> {
    if let (Some(offset), Some(value)) = (extract_binder_offset(lhs, binder), eval_const_int(rhs)) {
        let target = value.checked_sub(offset)?;
        return constraint_for_comparison(op, target);
    }
    if let (Some(value), Some(offset)) = (eval_const_int(lhs), extract_binder_offset(rhs, binder)) {
        let target = value.checked_sub(offset)?;
        let inverted = invert_comparison(op)?;
        return constraint_for_comparison(inverted, target);
    }
    None
}

fn constraint_for_comparison(op: BinOp, value: i64) -> Option<IntConstraint> {
    match op {
        BinOp::Lt => Some(IntConstraint::Upper(IntBound {
            value,
            inclusive: false,
        })),
        BinOp::Le => Some(IntConstraint::Upper(IntBound {
            value,
            inclusive: true,
        })),
        BinOp::Gt => Some(IntConstraint::Lower(IntBound {
            value,
            inclusive: false,
        })),
        BinOp::Ge => Some(IntConstraint::Lower(IntBound {
            value,
            inclusive: true,
        })),
        BinOp::Eq => Some(IntConstraint::Eq(value)),
        BinOp::Neq => Some(IntConstraint::Neq(value)),
        _ => None,
    }
}

fn invert_comparison(op: BinOp) -> Option<BinOp> {
    match op {
        BinOp::Lt => Some(BinOp::Gt),
        BinOp::Le => Some(BinOp::Ge),
        BinOp::Gt => Some(BinOp::Lt),
        BinOp::Ge => Some(BinOp::Le),
        BinOp::Eq => Some(BinOp::Eq),
        BinOp::Neq => Some(BinOp::Neq),
        _ => None,
    }
}

fn extract_binder_offset(expr: &Expr, binder: &str) -> Option<i64> {
    match expr {
        Expr::Var(name, _) if name == binder => Some(0),
        Expr::Bin {
            op: BinOp::Add,
            lhs,
            rhs,
            ..
        } => {
            if let Some(offset) = extract_binder_offset(lhs, binder) {
                return offset.checked_add(eval_const_int(rhs)?);
            }
            if let Some(offset) = extract_binder_offset(rhs, binder) {
                return offset.checked_add(eval_const_int(lhs)?);
            }
            None
        }
        Expr::Bin {
            op: BinOp::Sub,
            lhs,
            rhs,
            ..
        } => {
            let offset = extract_binder_offset(lhs, binder)?;
            offset.checked_sub(eval_const_int(rhs)?)
        }
        _ => None,
    }
}

fn eval_const_int(expr: &Expr) -> Option<i64> {
    match expr {
        Expr::Int(value, _) => Some(*value),
        Expr::Bin { op, lhs, rhs, .. } => {
            let left = eval_const_int(lhs)?;
            let right = eval_const_int(rhs)?;
            match op {
                BinOp::Add => left.checked_add(right),
                BinOp::Sub => left.checked_sub(right),
                BinOp::Mul => left.checked_mul(right),
                BinOp::Div => {
                    if right == 0 {
                        None
                    } else {
                        left.checked_div(right)
                    }
                }
                _ => None,
            }
        }
        _ => None,
    }
}

fn eval_const_bool(expr: &Expr) -> Option<bool> {
    match expr {
        Expr::Bool(value, _) => Some(*value),
        Expr::Unary {
            op: UnaryOp::Not,
            expr,
            ..
        } => eval_const_bool(expr).map(|value| !value),
        Expr::Bin { op, lhs, rhs, .. } => match op {
            BinOp::And => Some(eval_const_bool(lhs)? && eval_const_bool(rhs)?),
            BinOp::Or => Some(eval_const_bool(lhs)? || eval_const_bool(rhs)?),
            BinOp::Eq | BinOp::Neq | BinOp::Lt | BinOp::Le | BinOp::Gt | BinOp::Ge => {
                let left = eval_const_int(lhs)?;
                let right = eval_const_int(rhs)?;
                Some(match op {
                    BinOp::Eq => left == right,
                    BinOp::Neq => left != right,
                    BinOp::Lt => left < right,
                    BinOp::Le => left <= right,
                    BinOp::Gt => left > right,
                    BinOp::Ge => left >= right,
                    _ => return None,
                })
            }
            _ => None,
        },
        _ => None,
    }
}
