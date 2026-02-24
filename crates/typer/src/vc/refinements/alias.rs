use std::collections::HashMap;

use clg_ast::{Expr, Program, Span, Type};

use crate::builtins::builtin_sigs;

#[derive(Clone)]
pub(crate) struct AliasView<'a> {
    pub(crate) name: &'a str,
    pub(crate) type_params: &'a [String],
    pub(crate) base: &'a Type,
    pub(crate) binder: Option<&'a str>,
    pub(crate) predicate: &'a Expr,
    pub(crate) span: Span,
}

#[derive(Clone)]
pub(crate) struct AliasInstance {
    pub(crate) alias_name: String,
    pub(crate) alias_base: Type,
    pub(crate) alias_binder: Option<String>,
    pub(crate) alias_predicate: Expr,
    pub(crate) alias_span: Span,
}

#[derive(Clone)]
pub(crate) struct FnSigView {
    pub(crate) ret: Type,
    pub(crate) param_aliases: Vec<Option<AliasInstance>>,
    pub(crate) ret_alias: Option<AliasInstance>,
}

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

pub(crate) fn build_alias_map<'a>(program: &'a Program) -> HashMap<&'a str, AliasView<'a>> {
    program
        .refined_aliases
        .iter()
        .map(|a| {
            (
                a.name.as_str(),
                AliasView {
                    name: a.name.as_str(),
                    type_params: &a.type_params,
                    base: &a.base,
                    binder: a.binder.as_deref(),
                    predicate: &a.predicate,
                    span: a.span,
                },
            )
        })
        .collect()
}

pub(crate) fn build_fn_sigs<'a>(
    program: &'a Program,
    aliases: &'a HashMap<&'a str, AliasView<'a>>,
) -> HashMap<String, FnSigView> {
    let mut map: HashMap<String, FnSigView> = HashMap::new();
    for func in &program.funcs {
        let param_aliases = func
            .params
            .iter()
            .map(|p| top_level_alias(&p.ty, aliases))
            .collect();
        let ret_alias = top_level_alias(&func.ret, aliases);
        map.insert(
            func.name.clone(),
            FnSigView {
                ret: func.ret.clone(),
                param_aliases,
                ret_alias,
            },
        );
    }

    for (name, params, ret, _) in builtin_sigs() {
        map.entry(name.clone()).or_insert_with(|| FnSigView {
            param_aliases: params
                .iter()
                .map(|p| top_level_alias(&p.ty, aliases))
                .collect(),
            ret: ret.clone(),
            ret_alias: top_level_alias(&ret, aliases),
        });
    }
    map
}

pub(super) fn top_level_alias<'a>(
    ty: &Type,
    aliases: &'a HashMap<&'a str, AliasView<'a>>,
) -> Option<AliasInstance> {
    if let Type::Named { name, args } = ty {
        let alias = aliases.get(name.as_str())?;
        if alias.type_params.len() != args.len() {
            return None;
        }
        let mut subst = HashMap::with_capacity(alias.type_params.len());
        for (param, arg) in alias.type_params.iter().zip(args.iter()) {
            subst.insert(param.clone(), arg.clone());
        }
        return Some(AliasInstance {
            alias_name: alias.name.to_string(),
            alias_base: instantiate_type(alias.base, &subst),
            alias_binder: alias.binder.map(str::to_string),
            alias_predicate: alias.predicate.clone(),
            alias_span: alias.span,
        });
    }
    None
}
