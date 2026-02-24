use clg_ast::Expr;

use super::alias::AliasInstance;
use super::substitute::substitute_binder;
use crate::vc::{RefinementAttachment, RefinementObligation};

pub(crate) fn make_refinement_obligation(
    alias: &AliasInstance,
    replacement: &Expr,
    attachment: RefinementAttachment,
) -> Option<RefinementObligation> {
    let binder = alias.alias_binder.as_deref()?;
    let predicate = substitute_binder(&alias.alias_predicate, binder, replacement);
    Some(RefinementObligation {
        alias: alias.alias_name.clone(),
        binder: binder.to_string(),
        substitution: replacement.clone(),
        substitution_type: alias.alias_base.clone(),
        predicate,
        attachment,
    })
}
