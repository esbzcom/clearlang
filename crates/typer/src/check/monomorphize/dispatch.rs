use std::collections::{HashMap, HashSet};

use anyhow::Result;
use clg_ast::{Func, Span, Type};

use crate::errors::TyperError;

use super::super::expr::{ensure_trait_bound, trait_impl_exists, type_pattern_matches};
use super::super::{show_ty, type_param_names, unify_type_params, BoundsMap, TypeSubst};
use super::helpers::instantiate_func;
use super::mangle::{mangle_fn_name, mangle_impl_method_name};
use super::Monomorphizer;

fn trait_default_base_func(method_name: &str, method: &clg_ast::TraitMethod) -> Result<Func> {
    let Some(body) = method.default_body.clone() else {
        anyhow::bail!("missing impl method `{}`", method_name);
    };
    Ok(Func {
        is_exported: false,
        effect: method.effect,
        effect_span: method.effect_span,
        name: method.name.clone(),
        type_params: Vec::new(),
        params: method.params.clone(),
        ret: method.ret.clone(),
        where_bounds: Vec::new(),
        requires: Vec::new(),
        ensures: Vec::new(),
        body,
    })
}

impl<'a> Monomorphizer<'a> {
    pub(super) fn resolve_generic_call(
        &mut self,
        callee: &str,
        sig: &super::super::FnSig,
        arg_types: &[Type],
        type_params: &HashSet<String>,
        bounds: &BoundsMap,
        span: Span,
    ) -> Result<String> {
        let mut subst: TypeSubst = HashMap::new();
        let param_set: HashSet<String> = sig.type_params.iter().cloned().collect();
        for (param, arg_ty) in sig.params.iter().zip(arg_types.iter()) {
            unify_type_params(&param.ty, arg_ty, &param_set, &mut subst, self.aliases)?;
        }
        for name in &sig.type_params {
            if !subst.contains_key(name) {
                return Err(TyperError::cannot_infer_type_params(callee, span).into());
            }
        }
        for bound in &sig.bounds {
            let Some(bound_ty) = subst.get(&bound.param) else {
                return Err(TyperError::cannot_infer_type_params(callee, span).into());
            };
            ensure_trait_bound(
                bound_ty,
                bound.trait_name.as_str(),
                type_params,
                bounds,
                self.trait_env,
                self.aliases,
                span,
            )?;
        }

        let mut args_vec: Vec<Type> = Vec::with_capacity(sig.type_params.len());
        for name in &sig.type_params {
            let Some(ty) = subst.get(name) else {
                return Err(TyperError::cannot_infer_type_params(callee, span).into());
            };
            args_vec.push(ty.clone());
        }
        let mangled = mangle_fn_name(callee, &args_vec, self.aliases)?;
        if !self.mono_map.contains_key(mangled.as_str()) {
            let base = self
                .base_funcs
                .get(callee)
                .ok_or_else(|| anyhow::anyhow!("missing function `{}`", callee))?;
            let inst = instantiate_func(base, &subst, mangled.clone());
            self.register_func(inst);
        }
        Ok(mangled)
    }

    pub(super) fn resolve_trait_call(
        &mut self,
        trait_name: &str,
        method_name: &str,
        arg_types: &[Type],
        type_params: &HashSet<String>,
        bounds: &BoundsMap,
        span: Span,
    ) -> Result<String> {
        let trait_info = self
            .trait_env
            .traits
            .get(trait_name)
            .ok_or_else(|| TyperError::unknown_trait(trait_name, span))?;
        let method = trait_info.methods.get(method_name).ok_or_else(|| {
            TyperError::unknown_function(&format!("{}::{}", trait_name, method_name), span)
        })?;
        if method.params.len() != arg_types.len() {
            return Err(TyperError::arity_mismatch(
                &format!("{}::{}", trait_name, method_name),
                method.params.len(),
                arg_types.len(),
                span,
            )
            .into());
        }

        let mut self_params: HashSet<String> = HashSet::with_capacity(1);
        self_params.insert("Self".to_string());
        let mut subst: TypeSubst = HashMap::new();
        for (param, arg_ty) in method.params.iter().zip(arg_types.iter()) {
            unify_type_params(&param.ty, arg_ty, &self_params, &mut subst, self.aliases)?;
        }
        let Some(self_ty) = subst.get("Self").cloned() else {
            return Err(TyperError::cannot_infer_type_params(
                &format!("{}::{}", trait_name, method_name),
                span,
            )
            .into());
        };
        ensure_trait_bound(
            &self_ty,
            trait_name,
            type_params,
            bounds,
            self.trait_env,
            self.aliases,
            span,
        )?;

        let (imp, impl_subst) = self.find_impl(trait_name, &self_ty, span)?;
        let mut full_subst = impl_subst;
        full_subst.insert("Self".to_string(), self_ty.clone());
        let mangled = mangle_impl_method_name(trait_name, &self_ty, method_name, self.aliases)?;
        if !self.mono_map.contains_key(mangled.as_str()) {
            let inst = if let Some(impl_method) = imp.methods.get(method_name) {
                instantiate_func(impl_method, &full_subst, mangled.clone())
            } else {
                let base = trait_default_base_func(method_name, method)?;
                instantiate_func(&base, &full_subst, mangled.clone())
            };
            self.register_func(inst);
        }
        Ok(mangled)
    }

    pub(super) fn find_impl(
        &self,
        trait_name: &str,
        self_ty: &Type,
        span: Span,
    ) -> Result<(&super::super::ImplInfo<'a>, TypeSubst)> {
        let mut matches: Vec<(&super::super::ImplInfo<'a>, TypeSubst)> = Vec::new();
        for imp in &self.trait_env.impls {
            if imp.decl.trait_name != trait_name {
                continue;
            }
            let impl_params = type_param_names(&imp.decl.type_params);
            let mut subst: TypeSubst = HashMap::new();
            if !type_pattern_matches(
                &imp.decl.for_type,
                self_ty,
                &impl_params,
                &mut subst,
                self.aliases,
            )? {
                continue;
            }
            if impl_params.iter().any(|p| !subst.contains_key(p)) {
                continue;
            }
            let mut ok = true;
            for bound in &imp.decl.where_bounds {
                let Some(bound_ty) = subst.get(&bound.param) else {
                    ok = false;
                    break;
                };
                if !trait_impl_exists(
                    bound.trait_name.as_str(),
                    bound_ty,
                    self.trait_env,
                    self.aliases,
                )? {
                    ok = false;
                    break;
                }
            }
            if ok {
                matches.push((imp, subst));
            }
        }
        if matches.len() == 1 {
            Ok(matches.remove(0))
        } else if matches.is_empty() {
            let rendered = show_ty(self_ty.clone());
            Err(TyperError::missing_trait_bound(&rendered, trait_name, span).into())
        } else {
            let rendered = show_ty(self_ty.clone());
            let mut candidates: Vec<(String, Span)> = Vec::with_capacity(matches.len());
            for (imp, _) in &matches {
                candidates.push((show_ty(imp.decl.for_type.clone()), imp.decl.span));
            }
            Err(TyperError::ambiguous_impl(trait_name, &rendered, span, &candidates).into())
        }
    }
}
