mod block;
mod call_expr;
mod calls;
mod effects;
mod literals;
mod match_expr;
mod ops;
mod traits;
mod typing;

pub(crate) use self::effects::max_effect;
pub(super) use self::literals::{literal_can_coerce_unsigned, unsigned_literal_range_error};
pub(crate) use self::traits::{ensure_trait_bound, trait_impl_exists, type_pattern_matches};
pub(crate) use self::typing::infer_expr_type;
pub(super) use self::typing::{consume_var_expr, type_of};
use crate::errors::TyperError;
use anyhow::{bail, Result};
use clg_ast::{Expr, ParamKind, Span, Type};
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
    pub(super) fn register_param(&mut self, name: &str, kind: ParamKind, is_resource: bool) {
        if is_resource {
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
    pub(super) fn register_local(&mut self, name: &str, is_resource: bool) {
        if is_resource {
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
    pub(super) fn use_var_in_call(&mut self, name: &str, span: Span, callee: &str) -> Result<()> {
        if let Some(tracked) = self.states.get(name) {
            if let ResourceState::Consumed { span: consumed_at } = tracked.state {
                return Err(TyperError::resource_use_after_consume_in_call(
                    name,
                    consumed_at,
                    span,
                    callee,
                )
                .into());
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
    pub(super) fn consume_var_in_call(
        &mut self,
        name: &str,
        span: Span,
        callee: &str,
    ) -> Result<()> {
        if let Some(tracked) = self.states.get_mut(name) {
            let mark_consumed = match &mut tracked.state {
                ResourceState::Owned { borrows } => {
                    if *borrows > 0 {
                        return Err(TyperError::resource_consume_borrow_in_call(
                            name, span, callee,
                        )
                        .into());
                    }
                    true
                }
                ResourceState::ActiveBorrow => {
                    return Err(TyperError::resource_consume_borrow_in_call(name, span, callee).into());
                }
                ResourceState::Consumed { span: first } => {
                    return Err(TyperError::resource_double_consume_in_call(
                        name, *first, span, callee,
                    )
                    .into());
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
            Type::Named { name, args } => {
                if args.is_empty() {
                    name
                } else {
                    let rendered = args.into_iter().map(render).collect::<Vec<_>>().join(", ");
                    format!("{}<{}>", name, rendered)
                }
            }
            Type::Option(inner) => format!("Option<{}>", render(*inner)),
            Type::Result(ok, err) => format!("Result<{}, {}>", render(*ok), render(*err)),
            Type::List(inner) => format!("List<{}>", render(*inner)),
            Type::Set(inner) => format!("Set<{}>", render(*inner)),
            Type::Map(key, val) => format!("Map<{}, {}>", render(*key), render(*val)),
            Type::Array(inner, Some(len)) => format!("[{}; {}]", render(*inner), len),
            Type::Array(inner, None) => format!("Array<{}>", render(*inner)),
            Type::Slice(inner) => format!("Slice<{}>", render(*inner)),
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
