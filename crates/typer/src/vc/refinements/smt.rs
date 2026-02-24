use std::collections::HashMap;

use clg_ast::Type;

use super::alias::AliasView;
use crate::vc::{snapshot_expr, RefinementObligation};

fn instantiate_type(ty: &Type, subst: &HashMap<String, Type>) -> Type {
    match ty {
        Type::Named { name, args } => {
            if args.is_empty() {
                if let Some(mapped) = subst.get(name) {
                    return mapped.clone();
                }
            }
            Type::Named {
                name: name.clone(),
                args: args
                    .iter()
                    .map(|arg| instantiate_type(arg, subst))
                    .collect(),
            }
        }
        Type::Option(inner) => Type::Option(Box::new(instantiate_type(inner, subst))),
        Type::Result(ok, err) => Type::Result(
            Box::new(instantiate_type(ok, subst)),
            Box::new(instantiate_type(err, subst)),
        ),
        Type::List(inner) => Type::List(Box::new(instantiate_type(inner, subst))),
        Type::Set(inner) => Type::Set(Box::new(instantiate_type(inner, subst))),
        Type::Map(k, v) => Type::Map(
            Box::new(instantiate_type(k, subst)),
            Box::new(instantiate_type(v, subst)),
        ),
        Type::Array(inner, len) => Type::Array(Box::new(instantiate_type(inner, subst)), *len),
        Type::Slice(inner) => Type::Slice(Box::new(instantiate_type(inner, subst))),
        Type::Tuple(elements) => Type::Tuple(
            elements
                .iter()
                .map(|elem| instantiate_type(elem, subst))
                .collect(),
        ),
        Type::Fn { params, ret } => Type::Fn {
            params: params
                .iter()
                .map(|param| instantiate_type(param, subst))
                .collect(),
            ret: Box::new(instantiate_type(ret, subst)),
        },
        _ => ty.clone(),
    }
}

pub(super) fn smt_sort_for_type(ty: &Type, aliases: &HashMap<&str, AliasView<'_>>) -> &'static str {
    match ty {
        Type::Int => "Int",
        Type::U8 => "Int",
        Type::U64 | Type::U128 | Type::U256 => "Int",
        Type::Bool => "Bool",
        Type::String => "String",
        Type::Bytes => "String",
        Type::Option(_)
        | Type::Result(_, _)
        | Type::List(_)
        | Type::Set(_)
        | Type::Map(_, _)
        | Type::Array(_, _)
        | Type::Slice(_)
        | Type::Tuple(_)
        | Type::Fn { .. }
        | Type::Named { .. } => {
            if let Type::Named { name, args } = ty {
                if let Some(alias) = aliases.get(name.as_str()) {
                    if alias.type_params.len() == args.len() {
                        let mut subst = HashMap::with_capacity(alias.type_params.len());
                        for (param, arg) in alias.type_params.iter().zip(args.iter()) {
                            subst.insert(param.clone(), arg.clone());
                        }
                        let instantiated = instantiate_type(alias.base, &subst);
                        return smt_sort_for_type(&instantiated, aliases);
                    }
                }
            }
            "Int"
        }
    }
}

pub(crate) fn refinement_prelude(
    obligations: &[RefinementObligation],
    extra: Option<&RefinementObligation>,
    aliases: &HashMap<&str, AliasView<'_>>,
) -> String {
    let total = obligations.len() + usize::from(extra.is_some());
    if total == 0 {
        return String::new();
    }

    let mut lines = Vec::new();
    let iter = obligations.iter().chain(extra);
    for (idx, obligation) in iter.enumerate() {
        let sort = smt_sort_for_type(&obligation.substitution_type, aliases);
        let substitution = snapshot_expr(&obligation.substitution);
        let predicate = snapshot_expr(&obligation.predicate);
        lines.push(format!(
            "; refinement ref:{} alias {} binder {}",
            idx, obligation.alias, obligation.binder
        ));
        lines.push(format!(
            "(define-fun cl.ref.premise.{}.sub () {} {})",
            idx, sort, substitution.smt2
        ));
        lines.push(format!(
            "(define-fun cl.ref.premise.{}.pred () Bool {})",
            idx, predicate.smt2
        ));
    }
    lines.join("\n")
}

pub(crate) fn merge_extras(parts: &[&str]) -> String {
    let mut out = Vec::new();
    for part in parts {
        let trimmed = part.trim();
        if !trimmed.is_empty() {
            out.push(trimmed);
        }
    }
    out.join("\n")
}
