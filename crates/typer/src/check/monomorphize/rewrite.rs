use std::collections::{HashMap, HashSet};

use anyhow::Result;
use clg_ast::{Block, Expr, ParamKind, Stmt, Type};

use super::super::{infer_expr_type, BoundsMap, LocalBinding};
use super::helpers::{
    bounds_map_from_where, is_special_callee, pattern_binders, restore_scope, ScopeEntry,
};
use super::Monomorphizer;

impl<'a> Monomorphizer<'a> {
    pub(super) fn rewrite_func(&mut self, func: &mut clg_ast::Func) -> Result<()> {
        let type_params = super::super::type_param_names(&func.type_params);
        let bounds = bounds_map_from_where(&func.where_bounds);
        let mut env: HashMap<&str, LocalBinding> = HashMap::with_capacity(func.params.len() + 1);
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

                if let Some((trait_name, method_name)) = callee.rsplit_once("::") {
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
                            let inst = super::helpers::instantiate_func(
                                base,
                                &HashMap::new(),
                                callee.clone(),
                            );
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
            Expr::Lambda { body, .. } => self.rewrite_expr(body.as_mut(), env, type_params, bounds),
        }
    }
}
