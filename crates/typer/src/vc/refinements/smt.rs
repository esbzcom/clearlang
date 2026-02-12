use std::collections::HashMap;

use clg_ast::Type;

use super::alias::AliasView;
use crate::vc::{snapshot_expr, RefinementObligation};

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
                if args.is_empty() {
                    if let Some(alias) = aliases.get(name.as_str()) {
                        return smt_sort_for_type(alias.base, aliases);
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
