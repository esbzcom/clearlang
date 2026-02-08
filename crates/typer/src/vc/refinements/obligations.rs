use clg_ast::Expr;

use super::alias::AliasView;
use super::substitute::substitute_binder;
use crate::vc::{RefinementAttachment, RefinementObligation};

pub(crate) fn make_refinement_obligation(
    alias: &AliasView<'_>,
    replacement: &Expr,
    attachment: RefinementAttachment,
) -> Option<RefinementObligation> {
    let binder = alias.binder?;
    let predicate = substitute_binder(alias.predicate, binder, replacement);
    Some(RefinementObligation {
        alias: alias.name.to_string(),
        binder: binder.to_string(),
        substitution: replacement.clone(),
        substitution_type: alias.base.clone(),
        predicate,
        attachment,
    })
}
