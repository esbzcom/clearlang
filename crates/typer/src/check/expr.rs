use super::{
    base_type, base_types_match, binding_compatible, effect_label, find_resource_collection,
    is_resource_type, refinement_loss, AliasMap, EffectLevel, FnSig, LocalBinding, RETURN_KEY,
};
use crate::errors::TyperError;
use anyhow::{bail, Result};
use clg_ast::{BinOp, Block, Expr, ParamKind, Span, Stmt, Type, UnaryOp};
use std::collections::HashMap;
#[derive(Clone, Debug)]
pub(super) enum ResourceState {
    Owned { borrows: usize },
    ActiveBorrow,
    Consumed { span: Span },
}
#[derive(Clone, Debug)]
pub(super) struct TrackedResource {
    pub state: ResourceState,
    pub must_consume: bool,
}
#[derive(Clone, Debug, Default)]
pub(super) struct ResourceTracker {
    states: HashMap<String, TrackedResource>,
}
impl ResourceTracker {
    pub(super) fn new() -> Self {
        Self {
            states: HashMap::new(),
        }
    }
    pub(super) fn with_capacity(capacity: usize) -> Self {
        Self {
            states: HashMap::with_capacity(capacity),
        }
    }
    pub(super) fn register_param(&mut self, name: &str, kind: ParamKind, ty: &Type) {
        if let Type::Resource(_) = ty {
            let state = match kind {
                ParamKind::Consume => ResourceState::Owned { borrows: 0 },
                ParamKind::Borrow => ResourceState::ActiveBorrow,
            };
            let must_consume = matches!(kind, ParamKind::Consume);
            self.states.insert(
                name.to_string(),
                TrackedResource {
                    state,
                    must_consume,
                },
            );
        }
    }
    pub(super) fn register_local(&mut self, name: &str, ty: &Type) {
        if let Type::Resource(_) = ty {
            self.states.insert(
                name.to_string(),
                TrackedResource {
                    state: ResourceState::Owned { borrows: 0 },
                    must_consume: true,
                },
            );
        }
    }
    pub(super) fn retain_keys_from(&mut self, baseline: &ResourceTracker) {
        self.states
            .retain(|name, _| baseline.states.contains_key(name));
    }
    pub(super) fn merge_branch(&mut self, other: &ResourceTracker, span: Span) -> Result<()> {
        for (name, left) in &self.states {
            let right = other
                .states
                .get(name)
                .ok_or_else(|| TyperError::resource_branch_mismatch(name, span))?;
            if !Self::states_compatible(&left.state, &right.state)
                || left.must_consume != right.must_consume
            {
                return Err(TyperError::resource_branch_mismatch(name, span).into());
            }
        }
        for name in other.states.keys() {
            if !self.states.contains_key(name) {
                return Err(TyperError::resource_branch_mismatch(name, span).into());
            }
        }
        Ok(())
    }
    fn states_compatible(left: &ResourceState, right: &ResourceState) -> bool {
        match (left, right) {
            (ResourceState::Owned { borrows: lb }, ResourceState::Owned { borrows: rb }) => {
                lb == rb
            }
            (ResourceState::ActiveBorrow, ResourceState::ActiveBorrow) => true,
            (ResourceState::Consumed { .. }, ResourceState::Consumed { .. }) => true,
            _ => false,
        }
    }
    pub(super) fn use_var(&mut self, name: &str, span: Span) -> Result<()> {
        if let Some(tracked) = self.states.get(name) {
            if let ResourceState::Consumed { span: consumed_at } = tracked.state {
                return Err(TyperError::resource_use_after_consume(name, consumed_at, span).into());
            }
        }
        Ok(())
    }
    pub(super) fn consume_var(&mut self, name: &str, span: Span) -> Result<()> {
        if let Some(tracked) = self.states.get_mut(name) {
            let mark_consumed = match &mut tracked.state {
                ResourceState::Owned { borrows } => {
                    if *borrows > 0 {
                        return Err(TyperError::resource_consume_borrow(name, span).into());
                    }
                    true
                }
                ResourceState::ActiveBorrow => {
                    return Err(TyperError::resource_consume_borrow(name, span).into());
                }
                ResourceState::Consumed { span: first } => {
                    return Err(TyperError::resource_double_consume(name, *first, span).into());
                }
            };
            if mark_consumed {
                tracked.state = ResourceState::Consumed { span };
            }
        }
        Ok(())
    }
    pub(super) fn borrow_var(&mut self, name: &str, span: Span) -> Result<()> {
        if let Some(tracked) = self.states.get_mut(name) {
            match &mut tracked.state {
                ResourceState::Owned { borrows } => {
                    *borrows += 1;
                }
                ResourceState::ActiveBorrow => {}
                ResourceState::Consumed { span: consumed_at } => {
                    return Err(
                        TyperError::resource_use_after_consume(name, *consumed_at, span).into(),
                    );
                }
            }
        }
        Ok(())
    }
    pub(super) fn release_borrow(&mut self, name: &str) -> Result<()> {
        if let Some(tracked) = self.states.get_mut(name) {
            match &mut tracked.state {
                ResourceState::Owned { borrows } => {
                    if *borrows == 0 {
                        bail!("release_borrow called without active borrow for {}", name);
                    }
                    *borrows -= 1;
                }
                ResourceState::ActiveBorrow => {}
                ResourceState::Consumed { .. } => {}
            }
        }
        Ok(())
    }
    pub(super) fn ensure_consumed(&self) -> Result<()> {
        for (name, tracked) in &self.states {
            if tracked.must_consume {
                match &tracked.state {
                    ResourceState::Consumed { .. } => {}
                    ResourceState::Owned { .. } | ResourceState::ActiveBorrow => {
                        return Err(TyperError::resource_not_consumed(name).into());
                    }
                }
            }
            if let ResourceState::Owned { borrows } = &tracked.state {
                debug_assert_eq!(
                    *borrows, 0,
                    "resource {} has outstanding borrows at scope end",
                    name
                );
            }
        }
        Ok(())
    }
}
pub(super) fn consume_var_expr(tracker: &mut ResourceTracker, expr: &Expr) -> Result<()> {
    if let Expr::Var(name, span) = expr {
        tracker.consume_var(name, *span)?;
    }
    Ok(())
}
pub(super) fn type_of<'a>(
    e: &'a Expr,
    env: &HashMap<&'a str, LocalBinding>,
    tracker: &mut ResourceTracker,
    fns: &HashMap<&'a str, FnSig>,
    aliases: &AliasMap,
    depth: usize,
) -> Result<Type> {
    if depth > 1024 {
        return Err(TyperError::new(
            "T011",
            "type-check recursion limit exceeded".to_string(),
            0,
            0,
        )
        .into());
    }
    match e {
        Expr::Int(_, _) => Ok(Type::Int),
        Expr::Bool(_, _) => Ok(Type::Bool),
        Expr::String(_, _) => Ok(Type::String),
        Expr::ArrayLit { elems, span } => {
            let mut local_tracker = tracker.clone();
            let first = elems
                .first()
                .ok_or_else(|| anyhow::anyhow!("array literal must have at least one element"))?;
            let elem_ty = type_of(first, env, &mut local_tracker, fns, aliases, depth + 1)?;
            for elem in elems.iter().skip(1) {
                let ety = type_of(elem, env, &mut local_tracker, fns, aliases, depth + 1)?;
                if !binding_compatible(&elem_ty, &ety, aliases)? {
                    if literal_can_coerce_unsigned(&elem_ty, &ety, elem) {
                        continue;
                    }
                    if let Some(err) = unsigned_literal_range_error(&elem_ty, &ety, elem) {
                        return Err(err.into());
                    }
                    let sp = expr_span(elem);
                    if refinement_loss(&elem_ty, &ety, aliases) {
                        return Err(TyperError::refinement_loss(elem_ty.clone(), ety, sp).into());
                    }
                    return Err(TyperError::element_type_mismatch(elem_ty.clone(), ety, sp).into());
                }
            }
            let arr_ty = Type::Array(Box::new(elem_ty), elems.len() as u32);
            if let Some(offending) = find_resource_collection(&arr_ty) {
                return Err(TyperError::resource_in_collection(offending, Some(*span)).into());
            }
            *tracker = local_tracker;
            Ok(arr_ty)
        }
        Expr::TupleLit { elems, span } => {
            let mut local_tracker = tracker.clone();
            let mut elem_tys = Vec::with_capacity(elems.len());
            for elem in elems {
                elem_tys.push(type_of(
                    elem,
                    env,
                    &mut local_tracker,
                    fns,
                    aliases,
                    depth + 1,
                )?);
            }
            let tuple_ty = Type::Tuple(elem_tys);
            if let Some(offending) = find_resource_collection(&tuple_ty) {
                return Err(TyperError::resource_in_collection(offending, Some(*span)).into());
            }
            *tracker = local_tracker;
            Ok(tuple_ty)
        }
        Expr::StructLit { span, .. } => {
            Err(TyperError::feature_not_supported("struct literals", *span).into())
        }
        Expr::FieldAccess { span, .. } => {
            Err(TyperError::feature_not_supported("field access", *span).into())
        }
        Expr::Index { base, index, span } => {
            let mut local_tracker = tracker.clone();
            let base_ty = type_of(base, env, &mut local_tracker, fns, aliases, depth + 1)?;
            let idx_ty = type_of(index, env, &mut local_tracker, fns, aliases, depth + 1)?;
            ensure_int(idx_ty, aliases, "index", Some(expr_span(index)))?;
            let resolved = base_type(&base_ty, aliases)?;
            match resolved {
                Type::Array(inner, len) => {
                    if let Some(idx) = int_literal_value(index) {
                        if idx < 0 || idx as u64 >= len as u64 {
                            return Err(
                                TyperError::array_index_out_of_bounds(expr_span(index)).into()
                            );
                        }
                    }
                    *tracker = local_tracker;
                    Ok(*inner)
                }
                Type::Tuple(elems) => {
                    let Some(idx) = int_literal_value(index) else {
                        return Err(
                            TyperError::tuple_index_requires_constant(expr_span(index)).into()
                        );
                    };
                    if idx < 0 || idx as usize >= elems.len() {
                        return Err(TyperError::array_index_out_of_bounds(expr_span(index)).into());
                    }
                    *tracker = local_tracker;
                    Ok(elems[idx as usize].clone())
                }
                other => {
                    Err(TyperError::expected_collection("array or tuple", other, *span).into())
                }
            }
        }
        Expr::Block { block } => type_block(block, env, tracker, fns, aliases, depth + 1),
        Expr::If {
            cond,
            then_br,
            else_br,
            span,
        } => {
            let cty = type_of(cond, env, tracker, fns, aliases, depth + 1)?;
            if base_type(&cty, aliases)? != Type::Bool {
                // Reuse arg_type_mismatch with pseudo-callee `if` for stable code T003
                return Err(TyperError::arg_type_mismatch(1, "if", Type::Bool, cty, *span).into());
            }
            let baseline = tracker.clone();
            let mut then_tracker = baseline.clone();
            let tty = type_of(then_br, env, &mut then_tracker, fns, aliases, depth + 1)?;
            let mut else_tracker = baseline.clone();
            let ety = type_of(else_br, env, &mut else_tracker, fns, aliases, depth + 1)?;
            if tty != ety {
                return Err(TyperError::branch_type_mismatch(tty, ety, *span).into());
            }
            then_tracker.retain_keys_from(&baseline);
            else_tracker.retain_keys_from(&baseline);
            then_tracker.merge_branch(&else_tracker, *span)?;
            *tracker = then_tracker;
            Ok(tty)
        }
        Expr::Match {
            scrutinee,
            arms,
            span,
        } => {
            let scrut_ty = type_of(scrutinee, env, tracker, fns, aliases, depth + 1)?;
            use clg_ast::MatchPat;
            match scrut_ty.clone() {
                Type::Option(inner_ty) => {
                    let baseline = tracker.clone();
                    let mut seen_some = false;
                    let mut seen_none = false;
                    let mut res_ty_opt: Option<Type> = None;
                    let mut branch_trackers: Vec<(ResourceTracker, Span)> = Vec::new();
                    for arm in arms {
                        match &arm.pat {
                            MatchPat::Some(name) => {
                                if seen_some {
                                    return Err(
                                        TyperError::match_duplicate_arm("Some", *span).into()
                                    );
                                }
                                seen_some = true;
                                if env.contains_key(name.as_str()) {
                                    return Err(TyperError::binder_conflict(name, *span).into());
                                }
                                let mut env2 = env.clone();
                                env2.insert(
                                    name.as_str(),
                                    LocalBinding {
                                        ty: *inner_ty.clone(),
                                        kind: ParamKind::Borrow,
                                    },
                                );
                                let mut arm_tracker = baseline.clone();
                                let at = type_of(
                                    &arm.expr,
                                    &env2,
                                    &mut arm_tracker,
                                    fns,
                                    aliases,
                                    depth + 1,
                                )?;
                                branch_trackers.push((arm_tracker, expr_span(&arm.expr)));
                                if let Some(rt) = &res_ty_opt {
                                    if &at != rt {
                                        let sp = expr_span(&arm.expr);
                                        return Err(TyperError::match_arm_type_mismatch(
                                            rt.clone(),
                                            at,
                                            sp,
                                        )
                                        .into());
                                    }
                                } else {
                                    res_ty_opt = Some(at);
                                }
                            }
                            MatchPat::None => {
                                if seen_none {
                                    return Err(
                                        TyperError::match_duplicate_arm("None", *span).into()
                                    );
                                }
                                seen_none = true;
                                let mut arm_tracker = baseline.clone();
                                let at = type_of(
                                    &arm.expr,
                                    env,
                                    &mut arm_tracker,
                                    fns,
                                    aliases,
                                    depth + 1,
                                )?;
                                branch_trackers.push((arm_tracker, expr_span(&arm.expr)));
                                if let Some(rt) = &res_ty_opt {
                                    if &at != rt {
                                        let sp = expr_span(&arm.expr);
                                        return Err(TyperError::match_arm_type_mismatch(
                                            rt.clone(),
                                            at,
                                            sp,
                                        )
                                        .into());
                                    }
                                } else {
                                    res_ty_opt = Some(at);
                                }
                            }
                            MatchPat::Ok(_) | MatchPat::Err(_) => {
                                return Err(TyperError::match_invalid_scrutinee(
                                    scrut_ty.clone(),
                                    *span,
                                )
                                .into());
                            }
                        }
                    }
                    if !(seen_some && seen_none) {
                        return Err(TyperError::match_non_exhaustive(*span).into());
                    }
                    let result_ty = res_ty_opt.expect("match arms must not be empty");
                    if let Some((first_tracker, _)) = branch_trackers.first() {
                        let mut merged = first_tracker.clone();
                        merged.retain_keys_from(&baseline);
                        for (branch_tracker, branch_span) in branch_trackers.iter().skip(1) {
                            let mut filtered = branch_tracker.clone();
                            filtered.retain_keys_from(&baseline);
                            merged.merge_branch(&filtered, *branch_span)?;
                        }
                        *tracker = merged;
                    } else {
                        *tracker = baseline;
                    }
                    Ok(result_ty)
                }
                Type::Result(ok_ty, err_ty) => {
                    let baseline = tracker.clone();
                    let mut seen_ok = false;
                    let mut seen_err = false;
                    let mut res_ty_opt: Option<Type> = None;
                    let mut branch_trackers: Vec<(ResourceTracker, Span)> = Vec::new();
                    for arm in arms {
                        match &arm.pat {
                            MatchPat::Ok(name) => {
                                if seen_ok {
                                    return Err(TyperError::match_duplicate_arm("Ok", *span).into());
                                }
                                seen_ok = true;
                                if env.contains_key(name.as_str()) {
                                    return Err(TyperError::binder_conflict(name, *span).into());
                                }
                                let mut env2 = env.clone();
                                env2.insert(
                                    name.as_str(),
                                    LocalBinding {
                                        ty: *ok_ty.clone(),
                                        kind: ParamKind::Borrow,
                                    },
                                );
                                let mut arm_tracker = baseline.clone();
                                let at = type_of(
                                    &arm.expr,
                                    &env2,
                                    &mut arm_tracker,
                                    fns,
                                    aliases,
                                    depth + 1,
                                )?;
                                branch_trackers.push((arm_tracker, expr_span(&arm.expr)));
                                if let Some(rt) = &res_ty_opt {
                                    if &at != rt {
                                        let sp = expr_span(&arm.expr);
                                        return Err(TyperError::match_arm_type_mismatch(
                                            rt.clone(),
                                            at,
                                            sp,
                                        )
                                        .into());
                                    }
                                } else {
                                    res_ty_opt = Some(at);
                                }
                            }
                            MatchPat::Err(name) => {
                                if seen_err {
                                    return Err(
                                        TyperError::match_duplicate_arm("Err", *span).into()
                                    );
                                }
                                seen_err = true;
                                if env.contains_key(name.as_str()) {
                                    return Err(TyperError::binder_conflict(name, *span).into());
                                }
                                let mut env2 = env.clone();
                                env2.insert(
                                    name.as_str(),
                                    LocalBinding {
                                        ty: *err_ty.clone(),
                                        kind: ParamKind::Borrow,
                                    },
                                );
                                let mut arm_tracker = baseline.clone();
                                let at = type_of(
                                    &arm.expr,
                                    &env2,
                                    &mut arm_tracker,
                                    fns,
                                    aliases,
                                    depth + 1,
                                )?;
                                branch_trackers.push((arm_tracker, expr_span(&arm.expr)));
                                if let Some(rt) = &res_ty_opt {
                                    if &at != rt {
                                        let sp = expr_span(&arm.expr);
                                        return Err(TyperError::match_arm_type_mismatch(
                                            rt.clone(),
                                            at,
                                            sp,
                                        )
                                        .into());
                                    }
                                } else {
                                    res_ty_opt = Some(at);
                                }
                            }
                            MatchPat::Some(_) | MatchPat::None => {
                                return Err(TyperError::match_invalid_scrutinee(
                                    scrut_ty.clone(),
                                    *span,
                                )
                                .into());
                            }
                        }
                    }
                    if !(seen_ok && seen_err) {
                        return Err(TyperError::match_non_exhaustive(*span).into());
                    }
                    let result_ty = res_ty_opt.expect("match arms must not be empty");
                    if let Some((first_tracker, _)) = branch_trackers.first() {
                        let mut merged = first_tracker.clone();
                        merged.retain_keys_from(&baseline);
                        for (branch_tracker, branch_span) in branch_trackers.iter().skip(1) {
                            let mut filtered = branch_tracker.clone();
                            filtered.retain_keys_from(&baseline);
                            merged.merge_branch(&filtered, *branch_span)?;
                        }
                        *tracker = merged;
                    } else {
                        *tracker = baseline;
                    }
                    Ok(result_ty)
                }
                other => Err(TyperError::match_invalid_scrutinee(other, *span).into()),
            }
        }
        Expr::Try { expr, span } => {
            let inner = type_of(expr, env, tracker, fns, aliases, depth + 1)?;
            let ret_binding = env
                .get(RETURN_KEY)
                .cloned()
                .ok_or_else(|| TyperError::try_missing_return(*span))?;
            let ret_ty = ret_binding.ty;
            match (inner, ret_ty) {
                (Type::Option(inner_ty), Type::Option(ret_inner)) => {
                    let found = (*inner_ty).clone();
                    let declared = (*ret_inner).clone();
                    if found != declared {
                        return Err(
                            TyperError::try_option_inner_mismatch(declared, found, *span).into(),
                        );
                    }
                    Ok(found)
                }
                (Type::Option(_), ret_other) => {
                    Err(TyperError::try_option_return_required(ret_other, *span).into())
                }
                (Type::Result(ok_ty, err_ty), Type::Result(ret_ok, ret_err)) => {
                    let ok_found = (*ok_ty).clone();
                    let err_found = (*err_ty).clone();
                    let ok_decl = (*ret_ok).clone();
                    let err_decl = (*ret_err).clone();
                    if ok_found != ok_decl || err_found != err_decl {
                        return Err(TyperError::try_result_mismatch(
                            ok_decl, err_decl, ok_found, err_found, *span,
                        )
                        .into());
                    }
                    Ok(ok_found)
                }
                (Type::Result(_, _), ret_other) => {
                    Err(TyperError::try_result_return_required(ret_other, *span).into())
                }
                (other, _) => Err(TyperError::try_input_not_option_result(other, *span).into()),
            }
        }
        Expr::Var(name, sp) => {
            tracker.use_var(name, *sp)?;
            match env.get(name.as_str()) {
                Some(binding) => Ok(binding.ty.clone()),
                None => {
                    if name == "result" {
                        if let Some(binding) = env.get(RETURN_KEY) {
                            return Ok(binding.ty.clone());
                        }
                    }
                    Err(TyperError::unknown_variable(name, *sp).into())
                }
            }
        }
        Expr::Return { expr, .. } => {
            let ty = type_of(expr, env, tracker, fns, aliases, depth + 1)?;
            if is_resource_type(&ty, aliases)? {
                consume_var_expr(tracker, expr.as_ref())?;
            }
            Ok(ty)
        }
        Expr::Unary { op, expr, span } => {
            let inner = type_of(expr, env, tracker, fns, aliases, depth + 1)?;
            match op {
                UnaryOp::Not => {
                    ensure_bool(inner, aliases, "operand", Some(*span))?;
                    Ok(Type::Bool)
                }
            }
        }
        Expr::Bin { op, lhs, rhs, span } => {
            let lt = type_of(lhs, env, tracker, fns, aliases, depth + 1)?;
            let rt = type_of(rhs, env, tracker, fns, aliases, depth + 1)?;
            match op {
                BinOp::Add | BinOp::Sub | BinOp::Mul | BinOp::Div => {
                    let op_str = match op {
                        BinOp::Add => "+",
                        BinOp::Sub => "-",
                        BinOp::Mul => "*",
                        BinOp::Div => "/",
                        _ => "?",
                    };
                    let lt_base = base_type(&lt, aliases)?;
                    let rt_base = base_type(&rt, aliases)?;
                    if matches!(lt_base, Type::U128 | Type::U256) {
                        return Err(
                            TyperError::unsigned_int_not_supported(lt_base, Some(*span)).into()
                        );
                    }
                    if matches!(rt_base, Type::U128 | Type::U256) {
                        return Err(
                            TyperError::unsigned_int_not_supported(rt_base, Some(*span)).into()
                        );
                    }
                    if matches!(lt_base, Type::U64) || matches!(rt_base, Type::U64) {
                        if let (Some(lv), Some(rv)) =
                            (unsigned_literal_value(lhs), unsigned_literal_value(rhs))
                        {
                            if u64_literal_overflow(op, lv, rv) {
                                return Err(
                                    TyperError::unsigned_constant_overflow(op_str, *span).into()
                                );
                            }
                        }
                    }
                    let lt_is_int = matches!(lt_base, Type::Int);
                    let rt_is_int = matches!(rt_base, Type::Int);
                    let lt_is_u64 = matches!(lt_base, Type::U64);
                    let rt_is_u64 = matches!(rt_base, Type::U64);
                    if lt_is_int && !rt_is_int && !rt_is_u64 {
                        return Err(
                            TyperError::int_operand("right operand", rt, Some(*span)).into()
                        );
                    }
                    if rt_is_int && !lt_is_int && !lt_is_u64 {
                        return Err(TyperError::int_operand("left operand", lt, Some(*span)).into());
                    }
                    match (&lt_base, &rt_base) {
                        (Type::U64, Type::U64) => Ok(Type::U64),
                        (Type::U64, Type::Int) if unsigned_literal_value(rhs).is_some() => {
                            Ok(Type::U64)
                        }
                        (Type::Int, Type::U64) if unsigned_literal_value(lhs).is_some() => {
                            Ok(Type::U64)
                        }
                        (Type::Int, Type::Int) => Ok(Type::Int),
                        _ => {
                            Err(TyperError::binary_operands_mismatch(op_str, lt, rt, *span).into())
                        }
                    }
                }
                BinOp::BitAnd | BinOp::BitOr | BinOp::BitXor => {
                    let op_str = match op {
                        BinOp::BitAnd => "&",
                        BinOp::BitOr => "|",
                        BinOp::BitXor => "^",
                        _ => "?",
                    };
                    let lt_base = base_type(&lt, aliases)?;
                    let rt_base = base_type(&rt, aliases)?;
                    if matches!(lt_base, Type::U128 | Type::U256) {
                        return Err(
                            TyperError::unsigned_int_not_supported(lt_base, Some(*span)).into()
                        );
                    }
                    if matches!(rt_base, Type::U128 | Type::U256) {
                        return Err(
                            TyperError::unsigned_int_not_supported(rt_base, Some(*span)).into()
                        );
                    }
                    match (&lt_base, &rt_base) {
                        (Type::U64, Type::U64) => Ok(Type::U64),
                        (Type::U64, Type::Int) if unsigned_literal_value(rhs).is_some() => {
                            Ok(Type::U64)
                        }
                        (Type::Int, Type::U64) if unsigned_literal_value(lhs).is_some() => {
                            Ok(Type::U64)
                        }
                        (Type::Int, Type::Int) => Ok(Type::Int),
                        _ => {
                            Err(TyperError::binary_operands_mismatch(op_str, lt, rt, *span).into())
                        }
                    }
                }
                BinOp::Shl | BinOp::Shr => {
                    let op_str = match op {
                        BinOp::Shl => "<<",
                        BinOp::Shr => ">>",
                        _ => "?",
                    };
                    let lt_base = base_type(&lt, aliases)?;
                    let rt_base = base_type(&rt, aliases)?;
                    if matches!(lt_base, Type::U128 | Type::U256) {
                        return Err(
                            TyperError::unsigned_int_not_supported(lt_base, Some(*span)).into()
                        );
                    }
                    if matches!(rt_base, Type::U128 | Type::U256) {
                        return Err(
                            TyperError::unsigned_int_not_supported(rt_base, Some(*span)).into()
                        );
                    }
                    match (&lt_base, &rt_base) {
                        (Type::Int, Type::Int) => Ok(Type::Int),
                        (Type::U64, Type::U64) => Ok(Type::U64),
                        (Type::U64, Type::Int) if unsigned_literal_value(rhs).is_some() => {
                            Ok(Type::U64)
                        }
                        _ => {
                            Err(TyperError::binary_operands_mismatch(op_str, lt, rt, *span).into())
                        }
                    }
                }
                BinOp::Lt | BinOp::Le | BinOp::Gt | BinOp::Ge => {
                    let op_str = match op {
                        BinOp::Lt => "<",
                        BinOp::Le => "<=",
                        BinOp::Gt => ">",
                        BinOp::Ge => ">=",
                        _ => "?",
                    };
                    let lt_base = base_type(&lt, aliases)?;
                    let rt_base = base_type(&rt, aliases)?;
                    if matches!(lt_base, Type::U128 | Type::U256) {
                        return Err(
                            TyperError::unsigned_int_not_supported(lt_base, Some(*span)).into()
                        );
                    }
                    if matches!(rt_base, Type::U128 | Type::U256) {
                        return Err(
                            TyperError::unsigned_int_not_supported(rt_base, Some(*span)).into()
                        );
                    }
                    match (&lt_base, &rt_base) {
                        (Type::U64, Type::U64) => Ok(Type::Bool),
                        (Type::U64, Type::Int) if unsigned_literal_value(rhs).is_some() => {
                            Ok(Type::Bool)
                        }
                        (Type::Int, Type::U64) if unsigned_literal_value(lhs).is_some() => {
                            Ok(Type::Bool)
                        }
                        (Type::Int, Type::Int) => Ok(Type::Bool),
                        _ => {
                            Err(TyperError::binary_operands_mismatch(op_str, lt, rt, *span).into())
                        }
                    }
                }
                BinOp::Eq | BinOp::Neq => {
                    let lt_base = base_type(&lt, aliases)?;
                    let rt_base = base_type(&rt, aliases)?;
                    if matches!(lt_base, Type::U128 | Type::U256) {
                        return Err(
                            TyperError::unsigned_int_not_supported(lt_base, Some(*span)).into()
                        );
                    }
                    if matches!(rt_base, Type::U128 | Type::U256) {
                        return Err(
                            TyperError::unsigned_int_not_supported(rt_base, Some(*span)).into()
                        );
                    }
                    if !base_types_match(&lt, &rt, aliases)?
                        && !literal_can_coerce_unsigned(&lt, &rt, rhs)
                        && !literal_can_coerce_unsigned(&rt, &lt, lhs)
                    {
                        if let Some(err) = unsigned_literal_range_error(&lt, &rt, rhs) {
                            return Err(err.into());
                        }
                        if let Some(err) = unsigned_literal_range_error(&rt, &lt, lhs) {
                            return Err(err.into());
                        }
                        let op_str = if *op == BinOp::Eq { "==" } else { "!=" };
                        return Err(
                            TyperError::binary_operands_mismatch(op_str, lt, rt, *span).into()
                        );
                    }
                    Ok(Type::Bool)
                }
                BinOp::And | BinOp::Or => {
                    ensure_bool(lt, aliases, "left operand", Some(*span))?;
                    ensure_bool(rt, aliases, "right operand", Some(*span))?;
                    Ok(Type::Bool)
                }
            }
        }
        Expr::Call { callee, args, span } => {
            // Phase 4.6 - Collections signatures (type-only)
            if let Some(t) =
                type_collection_call(callee, args, env, tracker, fns, aliases, depth, *span)?
            {
                return Ok(t);
            }
            if callee == "U8" || callee == "U64" || callee == "U128" || callee == "U256" {
                if args.len() != 1 {
                    return Err(TyperError::arity_mismatch(callee, 1, args.len(), *span).into());
                }
                let target_ty = match callee.as_str() {
                    "U8" => Type::U8,
                    "U64" => Type::U64,
                    "U128" => Type::U128,
                    "U256" => Type::U256,
                    _ => Type::U64,
                };
                let mut local_tracker = tracker.clone();
                let arg_ty = type_of(&args[0], env, &mut local_tracker, fns, aliases, depth + 1)?;
                if arg_ty != target_ty {
                    if matches!(arg_ty, Type::Int) {
                        if let Some(value) = unsigned_literal_value(&args[0]) {
                            if !unsigned_literal_fits_value(&target_ty, value) {
                                let sp = expr_span(&args[0]);
                                return Err(TyperError::unsigned_literal_out_of_range(
                                    target_ty.clone(),
                                    value,
                                    sp,
                                )
                                .into());
                            }
                        } else {
                            let sp = expr_span(&args[0]);
                            return Err(
                                TyperError::unsigned_cast_invalid(callee, arg_ty, sp).into()
                            );
                        }
                    } else {
                        let sp = expr_span(&args[0]);
                        return Err(TyperError::unsigned_cast_invalid(callee, arg_ty, sp).into());
                    }
                }
                *tracker = local_tracker;
                return Ok(target_ty);
            }
            // Phase 4.5 - ADT constructors (partial): Some(T) infers Option<T>
            if callee == "Some" {
                if args.len() != 1 {
                    return Err(TyperError::arity_mismatch(callee, 1, args.len(), *span).into());
                }
                let mut local_tracker = tracker.clone();
                let t0 = type_of(&args[0], env, &mut local_tracker, fns, aliases, depth + 1)?;
                *tracker = local_tracker;
                return Ok(Type::Option(Box::new(t0)));
            }
            if callee == "None" {
                if !args.is_empty() {
                    return Err(TyperError::arity_mismatch(callee, 0, args.len(), *span).into());
                }
                let ret_binding = env
                    .get(RETURN_KEY)
                    .cloned()
                    .ok_or_else(|| TyperError::option_ctor_missing_return(*span))?;
                return match ret_binding.ty {
                    Type::Option(inner) => Ok(Type::Option(inner)),
                    other => Err(TyperError::none_return_required(other, *span).into()),
                };
            }
            if callee == "Ok" {
                if args.len() != 1 {
                    return Err(TyperError::arity_mismatch(callee, 1, args.len(), *span).into());
                }
                let mut local_tracker = tracker.clone();
                let arg_ty = type_of(&args[0], env, &mut local_tracker, fns, aliases, depth + 1)?;
                let ret_binding = env
                    .get(RETURN_KEY)
                    .cloned()
                    .ok_or_else(|| TyperError::result_ctor_missing_return(*span))?;
                let result_ty = match ret_binding.ty {
                    Type::Result(ok_ty, err_ty) => {
                        if arg_ty != *ok_ty {
                            let sp = expr_span(&args[0]);
                            Err(
                                TyperError::ok_argument_mismatch((*ok_ty).clone(), arg_ty, sp)
                                    .into(),
                            )
                        } else {
                            Ok(Type::Result(ok_ty, err_ty))
                        }
                    }
                    other => Err(TyperError::ok_return_required(other, *span).into()),
                };
                if result_ty.is_ok() {
                    *tracker = local_tracker;
                }
                return result_ty;
            }
            if callee == "Err" {
                if args.len() != 1 {
                    return Err(TyperError::arity_mismatch(callee, 1, args.len(), *span).into());
                }
                let mut local_tracker = tracker.clone();
                let arg_ty = type_of(&args[0], env, &mut local_tracker, fns, aliases, depth + 1)?;
                let ret_binding = env
                    .get(RETURN_KEY)
                    .cloned()
                    .ok_or_else(|| TyperError::result_ctor_missing_return(*span))?;
                let result_ty = match ret_binding.ty {
                    Type::Result(ok_ty, err_ty) => {
                        if arg_ty != *err_ty {
                            let sp = expr_span(&args[0]);
                            Err(
                                TyperError::err_argument_mismatch((*err_ty).clone(), arg_ty, sp)
                                    .into(),
                            )
                        } else {
                            Ok(Type::Result(ok_ty, err_ty))
                        }
                    }
                    other => Err(TyperError::err_return_required(other, *span).into()),
                };
                if result_ty.is_ok() {
                    *tracker = local_tracker;
                }
                return result_ty;
            }
            let FnSig { params, ret, .. } = fns
                .get(callee.as_str())
                .cloned()
                .ok_or_else(|| TyperError::unknown_function(callee, *span))?;
            if params.len() != args.len() {
                return Err(
                    TyperError::arity_mismatch(callee, params.len(), args.len(), *span).into(),
                );
            }
            let mut local_tracker = tracker.clone();
            let mut borrowed: Vec<String> = Vec::new();
            for (i, (p, a)) in params.iter().zip(args.iter()).enumerate() {
                let at = type_of(a, env, &mut local_tracker, fns, aliases, depth + 1)?;
                let expected = p.ty.clone();
                if !base_types_match(&expected, &at, aliases)? {
                    if literal_can_coerce_unsigned(&expected, &at, a) {
                        continue;
                    }
                    if let Some(err) = unsigned_literal_range_error(&expected, &at, a) {
                        return Err(err.into());
                    }
                    let sp = expr_span(a);
                    return Err(TyperError::arg_type_mismatch(i, callee, expected, at, sp).into());
                }
                if !binding_compatible(&expected, &at, aliases)? {
                    if literal_can_coerce_unsigned(&expected, &at, a) {
                        continue;
                    }
                    if let Some(err) = unsigned_literal_range_error(&expected, &at, a) {
                        return Err(err.into());
                    }
                    let sp = expr_span(a);
                    if refinement_loss(&expected, &at, aliases) {
                        return Err(TyperError::refinement_loss(expected, at, sp).into());
                    }
                    return Err(TyperError::arg_type_mismatch(i, callee, expected, at, sp).into());
                }
                if is_resource_type(&expected, aliases)? {
                    match p.kind {
                        ParamKind::Consume => {
                            if let Expr::Var(arg_name, arg_span) = a {
                                local_tracker.consume_var(arg_name, *arg_span)?;
                            }
                        }
                        ParamKind::Borrow => {
                            if let Expr::Var(arg_name, arg_span) = a {
                                local_tracker.borrow_var(arg_name, *arg_span)?;
                                borrowed.push(arg_name.clone());
                            }
                        }
                    }
                }
            }
            for name in borrowed {
                local_tracker.release_borrow(&name)?;
            }
            *tracker = local_tracker;
            Ok(ret)
        }
    }
}

pub(crate) fn infer_expr_type<'a>(
    e: &'a Expr,
    env: &HashMap<&'a str, LocalBinding>,
    fns: &HashMap<&'a str, FnSig>,
    aliases: &AliasMap,
) -> Result<Type> {
    let mut tracker = ResourceTracker::new();
    type_of(e, env, &mut tracker, fns, aliases, 0)
}
#[allow(clippy::too_many_arguments)]
fn type_collection_call<'a>(
    callee: &str,
    args: &'a [Expr],
    env: &HashMap<&'a str, LocalBinding>,
    tracker: &mut ResourceTracker,
    fns: &HashMap<&'a str, FnSig>,
    aliases: &AliasMap,
    depth: usize,
    span: Span,
) -> Result<Option<Type>> {
    // Helper to get type of an expression
    let arg_ty = |i: usize| -> Result<Type> {
        let mut tmp = tracker.clone();
        type_of(&args[i], env, &mut tmp, fns, aliases, depth + 1)
    };
    let normalized_callee = match callee {
        "std::list::push_mut" => "std::list::push",
        "std::list::insert_mut" => "std::list::insert",
        "std::list::remove_mut" => "std::list::remove",
        "std::list::pop_mut" => "std::list::pop",
        "std::set::insert_mut" => "std::set::insert",
        "std::set::remove_mut" => "std::set::remove",
        "std::map::insert_mut" => "std::map::insert",
        "std::map::remove_mut" => "std::map::remove",
        other => other,
    };
    match normalized_callee {
        // List
        "std::list::len" => {
            if args.len() != 1 {
                return Err(TyperError::arity_mismatch(callee, 1, args.len(), span).into());
            }
            let lty = arg_ty(0)?;
            if let Type::List(_) = lty {
                Ok(Some(Type::Int))
            } else {
                Err(TyperError::expected_collection("List", lty, span).into())
            }
        }
        "std::list::can_mut" => {
            if args.len() != 1 {
                return Err(TyperError::arity_mismatch(callee, 1, args.len(), span).into());
            }
            let lty = arg_ty(0)?;
            if let Type::List(_) = lty {
                Ok(Some(Type::Bool))
            } else {
                Err(TyperError::expected_collection("List", lty, span).into())
            }
        }
        "std::list::get" => {
            if args.len() != 2 {
                return Err(TyperError::arity_mismatch(callee, 2, args.len(), span).into());
            }
            let lty = arg_ty(0)?;
            let ity = arg_ty(1)?;
            let sp = expr_span(&args[1]);
            ensure_int(ity, aliases, "index", Some(sp))?;
            match lty {
                Type::List(inner) => Ok(Some(Type::Option(inner))),
                other => Err(TyperError::expected_collection("List", other, span).into()),
            }
        }
        "std::list::push" => {
            if args.len() != 2 {
                return Err(TyperError::arity_mismatch(callee, 2, args.len(), span).into());
            }
            let lty = arg_ty(0)?;
            match lty {
                Type::List(inner) => {
                    let letxty: Type = (*inner).clone();
                    let aty = arg_ty(1)?;
                    if aty != letxty {
                        let sp = expr_span(&args[1]);
                        return Err(TyperError::element_type_mismatch(letxty, aty, sp).into());
                    }
                    Ok(Some(Type::List(Box::new(*inner))))
                }
                other => Err(TyperError::expected_collection("List", other, span).into()),
            }
        }
        "std::list::insert" => {
            if args.len() != 3 {
                return Err(TyperError::arity_mismatch(callee, 3, args.len(), span).into());
            }
            let lty = arg_ty(0)?;
            match lty {
                Type::List(inner) => {
                    let elem_expected: Type = (*inner).clone();
                    let aty_elem = arg_ty(1)?;
                    if aty_elem != elem_expected {
                        let sp = expr_span(&args[1]);
                        return Err(
                            TyperError::element_type_mismatch(elem_expected, aty_elem, sp).into(),
                        );
                    }
                    let ity = arg_ty(2)?;
                    let sp = expr_span(&args[2]);
                    ensure_int(ity, aliases, "index", Some(sp))?;
                    Ok(Some(Type::List(Box::new(*inner))))
                }
                other => Err(TyperError::expected_collection("List", other, span).into()),
            }
        }
        "std::list::remove" => {
            if args.len() != 2 {
                return Err(TyperError::arity_mismatch(callee, 2, args.len(), span).into());
            }
            let lty = arg_ty(0)?;
            let ity = arg_ty(1)?;
            let sp = expr_span(&args[1]);
            ensure_int(ity, aliases, "index", Some(sp))?;
            match lty {
                Type::List(inner) => Ok(Some(Type::List(inner))),
                other => Err(TyperError::expected_collection("List", other, span).into()),
            }
        }
        "std::list::pop" => {
            if args.len() != 1 {
                return Err(TyperError::arity_mismatch(callee, 1, args.len(), span).into());
            }
            let lty = arg_ty(0)?;
            match lty {
                Type::List(inner) => Ok(Some(Type::Option(inner))),
                other => Err(TyperError::expected_collection("List", other, span).into()),
            }
        }
        "std::list::new" => Err(TyperError::cannot_infer_collection(span, "std::list").into()),
        // Set
        "std::set::len" => {
            if args.len() != 1 {
                return Err(TyperError::arity_mismatch(callee, 1, args.len(), span).into());
            }
            let sty = arg_ty(0)?;
            if let Type::Set(_) = sty {
                Ok(Some(Type::Int))
            } else {
                Err(TyperError::expected_collection("Set", sty, span).into())
            }
        }
        "std::set::can_mut" => {
            if args.len() != 1 {
                return Err(TyperError::arity_mismatch(callee, 1, args.len(), span).into());
            }
            let sty = arg_ty(0)?;
            if let Type::Set(_) = sty {
                Ok(Some(Type::Bool))
            } else {
                Err(TyperError::expected_collection("Set", sty, span).into())
            }
        }
        "std::set::contains" => {
            if args.len() != 2 {
                return Err(TyperError::arity_mismatch(callee, 2, args.len(), span).into());
            }
            match arg_ty(0)? {
                Type::Set(inner) => {
                    let aty = arg_ty(1)?;
                    if aty != *inner {
                        let sp = expr_span(&args[1]);
                        return Err(TyperError::element_type_mismatch(*inner, aty, sp).into());
                    }
                    Ok(Some(Type::Bool))
                }
                other => Err(TyperError::expected_collection("Set", other, span).into()),
            }
        }
        "std::set::insert" | "std::set::remove" => {
            if args.len() != 2 {
                return Err(TyperError::arity_mismatch(callee, 2, args.len(), span).into());
            }
            match arg_ty(0)? {
                Type::Set(inner) => {
                    let aty = arg_ty(1)?;
                    if aty != *inner {
                        let sp = expr_span(&args[1]);
                        return Err(TyperError::element_type_mismatch(*inner, aty, sp).into());
                    }
                    Ok(Some(Type::Set(inner)))
                }
                other => Err(TyperError::expected_collection("Set", other, span).into()),
            }
        }
        "std::set::new" => Err(TyperError::cannot_infer_collection(span, "std::set").into()),
        // Map
        "std::map::len" => {
            if args.len() != 1 {
                return Err(TyperError::arity_mismatch(callee, 1, args.len(), span).into());
            }
            let mty = arg_ty(0)?;
            if let Type::Map(_, _) = mty {
                Ok(Some(Type::Int))
            } else {
                Err(TyperError::expected_collection("Map", mty, span).into())
            }
        }
        "std::map::can_mut" => {
            if args.len() != 1 {
                return Err(TyperError::arity_mismatch(callee, 1, args.len(), span).into());
            }
            let mty = arg_ty(0)?;
            if let Type::Map(_, _) = mty {
                Ok(Some(Type::Bool))
            } else {
                Err(TyperError::expected_collection("Map", mty, span).into())
            }
        }
        "std::map::contains" => {
            if args.len() != 2 {
                return Err(TyperError::arity_mismatch(callee, 2, args.len(), span).into());
            }
            match arg_ty(0)? {
                Type::Map(k, _v) => {
                    let aty = arg_ty(1)?;
                    if aty != *k {
                        let sp = expr_span(&args[1]);
                        return Err(TyperError::element_type_mismatch(*k, aty, sp).into());
                    }
                    Ok(Some(Type::Bool))
                }
                other => Err(TyperError::expected_collection("Map", other, span).into()),
            }
        }
        "std::map::get" => {
            if args.len() != 2 {
                return Err(TyperError::arity_mismatch(callee, 2, args.len(), span).into());
            }
            match arg_ty(0)? {
                Type::Map(k, v) => {
                    let aty = arg_ty(1)?;
                    if aty != *k {
                        let sp = expr_span(&args[1]);
                        return Err(TyperError::element_type_mismatch(*k, aty, sp).into());
                    }
                    Ok(Some(Type::Option(v)))
                }
                other => Err(TyperError::expected_collection("Map", other, span).into()),
            }
        }
        "std::map::insert" => {
            if args.len() != 3 {
                return Err(TyperError::arity_mismatch(callee, 3, args.len(), span).into());
            }
            match arg_ty(0)? {
                Type::Map(k, v) => {
                    let aty_k = arg_ty(1)?;
                    if aty_k != *k {
                        let sp = expr_span(&args[1]);
                        return Err(TyperError::element_type_mismatch(*k, aty_k, sp).into());
                    }
                    let aty_v = arg_ty(2)?;
                    if aty_v != *v {
                        let sp = expr_span(&args[2]);
                        return Err(TyperError::element_type_mismatch(*v, aty_v, sp).into());
                    }
                    Ok(Some(Type::Map(k, v)))
                }
                other => Err(TyperError::expected_collection("Map", other, span).into()),
            }
        }
        "std::map::remove" => {
            if args.len() != 2 {
                return Err(TyperError::arity_mismatch(callee, 2, args.len(), span).into());
            }
            match arg_ty(0)? {
                Type::Map(k, v) => {
                    let aty = arg_ty(1)?;
                    if aty != *k {
                        let sp = expr_span(&args[1]);
                        return Err(TyperError::element_type_mismatch(*k, aty, sp).into());
                    }
                    Ok(Some(Type::Map(k, v)))
                }
                other => Err(TyperError::expected_collection("Map", other, span).into()),
            }
        }
        "std::map::new" => Err(TyperError::cannot_infer_collection(span, "std::map").into()),
        _ => Ok(None),
    }
}
fn type_block_stmt<'a>(
    block: &'a Block,
    env: &HashMap<&'a str, LocalBinding>,
    tracker: &mut ResourceTracker,
    fns: &HashMap<&'a str, FnSig>,
    aliases: &AliasMap,
    depth: usize,
) -> Result<()> {
    let mut inner_env = env.clone();
    let mut inner_tracker = tracker.clone();
    for stmt in &block.statements {
        match stmt {
            Stmt::Let { name, expr, .. } => {
                let ty = type_of(
                    expr.as_ref(),
                    &inner_env,
                    &mut inner_tracker,
                    fns,
                    aliases,
                    depth + 1,
                )?;
                if let Some(existing) = inner_env.get(name.as_str()) {
                    if base_types_match(&existing.ty, &ty, aliases)?
                        && refinement_loss(&ty, &existing.ty, aliases)
                    {
                        let sp = expr_span(expr.as_ref());
                        return Err(TyperError::refinement_loss(existing.ty.clone(), ty, sp).into());
                    }
                }
                if is_resource_type(&ty, aliases)? {
                    consume_var_expr(&mut inner_tracker, expr.as_ref())?;
                }
                let base_ty = base_type(&ty, aliases)?;
                inner_tracker.register_local(name.as_str(), &base_ty);
                inner_env.insert(
                    name.as_str(),
                    LocalBinding {
                        ty,
                        kind: ParamKind::Borrow,
                    },
                );
            }
            Stmt::While {
                cond,
                invariant,
                variant,
                body,
                span,
            } => {
                type_while_stmt(
                    cond.as_ref(),
                    invariant.as_ref(),
                    variant.as_ref().map(|v| v.as_ref()),
                    body.as_ref(),
                    &inner_env,
                    &mut inner_tracker,
                    fns,
                    aliases,
                    depth + 1,
                    *span,
                )?;
            }
            Stmt::Expr { expr, .. } => {
                type_of(
                    expr.as_ref(),
                    &inner_env,
                    &mut inner_tracker,
                    fns,
                    aliases,
                    depth + 1,
                )?;
            }
        }
    }
    if let Some(tail) = &block.tail {
        let ty = type_of(
            tail.as_ref(),
            &inner_env,
            &mut inner_tracker,
            fns,
            aliases,
            depth + 1,
        )?;
        if is_resource_type(&ty, aliases)? {
            consume_var_expr(&mut inner_tracker, tail.as_ref())?;
        }
    }
    *tracker = inner_tracker;
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn type_while_stmt<'a>(
    cond: &'a Expr,
    invariant: &'a Expr,
    variant: Option<&'a Expr>,
    body: &'a Block,
    env: &HashMap<&'a str, LocalBinding>,
    tracker: &mut ResourceTracker,
    fns: &HashMap<&'a str, FnSig>,
    aliases: &AliasMap,
    depth: usize,
    span: Span,
) -> Result<()> {
    let cty = type_of(cond, env, tracker, fns, aliases, depth)?;
    ensure_bool(cty, aliases, "condition", Some(expr_span(cond)))?;
    let inv_ty = type_of(invariant, env, tracker, fns, aliases, depth)?;
    ensure_bool(
        inv_ty,
        aliases,
        "loop invariant",
        Some(expr_span(invariant)),
    )?;
    if let Some(var_expr) = variant {
        let vty = type_of(var_expr, env, tracker, fns, aliases, depth)?;
        ensure_int(vty, aliases, "loop variant", Some(expr_span(var_expr)))?;
    }
    let baseline = tracker.clone();
    let mut body_tracker = baseline.clone();
    type_block_stmt(body, env, &mut body_tracker, fns, aliases, depth + 1)?;
    body_tracker.retain_keys_from(&baseline);
    body_tracker.merge_branch(&baseline, span)?;
    *tracker = baseline;
    Ok(())
}

fn type_block<'a>(
    block: &'a Block,
    env: &HashMap<&'a str, LocalBinding>,
    tracker: &mut ResourceTracker,
    fns: &HashMap<&'a str, FnSig>,
    aliases: &AliasMap,
    depth: usize,
) -> Result<Type> {
    let mut inner_env = env.clone();
    let mut inner_tracker = tracker.clone();
    for stmt in &block.statements {
        match stmt {
            Stmt::Let { name, expr, .. } => {
                let ty = type_of(
                    expr.as_ref(),
                    &inner_env,
                    &mut inner_tracker,
                    fns,
                    aliases,
                    depth + 1,
                )?;
                if let Some(existing) = inner_env.get(name.as_str()) {
                    if base_types_match(&existing.ty, &ty, aliases)?
                        && refinement_loss(&ty, &existing.ty, aliases)
                    {
                        let sp = expr_span(expr.as_ref());
                        return Err(TyperError::refinement_loss(existing.ty.clone(), ty, sp).into());
                    }
                }
                if is_resource_type(&ty, aliases)? {
                    consume_var_expr(&mut inner_tracker, expr.as_ref())?;
                }
                let base_ty = base_type(&ty, aliases)?;
                inner_tracker.register_local(name.as_str(), &base_ty);
                inner_env.insert(
                    name.as_str(),
                    LocalBinding {
                        ty,
                        kind: ParamKind::Borrow,
                    },
                );
            }
            Stmt::While {
                cond,
                invariant,
                variant,
                body,
                span,
            } => {
                let baseline = inner_tracker.clone();
                type_while_stmt(
                    cond.as_ref(),
                    invariant.as_ref(),
                    variant.as_ref().map(|v| v.as_ref()),
                    body.as_ref(),
                    &inner_env,
                    &mut inner_tracker,
                    fns,
                    aliases,
                    depth + 1,
                    *span,
                )?;
                inner_tracker = baseline;
            }
            Stmt::Expr { expr, .. } => {
                type_of(
                    expr.as_ref(),
                    &inner_env,
                    &mut inner_tracker,
                    fns,
                    aliases,
                    depth + 1,
                )?;
            }
        }
    }
    let result = if let Some(tail) = &block.tail {
        let ty = type_of(
            tail.as_ref(),
            &inner_env,
            &mut inner_tracker,
            fns,
            aliases,
            depth + 1,
        )?;
        if is_resource_type(&ty, aliases)? {
            consume_var_expr(&mut inner_tracker, tail.as_ref())?;
        }
        Ok(ty)
    } else {
        Err(TyperError::block_missing_tail(block.span).into())
    };
    *tracker = inner_tracker;
    result
}
pub(super) fn max_effect<'a>(
    e: &'a Expr,
    fns: &HashMap<&'a str, FnSig>,
    allowed: EffectLevel,
) -> Result<EffectLevel> {
    fn block_effect<'a>(
        block: &'a Block,
        fns: &HashMap<&'a str, FnSig>,
        allowed: EffectLevel,
    ) -> Result<EffectLevel> {
        let mut eff = EffectLevel::Pure;
        for stmt in &block.statements {
            match stmt {
                Stmt::Let { expr, .. } | Stmt::Expr { expr, .. } => {
                    eff = eff.join(max_effect(expr.as_ref(), fns, allowed)?);
                }
                Stmt::While {
                    cond,
                    invariant,
                    variant,
                    body,
                    ..
                } => {
                    eff = eff.join(max_effect(cond.as_ref(), fns, allowed)?);
                    eff = eff.join(max_effect(invariant.as_ref(), fns, allowed)?);
                    if let Some(v) = variant {
                        eff = eff.join(max_effect(v.as_ref(), fns, allowed)?);
                    }
                    eff = eff.join(block_effect(body.as_ref(), fns, allowed)?);
                }
            }
        }
        if let Some(tail) = &block.tail {
            eff = eff.join(max_effect(tail.as_ref(), fns, allowed)?);
        }
        Ok(eff)
    }
    match e {
        Expr::Block { block } => block_effect(block, fns, allowed),
        Expr::Int(_, _) | Expr::Bool(_, _) | Expr::String(_, _) | Expr::Var(_, _) => {
            Ok(EffectLevel::Pure)
        }
        Expr::Return { expr, .. } => max_effect(expr, fns, allowed),
        Expr::Unary { expr, .. } => max_effect(expr, fns, allowed),
        Expr::Bin { lhs, rhs, .. } => {
            let left = max_effect(lhs, fns, allowed)?;
            let right = max_effect(rhs, fns, allowed)?;
            Ok(left.join(right))
        }
        Expr::ArrayLit { elems, .. } | Expr::TupleLit { elems, .. } => {
            let mut eff = EffectLevel::Pure;
            for elem in elems {
                eff = eff.join(max_effect(elem, fns, allowed)?);
            }
            Ok(eff)
        }
        Expr::StructLit { fields, .. } => {
            let mut eff = EffectLevel::Pure;
            for field in fields {
                eff = eff.join(max_effect(&field.expr, fns, allowed)?);
            }
            Ok(eff)
        }
        Expr::FieldAccess { base, .. } => max_effect(base, fns, allowed),
        Expr::Index { base, index, .. } => {
            let base_eff = max_effect(base, fns, allowed)?;
            let index_eff = max_effect(index, fns, allowed)?;
            Ok(base_eff.join(index_eff))
        }
        Expr::If {
            cond,
            then_br,
            else_br,
            ..
        } => {
            let cond_eff = max_effect(cond, fns, allowed)?;
            let then_eff = max_effect(then_br, fns, allowed)?;
            let else_eff = max_effect(else_br, fns, allowed)?;
            Ok(cond_eff.join(then_eff).join(else_eff))
        }
        Expr::Match {
            scrutinee, arms, ..
        } => {
            let mut eff = max_effect(scrutinee, fns, allowed)?;
            for arm in arms {
                eff = eff.join(max_effect(&arm.expr, fns, allowed)?);
            }
            Ok(eff)
        }
        Expr::Try { expr, .. } => max_effect(expr, fns, allowed),
        Expr::Call { callee, args, span } => {
            let mut eff = EffectLevel::Pure;
            for arg in args {
                eff = eff.join(max_effect(arg, fns, allowed)?);
            }
            let call_eff = call_effect(callee, fns);
            if call_eff > allowed {
                return Err(
                    TyperError::effect_required(callee, effect_label(call_eff), *span).into(),
                );
            }
            Ok(eff.join(call_eff))
        }
    }
}
fn call_effect(callee: &str, fns: &HashMap<&str, FnSig>) -> EffectLevel {
    if let Some(level) = builtin_effect(callee) {
        return level;
    }
    fns.get(callee)
        .map(|sig| sig.effect)
        .unwrap_or(EffectLevel::Pure)
}
fn builtin_effect(callee: &str) -> Option<EffectLevel> {
    match callee {
        "std::list::push_mut"
        | "std::list::insert_mut"
        | "std::list::remove_mut"
        | "std::list::pop_mut"
        | "std::set::insert_mut"
        | "std::set::remove_mut"
        | "std::map::insert_mut"
        | "std::map::remove_mut" => Some(EffectLevel::Mut),
        "std::wasi::print" | "std::env::time" | "std::env::random" => Some(EffectLevel::Io),
        _ => None,
    }
}
fn ensure_int(ty: Type, aliases: &AliasMap, what: &str, span: Option<Span>) -> Result<()> {
    if base_type(&ty, aliases)? != Type::Int {
        return Err(TyperError::int_operand(what, ty, span).into());
    }
    Ok(())
}
fn ensure_bool(ty: Type, aliases: &AliasMap, what: &str, span: Option<Span>) -> Result<()> {
    if base_type(&ty, aliases)? != Type::Bool {
        return Err(TyperError::bool_operand(what, ty, span).into());
    }
    Ok(())
}

fn unsigned_literal_value(expr: &Expr) -> Option<u128> {
    match expr {
        Expr::Int(value, _) => (*value >= 0).then_some(*value as u128),
        Expr::Return { expr, .. } => unsigned_literal_value(expr),
        Expr::Block { block } if block.statements.is_empty() => block
            .tail
            .as_ref()
            .and_then(|tail| unsigned_literal_value(tail)),
        Expr::Call { callee, args, .. }
            if matches!(callee.as_str(), "U8" | "U64" | "U128" | "U256") && args.len() == 1 =>
        {
            unsigned_literal_value(&args[0])
        }
        _ => None,
    }
}

fn int_literal_value(expr: &Expr) -> Option<i64> {
    match expr {
        Expr::Int(value, _) => Some(*value),
        Expr::Return { expr, .. } => int_literal_value(expr),
        Expr::Block { block } if block.statements.is_empty() => {
            block.tail.as_ref().and_then(|tail| int_literal_value(tail))
        }
        _ => None,
    }
}

fn unsigned_literal_max(target: &Type) -> Option<u128> {
    match target {
        Type::U8 => Some(u8::MAX as u128),
        Type::U64 => Some(u64::MAX as u128),
        Type::U128 => Some(u128::MAX),
        Type::U256 => None,
        _ => None,
    }
}

fn unsigned_literal_fits_value(target: &Type, value: u128) -> bool {
    match unsigned_literal_max(target) {
        Some(max) => value <= max,
        None => true,
    }
}

fn unsigned_literal_fits(target: &Type, expr: &Expr) -> bool {
    unsigned_literal_value(expr)
        .map(|value| unsigned_literal_fits_value(target, value))
        .unwrap_or(false)
}

pub(super) fn literal_can_coerce_unsigned(expected: &Type, actual: &Type, expr: &Expr) -> bool {
    matches!(expected, Type::U8 | Type::U64 | Type::U128 | Type::U256)
        && matches!(actual, Type::Int)
        && unsigned_literal_fits(expected, expr)
}

pub(super) fn unsigned_literal_range_error(
    expected: &Type,
    actual: &Type,
    expr: &Expr,
) -> Option<TyperError> {
    if matches!(expected, Type::U8 | Type::U64 | Type::U128 | Type::U256)
        && matches!(actual, Type::Int)
    {
        if let Some(value) = unsigned_literal_value(expr) {
            if !unsigned_literal_fits_value(expected, value) {
                return Some(TyperError::unsigned_literal_out_of_range(
                    expected.clone(),
                    value,
                    expr_span(expr),
                ));
            }
        }
    }
    None
}

fn u64_literal_overflow(op: &BinOp, lhs: u128, rhs: u128) -> bool {
    let max = u64::MAX as u128;
    match op {
        BinOp::Add => lhs + rhs > max,
        BinOp::Sub => rhs > lhs,
        BinOp::Mul => lhs
            .checked_mul(rhs)
            .map(|value| value > max)
            .unwrap_or(true),
        _ => false,
    }
}
pub(super) fn expr_span(e: &Expr) -> Span {
    match e {
        Expr::Int(_, sp) | Expr::Bool(_, sp) | Expr::String(_, sp) | Expr::Var(_, sp) => *sp,
        Expr::ArrayLit { span, .. }
        | Expr::TupleLit { span, .. }
        | Expr::StructLit { span, .. }
        | Expr::FieldAccess { span, .. }
        | Expr::Index { span, .. } => *span,
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
pub(crate) fn show_ty(t: Type) -> String {
    fn render(ty: Type) -> String {
        match ty {
            Type::Int => "Int".to_string(),
            Type::U8 => "U8".to_string(),
            Type::U64 => "U64".to_string(),
            Type::U128 => "U128".to_string(),
            Type::U256 => "U256".to_string(),
            Type::Bool => "Bool".to_string(),
            Type::String => "String".to_string(),
            Type::Bytes => "Bytes".to_string(),
            Type::Resource(name) => name,
            Type::Option(inner) => format!("Option<{}>", render(*inner)),
            Type::Result(ok, err) => format!("Result<{}, {}>", render(*ok), render(*err)),
            Type::List(inner) => format!("List<{}>", render(*inner)),
            Type::Set(inner) => format!("Set<{}>", render(*inner)),
            Type::Map(key, val) => format!("Map<{}, {}>", render(*key), render(*val)),
            Type::Array(inner, len) => format!("[{}; {}]", render(*inner), len),
            Type::Tuple(elements) => {
                let rendered = elements
                    .into_iter()
                    .map(|elem| render(elem))
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("({})", rendered)
            }
        }
    }
    render(t)
}
