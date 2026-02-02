use std::collections::{HashMap, HashSet, VecDeque};

use anyhow::Result;
use clg_ast::{Block, Expr, Func, MatchPat, ParamKind, Program, Span, Stmt, Type, TypeParam};

use crate::errors::TyperError;

use super::expr::{ensure_trait_bound, infer_expr_type, trait_impl_exists, type_pattern_matches};
use super::{
    base_type, show_ty, substitute_type, type_param_names, unify_type_params, AliasMap, BoundsMap,
    FnSig, LocalBinding, TraitEnv, TypeDefs, TypeSubst,
};

pub(super) fn monomorphize_program<'a>(
    program: &'a Program,
    fns: &'a HashMap<&'a str, FnSig>,
    trait_env: &'a TraitEnv<'a>,
    aliases: &'a AliasMap,
    type_defs: &'a TypeDefs<'a>,
) -> Result<Program> {
    let mut base_funcs: HashMap<&'a str, &'a Func> = HashMap::with_capacity(program.funcs.len());
    for func in &program.funcs {
        base_funcs.insert(func.name.as_str(), func);
    }

    let mut mono = Monomorphizer {
        base_fns: fns,
        base_funcs,
        trait_env,
        aliases,
        type_defs,
        mono_funcs: Vec::new(),
        mono_map: HashMap::new(),
        queue: VecDeque::new(),
    };

    for func in &program.funcs {
        if func.type_params.is_empty() {
            let inst = instantiate_func(func, &TypeSubst::new(), func.name.clone());
            mono.register_func(inst);
        }
    }

    mono.process_queue()?;

    let mut out = program.clone();
    out.funcs = mono.mono_funcs;
    out.traits = Vec::new();
    out.impls = Vec::new();
    Ok(out)
}

struct Monomorphizer<'a> {
    base_fns: &'a HashMap<&'a str, FnSig>,
    base_funcs: HashMap<&'a str, &'a Func>,
    trait_env: &'a TraitEnv<'a>,
    aliases: &'a AliasMap,
    type_defs: &'a TypeDefs<'a>,
    mono_funcs: Vec<Func>,
    mono_map: HashMap<String, usize>,
    queue: VecDeque<usize>,
}

impl<'a> Monomorphizer<'a> {
    fn register_func(&mut self, func: Func) -> usize {
        if let Some(idx) = self.mono_map.get(&func.name).copied() {
            return idx;
        }
        let idx = self.mono_funcs.len();
        self.mono_map.insert(func.name.clone(), idx);
        self.mono_funcs.push(func);
        self.queue.push_back(idx);
        idx
    }

    fn process_queue(&mut self) -> Result<()> {
        while let Some(idx) = self.queue.pop_front() {
            let mut func = self
                .mono_funcs
                .get(idx)
                .cloned()
                .ok_or_else(|| anyhow::anyhow!("missing monomorphized function"))?;
            self.rewrite_func(&mut func)?;
            self.mono_funcs[idx] = func;
        }
        Ok(())
    }

    fn rewrite_func(&mut self, func: &mut Func) -> Result<()> {
        let type_params = type_param_names(&func.type_params);
        let bounds = bounds_map_from_where(&func.where_bounds);
        let mut env: HashMap<&str, LocalBinding> =
            HashMap::with_capacity(func.params.len() + 1);
        env.insert(
            "$return",
            LocalBinding {
                ty: func.ret.clone(),
                kind: ParamKind::Borrow,
            },
        );
        for param in &func.params {
            env.insert(
                param.name.as_str(),
                LocalBinding {
                    ty: param.ty.clone(),
                    kind: param.kind,
                },
            );
        }

        for req in &mut func.requires {
            self.rewrite_expr(&mut req.expr, &mut env, &type_params, &bounds)?;
        }

        self.rewrite_expr(&mut func.body, &mut env, &type_params, &bounds)?;

        if !func.ensures.is_empty() {
            env.insert(
                "result",
                LocalBinding {
                    ty: func.ret.clone(),
                    kind: ParamKind::Borrow,
                },
            );
            for ens in &mut func.ensures {
                self.rewrite_expr(&mut ens.expr, &mut env, &type_params, &bounds)?;
            }
            env.remove("result");
        }
        Ok(())
    }

    fn rewrite_block<'b>(
        &mut self,
        block: &'b mut Block,
        env: &mut HashMap<&'b str, LocalBinding>,
        type_params: &HashSet<String>,
        bounds: &BoundsMap,
    ) -> Result<()> {
        let mut inserted: Vec<ScopeEntry<'b>> = Vec::new();
        for stmt in &mut block.statements {
            match stmt {
                Stmt::Let { name, expr, .. } => {
                    let ty = infer_expr_type(
                        expr.as_ref(),
                        env,
                        self.base_fns,
                        self.trait_env,
                        self.aliases,
                        self.type_defs,
                        type_params,
                        bounds,
                    )?;
                    self.rewrite_expr(expr.as_mut(), env, type_params, bounds)?;
                    let key = name.as_str();
                    let prev = env.insert(
                        key,
                        LocalBinding {
                            ty,
                            kind: ParamKind::Borrow,
                        },
                    );
                    inserted.push(ScopeEntry { name: key, prev });
                }
                Stmt::Expr { expr, .. } => {
                    self.rewrite_expr(expr.as_mut(), env, type_params, bounds)?;
                }
                Stmt::While {
                    cond,
                    invariant,
                    variant,
                    body,
                    ..
                } => {
                    self.rewrite_expr(cond.as_mut(), env, type_params, bounds)?;
                    self.rewrite_expr(invariant.as_mut(), env, type_params, bounds)?;
                    if let Some(v) = variant.as_mut() {
                        self.rewrite_expr(v.as_mut(), env, type_params, bounds)?;
                    }
                    self.rewrite_block(body.as_mut(), env, type_params, bounds)?;
                }
            }
        }
        if let Some(tail) = block.tail.as_mut() {
            self.rewrite_expr(tail.as_mut(), env, type_params, bounds)?;
        }
        restore_scope(env, inserted);
        Ok(())
    }

    fn rewrite_expr<'b>(
        &mut self,
        expr: &'b mut Expr,
        env: &mut HashMap<&'b str, LocalBinding>,
        type_params: &HashSet<String>,
        bounds: &BoundsMap,
    ) -> Result<()> {
        match expr {
            Expr::Int(_, _) | Expr::Bool(_, _) | Expr::String(_, _) | Expr::Var(_, _) => Ok(()),
            Expr::ArrayLit { elems, .. } | Expr::TupleLit { elems, .. } => {
                for elem in elems {
                    self.rewrite_expr(elem, env, type_params, bounds)?;
                }
                Ok(())
            }
            Expr::StructLit { fields, .. } => {
                for field in fields {
                    self.rewrite_expr(&mut field.expr, env, type_params, bounds)?;
                }
                Ok(())
            }
            Expr::FieldAccess { base, .. } => {
                self.rewrite_expr(base.as_mut(), env, type_params, bounds)
            }
            Expr::Index { base, index, .. } => {
                self.rewrite_expr(base.as_mut(), env, type_params, bounds)?;
                self.rewrite_expr(index.as_mut(), env, type_params, bounds)
            }
            Expr::Block { block } => self.rewrite_block(block.as_mut(), env, type_params, bounds),
            Expr::Bin { lhs, rhs, .. } => {
                self.rewrite_expr(lhs.as_mut(), env, type_params, bounds)?;
                self.rewrite_expr(rhs.as_mut(), env, type_params, bounds)
            }
            Expr::Return { expr, .. } | Expr::Unary { expr, .. } | Expr::Try { expr, .. } => {
                self.rewrite_expr(expr.as_mut(), env, type_params, bounds)
            }
            Expr::If {
                cond,
                then_br,
                else_br,
                ..
            } => {
                self.rewrite_expr(cond.as_mut(), env, type_params, bounds)?;
                self.rewrite_expr(then_br.as_mut(), env, type_params, bounds)?;
                self.rewrite_expr(else_br.as_mut(), env, type_params, bounds)
            }
            Expr::Match {
                scrutinee, arms, ..
            } => {
                let scrut_ty = infer_expr_type(
                    scrutinee.as_ref(),
                    env,
                    self.base_fns,
                    self.trait_env,
                    self.aliases,
                    self.type_defs,
                    type_params,
                    bounds,
                )?;
                self.rewrite_expr(scrutinee.as_mut(), env, type_params, bounds)?;
                for arm in arms {
                    let mut arm_env = env.clone();
                    let binders =
                        pattern_binders(&arm.pat, &scrut_ty, self.type_defs, self.aliases)?;
                    for (name, ty) in binders {
                        arm_env.insert(
                            name,
                            LocalBinding {
                                ty,
                                kind: ParamKind::Borrow,
                            },
                        );
                    }
                    self.rewrite_expr(&mut arm.expr, &mut arm_env, type_params, bounds)?;
                }
                Ok(())
            }
            Expr::Call { callee, args, span } => {
                let arg_types: Vec<Type> = args
                    .iter()
                    .map(|arg| {
                        infer_expr_type(
                            arg,
                            env,
                            self.base_fns,
                            self.trait_env,
                            self.aliases,
                            self.type_defs,
                            type_params,
                            bounds,
                        )
                    })
                    .collect::<Result<Vec<_>>>()?;
                for arg in args.iter_mut() {
                    self.rewrite_expr(arg, env, type_params, bounds)?;
                }

                if is_special_callee(callee.as_str()) {
                    return Ok(());
                }

                if let Some((trait_name, method_name)) = callee.split_once("::") {
                    if self.trait_env.traits.contains_key(trait_name) {
                        let new_name = self.resolve_trait_call(
                            trait_name,
                            method_name,
                            &arg_types,
                            type_params,
                            bounds,
                            *span,
                        )?;
                        *callee = new_name;
                        return Ok(());
                    }
                }

                if self.is_enum_constructor(callee.as_str()) {
                    return Ok(());
                }

                if self.mono_map.contains_key(callee.as_str()) {
                    return Ok(());
                }

                if let Some(sig) = self.base_fns.get(callee.as_str()) {
                    if sig.type_params.is_empty() {
                        if let Some(base) = self.base_funcs.get(callee.as_str()) {
                            let inst =
                                instantiate_func(base, &TypeSubst::new(), callee.clone());
                            self.register_func(inst);
                        }
                        return Ok(());
                    }
                    let new_name = self.resolve_generic_call(
                        callee.as_str(),
                        sig,
                        &arg_types,
                        type_params,
                        bounds,
                        *span,
                    )?;
                    *callee = new_name;
                    return Ok(());
                }
                Ok(())
            }
        }
    }

    fn resolve_generic_call(
        &mut self,
        callee: &str,
        sig: &FnSig,
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

    fn resolve_trait_call(
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
            return Err(
                TyperError::arity_mismatch(
                    &format!("{}::{}", trait_name, method_name),
                    method.params.len(),
                    arg_types.len(),
                    span,
                )
                .into(),
            );
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
        let impl_method = imp
            .methods
            .get(method_name)
            .ok_or_else(|| anyhow::anyhow!("missing impl method `{}`", method_name))?;

        let mut full_subst = impl_subst;
        full_subst.insert("Self".to_string(), self_ty.clone());
        let mangled = mangle_impl_method_name(trait_name, &self_ty, method_name, self.aliases)?;
        if !self.mono_map.contains_key(mangled.as_str()) {
            let inst = instantiate_func(impl_method, &full_subst, mangled.clone());
            self.register_func(inst);
        }
        Ok(mangled)
    }

    fn find_impl(
        &self,
        trait_name: &str,
        self_ty: &Type,
        span: Span,
    ) -> Result<(&super::ImplInfo<'a>, TypeSubst)> {
        let mut matches: Vec<(&super::ImplInfo<'a>, TypeSubst)> = Vec::new();
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
            Err(
                TyperError::ambiguous_impl(trait_name, &rendered, span, &candidates).into(),
            )
        }
    }

    fn is_enum_constructor(&self, callee: &str) -> bool {
        let Some((enum_name, variant_name)) = callee.split_once("::") else {
            return false;
        };
        let Some(info) = self.type_defs.enums.get(enum_name) else {
            return false;
        };
        info.variants.contains_key(variant_name)
    }
}

fn bounds_map_from_where(bounds: &[clg_ast::TraitBound]) -> BoundsMap {
    let mut map: BoundsMap = HashMap::new();
    for bound in bounds {
        map.entry(bound.param.clone())
            .or_default()
            .insert(bound.trait_name.clone());
    }
    map
}

fn instantiate_func(base: &Func, subst: &TypeSubst, name: String) -> Func {
    let mut func = base.clone();
    func.name = name;
    func.type_params.clear();
    func.where_bounds.clear();
    for param in &mut func.params {
        param.ty = substitute_type(&param.ty, subst);
    }
    func.ret = substitute_type(&func.ret, subst);
    func
}

fn is_special_callee(name: &str) -> bool {
    matches!(name, "U8" | "U64" | "U128" | "U256" | "Some" | "None" | "Ok" | "Err")
}

fn build_type_param_subst(params: &[TypeParam], args: &[Type]) -> Result<TypeSubst> {
    if params.len() != args.len() {
        return Err(TyperError::type_arg_count_mismatch(
            "type",
            params.len(),
            args.len(),
            None,
        )
        .into());
    }
    let mut subst: TypeSubst = HashMap::new();
    for (param, arg) in params.iter().zip(args.iter()) {
        subst.insert(param.name.clone(), arg.clone());
    }
    Ok(subst)
}

fn pattern_binders<'a>(
    pat: &'a MatchPat,
    scrutinee_ty: &Type,
    type_defs: &TypeDefs,
    aliases: &AliasMap,
) -> Result<Vec<(&'a str, Type)>> {
    let resolved = base_type(scrutinee_ty, aliases)?;
    match pat {
        MatchPat::Some(name) => {
            if let Type::Option(inner) = resolved {
                Ok(vec![(name.as_str(), *inner)])
            } else {
                Ok(Vec::new())
            }
        }
        MatchPat::None | MatchPat::Wildcard => Ok(Vec::new()),
        MatchPat::Ok(name) => {
            if let Type::Result(ok, _) = resolved {
                Ok(vec![(name.as_str(), *ok)])
            } else {
                Ok(Vec::new())
            }
        }
        MatchPat::Err(name) => {
            if let Type::Result(_, err) = resolved {
                Ok(vec![(name.as_str(), *err)])
            } else {
                Ok(Vec::new())
            }
        }
        MatchPat::EnumVariant {
            enum_name,
            variant,
            binders,
        } => {
            let Type::Named { name, args } = resolved else {
                return Ok(Vec::new());
            };
            if name != *enum_name {
                return Ok(Vec::new());
            }
            let fields = enum_variant_fields(type_defs, enum_name, &args, variant)?;
            let mut out = Vec::new();
            for (binder, ty) in binders.iter().zip(fields.iter()) {
                out.push((binder.as_str(), ty.clone()));
            }
            Ok(out)
        }
    }
}

fn enum_variant_fields(
    type_defs: &TypeDefs,
    enum_name: &str,
    args: &[Type],
    variant_name: &str,
) -> Result<Vec<Type>> {
    let info = type_defs
        .enums
        .get(enum_name)
        .ok_or_else(|| anyhow::anyhow!("unknown enum `{}`", enum_name))?;
    let subst = build_type_param_subst(&info.decl.type_params, args)?;
    let variant = info
        .variants
        .get(variant_name)
        .ok_or_else(|| anyhow::anyhow!("unknown enum variant `{}`", variant_name))?;
    let mut fields = Vec::with_capacity(variant.fields.len());
    for ty in &variant.fields {
        fields.push(substitute_type(ty, &subst));
    }
    Ok(fields)
}

struct ScopeEntry<'a> {
    name: &'a str,
    prev: Option<LocalBinding>,
}

fn restore_scope<'a>(env: &mut HashMap<&'a str, LocalBinding>, inserted: Vec<ScopeEntry<'a>>) {
    for entry in inserted.into_iter().rev() {
        if let Some(prev) = entry.prev {
            env.insert(entry.name, prev);
        } else {
            env.remove(entry.name);
        }
    }
}

fn mangle_fn_name(name: &str, args: &[Type], aliases: &AliasMap) -> Result<String> {
    if args.is_empty() {
        return Ok(name.to_string());
    }
    let mut parts = Vec::with_capacity(args.len());
    for arg in args {
        parts.push(mangle_type(arg, aliases)?);
    }
    Ok(format!("{}${}", name, parts.join("$")))
}

fn mangle_impl_method_name(
    trait_name: &str,
    self_ty: &Type,
    method: &str,
    aliases: &AliasMap,
) -> Result<String> {
    let self_name = mangle_type(self_ty, aliases)?;
    Ok(format!("impl${}${}${}", trait_name, self_name, method))
}

fn mangle_type(ty: &Type, _aliases: &AliasMap) -> Result<String> {
    Ok(match ty {
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
                name.clone()
            } else {
                let mut parts = Vec::with_capacity(args.len() + 2);
                parts.push(name.clone());
                parts.push(args.len().to_string());
                for arg in args {
                    parts.push(mangle_type(arg, _aliases)?);
                }
                format!("N${}", parts.join("$"))
            }
        }
        Type::Option(inner) => format!("Option${}", mangle_type(inner, _aliases)?),
        Type::Result(ok, err) => format!(
            "Result${}${}",
            mangle_type(ok, _aliases)?,
            mangle_type(err, _aliases)?
        ),
        Type::List(inner) => format!("List${}", mangle_type(inner, _aliases)?),
        Type::Set(inner) => format!("Set${}", mangle_type(inner, _aliases)?),
        Type::Map(k, v) => format!(
            "Map${}${}",
            mangle_type(k, _aliases)?,
            mangle_type(v, _aliases)?
        ),
        Type::Array(inner, len) => {
            format!("Array${}${}", len, mangle_type(inner, _aliases)?)
        }
        Type::Tuple(elements) => {
            let mut parts = Vec::with_capacity(elements.len() + 1);
            parts.push(elements.len().to_string());
            for elem in elements {
                parts.push(mangle_type(elem, _aliases)?);
            }
            format!("Tuple${}", parts.join("$"))
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use clg_ast::ImplDecl;
    use crate::check::ImplInfo;

    #[test]
    fn find_impl_uses_call_span_for_missing_impl() {
        let trait_env = TraitEnv {
            traits: HashMap::new(),
            impls: Vec::new(),
        };
        let aliases: AliasMap = HashMap::new();
        let type_defs = TypeDefs {
            resources: HashSet::new(),
            structs: HashMap::new(),
            enums: HashMap::new(),
        };
        let base_fns: HashMap<&str, FnSig> = HashMap::new();
        let base_funcs: HashMap<&str, &Func> = HashMap::new();
        let mono = Monomorphizer {
            base_fns: &base_fns,
            base_funcs,
            trait_env: &trait_env,
            aliases: &aliases,
            type_defs: &type_defs,
            mono_funcs: Vec::new(),
            mono_map: HashMap::new(),
            queue: VecDeque::new(),
        };
        let span = Span { start: 12, end: 34 };
        let err = match mono.find_impl("Eq", &Type::Int, span) {
            Ok(_) => panic!("expected missing impl"),
            Err(err) => err,
        };
        let te = err.downcast_ref::<TyperError>().expect("typer error");
        assert_eq!(te.code, "T237");
        assert_eq!(te.start, span.start);
        assert_eq!(te.end, span.end);
    }

    #[test]
    fn find_impl_uses_call_span_for_ambiguous_impl() {
        let impl1 = ImplDecl {
            trait_name: "Eq".to_string(),
            trait_name_span: Span { start: 0, end: 0 },
            type_params: Vec::new(),
            for_type: Type::Int,
            where_bounds: Vec::new(),
            methods: Vec::new(),
            span: Span { start: 1, end: 2 },
        };
        let impl2 = ImplDecl {
            trait_name: "Eq".to_string(),
            trait_name_span: Span { start: 0, end: 0 },
            type_params: Vec::new(),
            for_type: Type::Int,
            where_bounds: Vec::new(),
            methods: Vec::new(),
            span: Span { start: 3, end: 4 },
        };
        let trait_env = TraitEnv {
            traits: HashMap::new(),
            impls: vec![
                ImplInfo {
                    decl: &impl1,
                    methods: HashMap::new(),
                },
                ImplInfo {
                    decl: &impl2,
                    methods: HashMap::new(),
                },
            ],
        };
        let aliases: AliasMap = HashMap::new();
        let type_defs = TypeDefs {
            resources: HashSet::new(),
            structs: HashMap::new(),
            enums: HashMap::new(),
        };
        let base_fns: HashMap<&str, FnSig> = HashMap::new();
        let base_funcs: HashMap<&str, &Func> = HashMap::new();
        let mono = Monomorphizer {
            base_fns: &base_fns,
            base_funcs,
            trait_env: &trait_env,
            aliases: &aliases,
            type_defs: &type_defs,
            mono_funcs: Vec::new(),
            mono_map: HashMap::new(),
            queue: VecDeque::new(),
        };
        let span = Span { start: 55, end: 89 };
        let err = match mono.find_impl("Eq", &Type::Int, span) {
            Ok(_) => panic!("expected ambiguous impl"),
            Err(err) => err,
        };
        let te = err.downcast_ref::<TyperError>().expect("typer error");
        assert_eq!(te.code, "T248");
        assert_eq!(te.start, span.start);
        assert_eq!(te.end, span.end);
        assert!(te.message.contains("candidates"));
        assert!(te.message.contains("impl for `Int`"));
        assert!(te.message.contains("1..2"));
        assert!(te.message.contains("3..4"));
    }

    #[test]
    fn mangled_names_use_identifier_safe_chars() {
        let aliases: AliasMap = HashMap::new();
        let types = vec![
            Type::Int,
            Type::Bool,
            Type::Named {
                name: "Box".to_string(),
                args: vec![Type::U8],
            },
            Type::Option(Box::new(Type::Named {
                name: "Pair".to_string(),
                args: vec![Type::U64, Type::String],
            })),
            Type::Result(Box::new(Type::Int), Box::new(Type::U256)),
            Type::List(Box::new(Type::Bytes)),
            Type::Set(Box::new(Type::U128)),
            Type::Map(Box::new(Type::U8), Box::new(Type::Named {
                name: "Thing".to_string(),
                args: vec![Type::Bool],
            })),
            Type::Array(Box::new(Type::U64), 4),
            Type::Tuple(vec![Type::Int, Type::Bool, Type::U8]),
        ];
        for ty in types {
            let name = mangle_type(&ty, &aliases).expect("mangle type");
            assert!(
                name.chars()
                    .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '$'),
                "mangled name contained disallowed char: {name}"
            );
        }
        let fn_name = mangle_fn_name(
            "do_work",
            &[
                Type::Named {
                    name: "Box".to_string(),
                    args: vec![Type::U8],
                },
                Type::Tuple(vec![Type::Int, Type::U64]),
            ],
            &aliases,
        )
        .expect("mangle fn");
        assert!(
            fn_name
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '$'),
            "mangled fn name contained disallowed char: {fn_name}"
        );
        let impl_name = mangle_impl_method_name("Eq", &Type::Int, "eq", &aliases)
            .expect("mangle impl");
        assert!(
            impl_name
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '$'),
            "mangled impl name contained disallowed char: {impl_name}"
        );
    }
}
