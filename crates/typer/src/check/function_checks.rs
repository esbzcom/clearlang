use anyhow::Result;
use clg_ast::{
    Block, Contract, ContractDecl, ContractInitDecl, Expr, Func, MigrationDecl, ParamKind, Stmt,
    TraitMethod, Type,
};
use std::collections::{HashMap, HashSet};

use super::expr::{self, consume_var_expr, expr_span, max_effect, type_of, ResourceTracker};
use super::mut_guards::enforce_mut_guards;
use super::totality::level_from_effect;
use super::type_params::validate_bounds;
use super::{
    base_type, base_types_match, binding_compatible, effect_label, is_resource_type,
    refinement_loss, substitute_type, type_param_names, AliasMap, BoundsMap, EffectLevel, FnSig,
    ImplInfo, LocalBinding, TypeDefs, TypeSubst, RETURN_KEY,
};
use crate::errors::TyperError;
use crate::guards::{collect_mut_guards, MutGuardKey};

pub(super) fn check_contract_invariant<'a>(
    contract: &'a ContractDecl,
    invariant: &'a Contract,
    fns: &HashMap<&'a str, FnSig>,
    trait_env: &super::TraitEnv<'a>,
    aliases: &AliasMap,
    type_defs: &TypeDefs,
) -> Result<()> {
    let mut env = HashMap::new();
    env.insert(
        "state",
        LocalBinding {
            ty: Type::Named {
                name: super::contract_state_type_name(&contract.name),
                args: Vec::new(),
            },
            kind: ParamKind::Borrow,
        },
    );
    let mut tracker = ResourceTracker::with_capacity(0);
    let type_params = HashSet::new();
    let bounds = BoundsMap::new();
    let ty = type_of(
        &invariant.expr,
        &env,
        &mut tracker,
        fns,
        trait_env,
        aliases,
        type_defs,
        &type_params,
        &bounds,
        0,
        None,
    )?;
    if base_type(&ty, aliases)? != Type::Bool {
        return Err(TyperError::contract_not_bool("invariant", ty, invariant.span).into());
    }
    max_effect(&invariant.expr, fns, trait_env, EffectLevel::Pure)?;
    Ok(())
}

pub(super) fn check_contract_migration<'a>(
    contract: &'a ContractDecl,
    migration: &'a MigrationDecl,
    fns: &HashMap<&'a str, FnSig>,
    trait_env: &super::TraitEnv<'a>,
    aliases: &AliasMap,
    type_defs: &TypeDefs,
) -> Result<()> {
    let valid_digest = migration
        .from_schema
        .strip_prefix("sha256:")
        .is_some_and(|hex| hex.len() == 64 && hex.bytes().all(|byte| byte.is_ascii_hexdigit()));
    if !valid_digest {
        return Err(TyperError::invalid_migration_schema_digest(migration.from_schema_span).into());
    }
    let mut env = HashMap::new();
    env.insert(
        "state",
        LocalBinding {
            ty: Type::Named {
                name: super::contract_state_type_name(&contract.name),
                args: Vec::new(),
            },
            kind: ParamKind::Borrow,
        },
    );
    env.insert(
        "__clg_state_write_cap",
        LocalBinding {
            ty: Type::Bool,
            kind: ParamKind::Borrow,
        },
    );
    let mut tracker = ResourceTracker::with_capacity(0);
    let type_params = HashSet::new();
    let bounds = BoundsMap::new();
    type_of(
        &migration.body,
        &env,
        &mut tracker,
        fns,
        trait_env,
        aliases,
        type_defs,
        &type_params,
        &bounds,
        0,
        None,
    )?;
    max_effect(&migration.body, fns, trait_env, EffectLevel::Mut)?;
    Ok(())
}

pub(super) fn check_contract_init<'a>(
    contract: &'a ContractDecl,
    init: &'a ContractInitDecl,
    fns: &HashMap<&'a str, FnSig>,
    trait_env: &super::TraitEnv<'a>,
    aliases: &AliasMap,
    type_defs: &TypeDefs,
) -> Result<()> {
    let mut env = HashMap::new();
    env.insert(
        "state",
        LocalBinding {
            ty: Type::Named {
                name: super::contract_state_type_name(&contract.name),
                args: Vec::new(),
            },
            kind: ParamKind::Borrow,
        },
    );
    env.insert(
        "__clg_state_write_cap",
        LocalBinding {
            ty: Type::Bool,
            kind: ParamKind::Borrow,
        },
    );
    let mut tracker = ResourceTracker::with_capacity(init.params.len());
    for param in &init.params {
        if env
            .insert(
                param.name.as_str(),
                LocalBinding {
                    ty: param.ty.clone(),
                    kind: param.kind,
                },
            )
            .is_some()
        {
            return Err(TyperError::duplicate_parameter(&param.name).into());
        }
        let resolved_ty = base_type(&param.ty, aliases)?;
        tracker.register_param(
            param.name.as_str(),
            param.kind,
            is_resource_type(&resolved_ty, aliases, type_defs)?,
        );
    }
    let type_params = HashSet::new();
    let bounds = BoundsMap::new();
    type_of(
        &init.body,
        &env,
        &mut tracker,
        fns,
        trait_env,
        aliases,
        type_defs,
        &type_params,
        &bounds,
        0,
        None,
    )?;
    max_effect(&init.body, fns, trait_env, EffectLevel::Mut)?;

    let initialized = direct_init_writes(&init.body);
    for field in &contract.fields {
        if !initialized.contains(field.name.as_str()) {
            return Err(TyperError::contract_init_incomplete(&field.name, init.span).into());
        }
    }
    Ok(())
}

fn direct_init_writes(body: &Expr) -> HashSet<&str> {
    let Expr::Block { block } = body else {
        return HashSet::new();
    };
    block
        .statements
        .iter()
        .filter_map(|stmt| match stmt {
            Stmt::Expr { expr, .. } => match expr.as_ref() {
                Expr::Call { callee, .. } => callee.strip_prefix("__clg_state_write$"),
                _ => None,
            },
            Stmt::Let { .. } | Stmt::While { .. } => None,
        })
        .collect()
}

pub(super) fn check_func<'a>(
    f: &'a Func,
    fns: &HashMap<&'a str, FnSig>,
    trait_env: &super::TraitEnv<'a>,
    aliases: &AliasMap,
    type_defs: &TypeDefs,
    contract_owner: Option<&str>,
) -> Result<()> {
    let type_params = type_param_names(&f.type_params);
    let bounds = validate_bounds(&f.where_bounds, &type_params, trait_env)?;
    let mut env: HashMap<&str, LocalBinding> = HashMap::with_capacity(f.params.len() + 1);
    let ret_ty = f.ret.clone();
    env.insert(
        RETURN_KEY,
        LocalBinding {
            ty: ret_ty.clone(),
            kind: ParamKind::Borrow,
        },
    );
    if let Some(contract_name) = contract_owner {
        if matches!(f.effect, clg_ast::Effect::Pure | clg_ast::Effect::Mut) {
            env.insert(
                "state",
                LocalBinding {
                    ty: Type::Named {
                        name: super::contract_state_type_name(contract_name),
                        args: Vec::new(),
                    },
                    kind: ParamKind::Borrow,
                },
            );
        }
        if matches!(f.effect, clg_ast::Effect::Mut) {
            env.insert(
                "__clg_state_write_cap",
                LocalBinding {
                    ty: Type::Bool,
                    kind: ParamKind::Borrow,
                },
            );
            env.insert(
                "__clg_event_emit_cap",
                LocalBinding {
                    ty: Type::Bool,
                    kind: ParamKind::Borrow,
                },
            );
            env.insert(
                "__clg_external_call_cap",
                LocalBinding {
                    ty: Type::Bool,
                    kind: ParamKind::Borrow,
                },
            );
        }
    }
    let mut tracker = ResourceTracker::with_capacity(f.params.len());
    for p in &f.params {
        let resolved_ty = base_type(&p.ty, aliases)?;
        if env
            .insert(
                p.name.as_str(),
                LocalBinding {
                    ty: p.ty.clone(),
                    kind: p.kind,
                },
            )
            .is_some()
        {
            return Err(TyperError::duplicate_parameter(&p.name).into());
        }
        let is_resource = is_resource_type(&resolved_ty, aliases, type_defs)?;
        tracker.register_param(p.name.as_str(), p.kind, is_resource);
    }

    let allowed_effect = level_from_effect(f.effect);

    for req in &f.requires {
        let mut req_tracker = tracker.clone();
        let ty = type_of(
            &req.expr,
            &env,
            &mut req_tracker,
            fns,
            trait_env,
            aliases,
            type_defs,
            &type_params,
            &bounds,
            0,
            None,
        )?;
        if base_type(&ty, aliases)? != Type::Bool {
            return Err(TyperError::contract_not_bool("require", ty, req.span).into());
        }
        max_effect(&req.expr, fns, trait_env, EffectLevel::Pure)?;
    }

    let guard_keys = collect_mut_guards(&f.requires);

    let mut ensure_env = env.clone();
    if !ensure_env.contains_key("result") {
        ensure_env.insert(
            "result",
            LocalBinding {
                ty: f.ret.clone(),
                kind: ParamKind::Borrow,
            },
        );
    }
    for ens in &f.ensures {
        let mut ensure_tracker = tracker.clone();
        ensure_env.insert(
            "__clg_old_cap",
            LocalBinding {
                ty: Type::Bool,
                kind: ParamKind::Borrow,
            },
        );
        let ty = type_of(
            &ens.expr,
            &ensure_env,
            &mut ensure_tracker,
            fns,
            trait_env,
            aliases,
            type_defs,
            &type_params,
            &bounds,
            0,
            None,
        )?;
        if base_type(&ty, aliases)? != Type::Bool {
            return Err(TyperError::contract_not_bool("ensure", ty, ens.span).into());
        }
        max_effect(&ens.expr, fns, trait_env, EffectLevel::Pure)?;
        ensure_env.remove("__clg_old_cap");
    }

    let body_ty = type_of(
        &f.body,
        &env,
        &mut tracker,
        fns,
        trait_env,
        aliases,
        type_defs,
        &type_params,
        &bounds,
        0,
        Some(&ret_ty),
    )?;
    if !binding_compatible(&ret_ty, &body_ty, aliases)? {
        let sp = expr_span(&f.body);
        let allow_unsigned_literal = expr::literal_can_coerce_unsigned(&ret_ty, &body_ty, &f.body);
        if !allow_unsigned_literal {
            if let Some(err) = expr::unsigned_literal_range_error(&ret_ty, &body_ty, &f.body) {
                return Err(err.into());
            }
            if base_types_match(&ret_ty, &body_ty, aliases)?
                && refinement_loss(&ret_ty, &body_ty, aliases)
            {
                return Err(TyperError::refinement_loss(ret_ty.clone(), body_ty, sp).into());
            }
            return Err(TyperError::return_type_mismatch(ret_ty.clone(), body_ty, sp).into());
        }
    }
    if is_resource_type(&ret_ty, aliases, type_defs)? {
        consume_var_expr(&mut tracker, &f.body)?;
    }
    tracker.ensure_consumed()?;
    if allowed_effect >= EffectLevel::Mut {
        enforce_mut_guards(&f.body, &guard_keys)?;
    }
    max_effect(&f.body, fns, trait_env, allowed_effect)?;
    if contract_owner.is_some() && matches!(f.effect, clg_ast::Effect::Mut) {
        enforce_external_call_cei(&f.body)?;
    }
    Ok(())
}

#[derive(Clone, Copy)]
struct ExternalCallFlow {
    can_continue_before_call: bool,
    can_continue_after_call: bool,
}

impl ExternalCallFlow {
    const fn entry() -> Self {
        Self {
            can_continue_before_call: true,
            can_continue_after_call: false,
        }
    }

    fn merge(self, other: Self) -> Self {
        Self {
            can_continue_before_call: self.can_continue_before_call
                || other.can_continue_before_call,
            can_continue_after_call: self.can_continue_after_call || other.can_continue_after_call,
        }
    }

    fn interaction(self, span: clg_ast::Span) -> Result<Self> {
        if self.can_continue_after_call {
            return Err(TyperError::external_call_order_violation(span).into());
        }
        Ok(self)
    }

    fn outbound_call(self, span: clg_ast::Span) -> Result<Self> {
        if self.can_continue_after_call {
            return Err(TyperError::external_call_order_violation(span).into());
        }
        Ok(Self {
            can_continue_before_call: false,
            can_continue_after_call: self.can_continue_before_call,
        })
    }
}

fn enforce_external_call_cei(body: &Expr) -> Result<()> {
    let _ = analyze_external_call_cei(body, ExternalCallFlow::entry())?;
    Ok(())
}

fn analyze_external_call_cei(expr: &Expr, flow: ExternalCallFlow) -> Result<ExternalCallFlow> {
    match expr {
        Expr::Int(_, _) | Expr::Bool(_, _) | Expr::String(_, _) | Expr::Var(_, _) => Ok(flow),
        Expr::Return { expr, .. } | Expr::Try { expr, .. } | Expr::Unary { expr, .. } => {
            analyze_external_call_cei(expr, flow)
        }
        Expr::ArrayLit { elems, .. } | Expr::TupleLit { elems, .. } => elems
            .iter()
            .try_fold(flow, |flow, item| analyze_external_call_cei(item, flow)),
        Expr::StructLit { fields, .. } => fields.iter().try_fold(flow, |flow, field| {
            analyze_external_call_cei(&field.expr, flow)
        }),
        Expr::FieldAccess { base, span, .. } => {
            let flow = analyze_external_call_cei(base, flow)?;
            if matches!(base.as_ref(), Expr::Var(name, _) if name == "state") {
                flow.interaction(*span)
            } else {
                Ok(flow)
            }
        }
        Expr::Index { base, index, .. } => {
            let flow = analyze_external_call_cei(base, flow)?;
            analyze_external_call_cei(index, flow)
        }
        Expr::Bin { lhs, rhs, .. } => {
            let flow = analyze_external_call_cei(lhs, flow)?;
            analyze_external_call_cei(rhs, flow)
        }
        Expr::Call {
            callee, args, span, ..
        } => {
            let flow = args
                .iter()
                .try_fold(flow, |flow, arg| analyze_external_call_cei(arg, flow))?;
            if callee.starts_with("__clg_state_write$") || callee.starts_with("__clg_event_emit$") {
                flow.interaction(*span)
            } else if callee.starts_with("__clg_external_call$") {
                flow.outbound_call(*span)
            } else {
                Ok(flow)
            }
        }
        Expr::Block { block } => analyze_external_call_cei_block(block, flow),
        Expr::If {
            cond,
            then_br,
            else_br,
            ..
        } => {
            let flow = analyze_external_call_cei(cond, flow)?;
            let then_flow = analyze_external_call_cei(then_br, flow)?;
            let else_flow = analyze_external_call_cei(else_br, flow)?;
            Ok(then_flow.merge(else_flow))
        }
        Expr::Match {
            scrutinee, arms, ..
        } => {
            let flow = analyze_external_call_cei(scrutinee, flow)?;
            arms.iter().try_fold(
                ExternalCallFlow {
                    can_continue_before_call: false,
                    can_continue_after_call: false,
                },
                |merged, arm| Ok(merged.merge(analyze_external_call_cei(&arm.expr, flow)?)),
            )
        }
        Expr::Lambda { body, .. } => {
            let _ = analyze_external_call_cei(body, ExternalCallFlow::entry())?;
            Ok(flow)
        }
    }
}

fn analyze_external_call_cei_block(
    block: &Block,
    flow: ExternalCallFlow,
) -> Result<ExternalCallFlow> {
    let flow = block
        .statements
        .iter()
        .try_fold(flow, |flow, stmt| match stmt {
            Stmt::Let { expr, .. } | Stmt::Expr { expr, .. } => {
                analyze_external_call_cei(expr, flow)
            }
            Stmt::While {
                cond,
                invariant,
                variant,
                body,
                span,
            } => {
                let flow = analyze_external_call_cei(cond, flow)?;
                let flow = analyze_external_call_cei(invariant, flow)?;
                let flow = if let Some(variant) = variant {
                    analyze_external_call_cei(variant, flow)?
                } else {
                    flow
                };
                let body_flow = analyze_external_call_cei_block(body, flow)?;
                if body_flow.can_continue_after_call {
                    return Err(TyperError::external_call_order_violation(*span).into());
                }
                Ok(flow)
            }
        })?;
    if let Some(tail) = &block.tail {
        analyze_external_call_cei(tail, flow)
    } else {
        Ok(flow)
    }
}

pub(super) fn check_impl_method<'a>(
    method: &'a Func,
    imp: &ImplInfo<'a>,
    fns: &HashMap<&'a str, FnSig>,
    trait_env: &super::TraitEnv<'a>,
    aliases: &AliasMap,
    type_defs: &TypeDefs,
) -> Result<()> {
    let type_params = type_param_names(&imp.decl.type_params);
    let bounds = validate_bounds(&imp.decl.where_bounds, &type_params, trait_env)?;
    let mut subst: TypeSubst = HashMap::new();
    subst.insert("Self".to_string(), imp.decl.for_type.clone());

    let ret_ty = substitute_type(&method.ret, &subst);
    let mut env: HashMap<&str, LocalBinding> = HashMap::with_capacity(method.params.len() + 1);
    env.insert(
        RETURN_KEY,
        LocalBinding {
            ty: ret_ty.clone(),
            kind: ParamKind::Borrow,
        },
    );
    let mut tracker = ResourceTracker::with_capacity(method.params.len());
    for param in &method.params {
        let param_ty = substitute_type(&param.ty, &subst);
        let resolved_ty = base_type(&param_ty, aliases)?;
        if env
            .insert(
                param.name.as_str(),
                LocalBinding {
                    ty: param_ty.clone(),
                    kind: param.kind,
                },
            )
            .is_some()
        {
            return Err(TyperError::duplicate_parameter(&param.name).into());
        }
        let is_resource = is_resource_type(&resolved_ty, aliases, type_defs)?;
        tracker.register_param(param.name.as_str(), param.kind, is_resource);
    }

    let allowed_effect = level_from_effect(method.effect);

    for req in &method.requires {
        let mut req_tracker = tracker.clone();
        let ty = type_of(
            &req.expr,
            &env,
            &mut req_tracker,
            fns,
            trait_env,
            aliases,
            type_defs,
            &type_params,
            &bounds,
            0,
            None,
        )?;
        if base_type(&ty, aliases)? != Type::Bool {
            return Err(TyperError::contract_not_bool("require", ty, req.span).into());
        }
        max_effect(&req.expr, fns, trait_env, EffectLevel::Pure)?;
    }

    let mut ensure_env = env.clone();
    if !ensure_env.contains_key("result") {
        ensure_env.insert(
            "result",
            LocalBinding {
                ty: ret_ty.clone(),
                kind: ParamKind::Borrow,
            },
        );
    }
    for ens in &method.ensures {
        let mut ensure_tracker = tracker.clone();
        let ty = type_of(
            &ens.expr,
            &ensure_env,
            &mut ensure_tracker,
            fns,
            trait_env,
            aliases,
            type_defs,
            &type_params,
            &bounds,
            0,
            None,
        )?;
        if base_type(&ty, aliases)? != Type::Bool {
            return Err(TyperError::contract_not_bool("ensure", ty, ens.span).into());
        }
        max_effect(&ens.expr, fns, trait_env, EffectLevel::Pure)?;
    }

    let body_ty = type_of(
        &method.body,
        &env,
        &mut tracker,
        fns,
        trait_env,
        aliases,
        type_defs,
        &type_params,
        &bounds,
        0,
        Some(&ret_ty),
    )?;
    if !binding_compatible(&ret_ty, &body_ty, aliases)? {
        let sp = expr_span(&method.body);
        let allow_unsigned_literal =
            expr::literal_can_coerce_unsigned(&ret_ty, &body_ty, &method.body);
        if !allow_unsigned_literal {
            if let Some(err) = expr::unsigned_literal_range_error(&ret_ty, &body_ty, &method.body) {
                return Err(err.into());
            }
            if base_types_match(&ret_ty, &body_ty, aliases)?
                && refinement_loss(&ret_ty, &body_ty, aliases)
            {
                return Err(TyperError::refinement_loss(ret_ty.clone(), body_ty, sp).into());
            }
            return Err(TyperError::return_type_mismatch(ret_ty.clone(), body_ty, sp).into());
        }
    }
    if is_resource_type(&ret_ty, aliases, type_defs)? {
        consume_var_expr(&mut tracker, &method.body)?;
    }
    tracker.ensure_consumed()?;
    if allowed_effect >= EffectLevel::Mut {
        let guard_keys = collect_mut_guards(&method.requires);
        enforce_mut_guards(&method.body, &guard_keys)?;
    }
    max_effect(&method.body, fns, trait_env, allowed_effect)?;
    Ok(())
}

pub(super) fn check_trait_default_method<'a>(
    trait_name: &str,
    method: &'a TraitMethod,
    fns: &HashMap<&'a str, FnSig>,
    trait_env: &super::TraitEnv<'a>,
    aliases: &AliasMap,
    type_defs: &TypeDefs,
) -> Result<()> {
    let Some(body) = method.default_body.as_ref() else {
        return Ok(());
    };

    let mut type_params: HashSet<String> = HashSet::with_capacity(1);
    type_params.insert("Self".to_string());

    let mut bounds: BoundsMap = HashMap::with_capacity(1);
    let mut self_bounds = HashSet::with_capacity(1);
    self_bounds.insert(trait_name.to_string());
    bounds.insert("Self".to_string(), self_bounds);

    let ret_ty = method.ret.clone();
    let mut env: HashMap<&str, LocalBinding> = HashMap::with_capacity(method.params.len() + 1);
    env.insert(
        RETURN_KEY,
        LocalBinding {
            ty: ret_ty.clone(),
            kind: ParamKind::Borrow,
        },
    );

    let mut tracker = ResourceTracker::with_capacity(method.params.len());
    for p in &method.params {
        let resolved_ty = base_type(&p.ty, aliases)?;
        if env
            .insert(
                p.name.as_str(),
                LocalBinding {
                    ty: p.ty.clone(),
                    kind: p.kind,
                },
            )
            .is_some()
        {
            return Err(TyperError::duplicate_parameter(&p.name).into());
        }
        let is_resource = is_resource_type(&resolved_ty, aliases, type_defs)?;
        tracker.register_param(p.name.as_str(), p.kind, is_resource);
    }

    let body_ty = type_of(
        body,
        &env,
        &mut tracker,
        fns,
        trait_env,
        aliases,
        type_defs,
        &type_params,
        &bounds,
        0,
        Some(&ret_ty),
    )?;
    if !binding_compatible(&ret_ty, &body_ty, aliases)? {
        let sp = expr_span(body);
        let allow_unsigned_literal = expr::literal_can_coerce_unsigned(&ret_ty, &body_ty, body);
        if !allow_unsigned_literal {
            if let Some(err) = expr::unsigned_literal_range_error(&ret_ty, &body_ty, body) {
                return Err(err.into());
            }
            if base_types_match(&ret_ty, &body_ty, aliases)?
                && refinement_loss(&ret_ty, &body_ty, aliases)
            {
                return Err(TyperError::refinement_loss(ret_ty.clone(), body_ty, sp).into());
            }
            return Err(TyperError::return_type_mismatch(ret_ty.clone(), body_ty, sp).into());
        }
    }

    if is_resource_type(&ret_ty, aliases, type_defs)? {
        consume_var_expr(&mut tracker, body)?;
    }
    tracker.ensure_consumed()?;

    let declared_effect = level_from_effect(method.effect);
    if declared_effect >= EffectLevel::Mut {
        let guard_keys: HashSet<MutGuardKey> = HashSet::new();
        enforce_mut_guards(body, &guard_keys)?;
    }

    let inferred_effect = max_effect(body, fns, trait_env, EffectLevel::Io)?;
    if inferred_effect != declared_effect {
        return Err(TyperError::trait_default_effect_mismatch(
            trait_name,
            method.name.as_str(),
            effect_label(declared_effect),
            effect_label(inferred_effect),
            method.span,
        )
        .into());
    }
    Ok(())
}
