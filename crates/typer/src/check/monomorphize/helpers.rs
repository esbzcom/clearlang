use std::collections::HashMap;

use anyhow::Result;
use clg_ast::{Func, MatchPat, Type, TypeParam};

use crate::errors::TyperError;

use super::super::{base_type, substitute_type, BoundsMap, LocalBinding, TypeSubst};
use super::{AliasMap, TypeDefs};

pub(super) fn bounds_map_from_where(bounds: &[clg_ast::TraitBound]) -> BoundsMap {
    let mut map: BoundsMap = HashMap::new();
    for bound in bounds {
        map.entry(bound.param.clone())
            .or_default()
            .insert(bound.trait_name.clone());
    }
    map
}

pub(super) fn instantiate_func(base: &Func, subst: &TypeSubst, name: String) -> Func {
    let mut func = base.clone();
    func.name = name;
    func.type_params.clear();
    func.where_bounds.clear();
    for param in &mut func.params {
        param.ty = substitute_type(&param.ty, subst);
    }
    func.ret = substitute_type(&func.ret, subst);
    for req in &mut func.requires {
        substitute_expr_type_args(&mut req.expr, subst);
    }
    substitute_expr_type_args(&mut func.body, subst);
    for ens in &mut func.ensures {
        substitute_expr_type_args(&mut ens.expr, subst);
    }
    func
}

pub(super) fn is_special_callee(name: &str) -> bool {
    matches!(
        name,
        "U8" | "U64" | "U128" | "U256" | "Some" | "None" | "Ok" | "Err"
    )
}

fn build_type_param_subst(params: &[TypeParam], args: &[Type]) -> Result<TypeSubst> {
    if params.len() != args.len() {
        return Err(
            TyperError::type_arg_count_mismatch("type", params.len(), args.len(), None).into(),
        );
    }
    let mut subst: TypeSubst = HashMap::new();
    for (param, arg) in params.iter().zip(args.iter()) {
        subst.insert(param.name.clone(), arg.clone());
    }
    Ok(subst)
}

pub(super) fn pattern_binders<'a>(
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

pub(super) struct ScopeEntry<'a> {
    pub(super) name: &'a str,
    pub(super) prev: Option<LocalBinding>,
}

pub(super) fn restore_scope<'a>(
    env: &mut HashMap<&'a str, LocalBinding>,
    inserted: Vec<ScopeEntry<'a>>,
) {
    for entry in inserted.into_iter().rev() {
        if let Some(prev) = entry.prev {
            env.insert(entry.name, prev);
        } else {
            env.remove(entry.name);
        }
    }
}

fn substitute_expr_type_args(expr: &mut clg_ast::Expr, subst: &TypeSubst) {
    match expr {
        clg_ast::Expr::Int(_, _)
        | clg_ast::Expr::Bool(_, _)
        | clg_ast::Expr::String(_, _)
        | clg_ast::Expr::Var(_, _) => {}
        clg_ast::Expr::ArrayLit { elems, .. } | clg_ast::Expr::TupleLit { elems, .. } => {
            for elem in elems {
                substitute_expr_type_args(elem, subst);
            }
        }
        clg_ast::Expr::StructLit { fields, .. } => {
            for field in fields {
                substitute_expr_type_args(&mut field.expr, subst);
            }
        }
        clg_ast::Expr::FieldAccess { base, .. } => substitute_expr_type_args(base, subst),
        clg_ast::Expr::Block { block } => substitute_block_type_args(block, subst),
        clg_ast::Expr::Bin { lhs, rhs, .. } => {
            substitute_expr_type_args(lhs, subst);
            substitute_expr_type_args(rhs, subst);
        }
        clg_ast::Expr::Call {
            type_args, args, ..
        } => {
            for ty in type_args {
                *ty = substitute_type(ty, subst);
            }
            for arg in args {
                substitute_expr_type_args(arg, subst);
            }
        }
        clg_ast::Expr::Return { expr, .. }
        | clg_ast::Expr::Unary { expr, .. }
        | clg_ast::Expr::Try { expr, .. } => substitute_expr_type_args(expr, subst),
        clg_ast::Expr::Match {
            scrutinee, arms, ..
        } => {
            substitute_expr_type_args(scrutinee, subst);
            for arm in arms {
                substitute_expr_type_args(&mut arm.expr, subst);
            }
        }
        clg_ast::Expr::If {
            cond,
            then_br,
            else_br,
            ..
        } => {
            substitute_expr_type_args(cond, subst);
            substitute_expr_type_args(then_br, subst);
            substitute_expr_type_args(else_br, subst);
        }
        clg_ast::Expr::Index { base, index, .. } => {
            substitute_expr_type_args(base, subst);
            substitute_expr_type_args(index, subst);
        }
        clg_ast::Expr::Lambda { body, .. } => substitute_expr_type_args(body, subst),
    }
}

fn substitute_block_type_args(block: &mut clg_ast::Block, subst: &TypeSubst) {
    for stmt in &mut block.statements {
        match stmt {
            clg_ast::Stmt::Let { expr, .. } | clg_ast::Stmt::Expr { expr, .. } => {
                substitute_expr_type_args(expr, subst);
            }
            clg_ast::Stmt::While {
                cond,
                invariant,
                variant,
                body,
                ..
            } => {
                substitute_expr_type_args(cond, subst);
                substitute_expr_type_args(invariant, subst);
                if let Some(variant) = variant {
                    substitute_expr_type_args(variant, subst);
                }
                substitute_block_type_args(body, subst);
            }
        }
    }
    if let Some(tail) = &mut block.tail {
        substitute_expr_type_args(tail, subst);
    }
}
