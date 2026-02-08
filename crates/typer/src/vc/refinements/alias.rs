use std::collections::HashMap;

use clg_ast::{Program, Span, Type};

use crate::builtins::builtin_sigs;

#[derive(Clone)]
pub(crate) struct AliasView<'a> {
    pub(crate) name: &'a str,
    pub(crate) base: &'a Type,
    pub(crate) binder: Option<&'a str>,
    pub(crate) predicate: &'a clg_ast::Expr,
    pub(crate) span: Span,
}

#[derive(Clone)]
pub(crate) struct FnSigView<'a> {
    pub(crate) ret: Type,
    pub(crate) param_aliases: Vec<Option<&'a AliasView<'a>>>,
    pub(crate) ret_alias: Option<&'a AliasView<'a>>,
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
) -> HashMap<String, FnSigView<'a>> {
    let mut map: HashMap<String, FnSigView<'a>> = HashMap::new();
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
) -> Option<&'a AliasView<'a>> {
    if let Type::Named { name, args } = ty {
        if args.is_empty() {
            return aliases.get(name.as_str());
        }
    }
    None
}
