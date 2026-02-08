use anyhow::Result;

use crate::errors::TyperError;
use crate::guards::{collect_mut_calls, guard_callee_for_kind, MutCall, MutGuardKey};
use clg_ast::Expr;
use std::collections::HashSet;

pub(super) fn enforce_mut_guards(expr: &Expr, guards: &HashSet<MutGuardKey>) -> Result<()> {
    let mut calls: Vec<MutCall> = Vec::new();
    collect_mut_calls(expr, &mut calls);
    for call in calls {
        let guard_name = guard_callee_for_kind(call.kind);
        let arg_name = match call.target {
            Some(name) => name,
            None => {
                return Err(TyperError::mut_guard_requires_variable(
                    call.callee.as_str(),
                    guard_name,
                    call.span,
                )
                .into());
            }
        };
        let key = MutGuardKey {
            kind: call.kind,
            target: arg_name.clone(),
        };
        if !guards.contains(&key) {
            return Err(TyperError::mut_guard_missing(
                call.callee.as_str(),
                guard_name,
                &arg_name,
                call.span,
            )
            .into());
        }
    }
    Ok(())
}
