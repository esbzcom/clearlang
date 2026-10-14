use anyhow::Result;
use clg_ast::{Block, ContractDecl, Expr, ImportKind, Program, Span, Stmt, Type};

use crate::errors::TyperError;

const ADDRESS: &str = "std::eth::Address";
const CALLER: &str = "std::eth::context::caller";
const EQUALS: &str = "std::eth::address::equals";

pub(super) fn validate_caller_authorization_profile(program: &Program) -> Result<()> {
    let imported_address = program.imports.iter().any(|import| {
        import.path.iter().map(String::as_str).eq(["std", "eth"])
            && matches!(&import.kind, ImportKind::Items { items } if items.iter().any(|item| item.name == "Address"))
    });

    for contract in &program.contracts {
        validate_contract(contract, imported_address)?;
    }
    for function in &program.funcs {
        if program.contracts.iter().any(|contract| {
            contract
                .functions
                .iter()
                .any(|member| member.name == function.name)
        }) {
            continue;
        }
        for require in &function.requires {
            reject_caller_surface(&require.expr)?;
        }
        for ensure in &function.ensures {
            reject_caller_surface(&ensure.expr)?;
        }
        reject_caller_surface(&function.body)?;
    }
    Ok(())
}

fn validate_contract(contract: &ContractDecl, imported_address: bool) -> Result<()> {
    let address_fields = contract
        .fields
        .iter()
        .filter(|field| is_address(&field.ty, imported_address))
        .collect::<Vec<_>>();
    if address_fields.is_empty() {
        for function in &contract.functions {
            reject_function_caller_surface(function)?;
        }
        if let Some(init) = &contract.init {
            reject_caller_surface(&init.body)?;
        }
        return Ok(());
    }
    if address_fields.len() != 1 || address_fields[0].name != "owner" {
        return Err(
            TyperError::caller_authorization_outside_profile(address_fields[0].span).into(),
        );
    }
    for field in &contract.fields {
        if contains_address(&field.ty, imported_address) && field.name != "owner" {
            return Err(TyperError::caller_authorization_outside_profile(field.span).into());
        }
    }
    let init = contract
        .init
        .as_ref()
        .ok_or_else(|| TyperError::caller_authorization_outside_profile(address_fields[0].span))?;
    if init
        .params
        .iter()
        .any(|param| contains_address(&param.ty, imported_address) && param.name != "owner")
        || init
            .params
            .iter()
            .filter(|param| is_address(&param.ty, imported_address))
            .count()
            != 1
    {
        return Err(TyperError::caller_authorization_outside_profile(init.span).into());
    }
    reject_caller_surface(&init.body)?;
    for event in &contract.events {
        if event
            .fields
            .iter()
            .any(|field| contains_address(&field.ty, imported_address))
        {
            return Err(TyperError::caller_authorization_outside_profile(event.span).into());
        }
    }
    for invariant in &contract.invariants {
        reject_caller_surface(&invariant.expr)?;
        reject_owner_address_use(&invariant.expr, invariant.span)?;
    }
    for function in &contract.functions {
        for param in &function.params {
            if contains_address(&param.ty, imported_address) {
                return Err(
                    TyperError::caller_authorization_outside_profile(function_span(function))
                        .into(),
                );
            }
        }
        if contains_address(&function.ret, imported_address) {
            return Err(
                TyperError::caller_authorization_outside_profile(function_span(function)).into(),
            );
        }
        validate_transition_guard(function)?;
    }
    Ok(())
}

fn validate_transition_guard(function: &clg_ast::Func) -> Result<()> {
    let mut caller_guards = 0usize;
    for require in &function.requires {
        if is_owner_guard(&require.expr) {
            caller_guards += 1;
        } else {
            reject_caller_surface(&require.expr)?;
            reject_owner_address_use(&require.expr, require.span)?;
        }
    }
    if caller_guards > 0 && (function.effect != clg_ast::Effect::Mut || caller_guards != 1) {
        return Err(
            TyperError::caller_authorization_outside_profile(function_span(function)).into(),
        );
    }
    for ensure in &function.ensures {
        reject_caller_surface(&ensure.expr)?;
        reject_owner_address_use(&ensure.expr, ensure.span)?;
    }
    reject_caller_surface(&function.body)?;
    reject_owner_address_use(&function.body, function_span(function))
}

fn reject_function_caller_surface(function: &clg_ast::Func) -> Result<()> {
    for require in &function.requires {
        reject_caller_surface(&require.expr)?;
    }
    for ensure in &function.ensures {
        reject_caller_surface(&ensure.expr)?;
    }
    reject_caller_surface(&function.body)
}

fn is_owner_guard(expr: &Expr) -> bool {
    matches!(expr, Expr::Call { callee, args, .. } if callee == EQUALS && matches!(args.as_slice(), [Expr::Call { callee: caller, args: caller_args, .. }, Expr::FieldAccess { base, field, .. }] if caller == CALLER && caller_args.is_empty() && field == "owner" && matches!(base.as_ref(), Expr::Var(name, _) if name == "state")))
}

fn reject_caller_surface(expr: &Expr) -> Result<()> {
    match expr {
        Expr::Call { callee, span, .. } if callee == CALLER || callee == EQUALS => {
            Err(TyperError::caller_authorization_outside_profile(*span).into())
        }
        Expr::ArrayLit { elems, .. } | Expr::TupleLit { elems, .. } => {
            for element in elems {
                reject_caller_surface(element)?;
            }
            Ok(())
        }
        Expr::StructLit { fields, .. } => {
            for field in fields {
                reject_caller_surface(&field.expr)?;
            }
            Ok(())
        }
        Expr::FieldAccess { base, .. }
        | Expr::Return { expr: base, .. }
        | Expr::Unary { expr: base, .. }
        | Expr::Try { expr: base, .. } => reject_caller_surface(base),
        Expr::Block { block } => reject_block_caller_surface(block),
        Expr::Bin { lhs, rhs, .. } => {
            reject_caller_surface(lhs)?;
            reject_caller_surface(rhs)
        }
        Expr::Call { args, .. } => {
            for arg in args {
                reject_caller_surface(arg)?;
            }
            Ok(())
        }
        Expr::Match {
            scrutinee, arms, ..
        } => {
            reject_caller_surface(scrutinee)?;
            for arm in arms {
                reject_caller_surface(&arm.expr)?;
            }
            Ok(())
        }
        Expr::If {
            cond,
            then_br,
            else_br,
            ..
        } => {
            reject_caller_surface(cond)?;
            reject_caller_surface(then_br)?;
            reject_caller_surface(else_br)
        }
        Expr::Index { base, index, .. } => {
            reject_caller_surface(base)?;
            reject_caller_surface(index)
        }
        Expr::Lambda { body, .. } => reject_caller_surface(body),
        Expr::Int(..) | Expr::Bool(..) | Expr::String(..) | Expr::Var(..) => Ok(()),
    }
}

fn reject_block_caller_surface(block: &Block) -> Result<()> {
    for statement in &block.statements {
        match statement {
            Stmt::Let { expr, .. } | Stmt::Expr { expr, .. } => reject_caller_surface(expr)?,
            Stmt::While {
                cond,
                invariant,
                body,
                ..
            } => {
                reject_caller_surface(cond)?;
                reject_caller_surface(invariant)?;
                reject_block_caller_surface(body)?;
            }
        }
    }
    if let Some(tail) = &block.tail {
        reject_caller_surface(tail)?;
    }
    Ok(())
}

fn reject_owner_address_use(expr: &Expr, span: Span) -> Result<()> {
    if contains_owner_field(expr) {
        Err(TyperError::caller_authorization_outside_profile(span).into())
    } else {
        Ok(())
    }
}

fn contains_owner_field(expr: &Expr) -> bool {
    match expr {
        Expr::FieldAccess { base, field, .. } => {
            field == "owner" && matches!(base.as_ref(), Expr::Var(name, _) if name == "state")
                || contains_owner_field(base)
        }
        Expr::ArrayLit { elems, .. } | Expr::TupleLit { elems, .. } => {
            elems.iter().any(contains_owner_field)
        }
        Expr::StructLit { fields, .. } => {
            fields.iter().any(|field| contains_owner_field(&field.expr))
        }
        Expr::Block { block } => {
            block.statements.iter().any(|statement| match statement {
                Stmt::Let { expr, .. } | Stmt::Expr { expr, .. } => contains_owner_field(expr),
                Stmt::While {
                    cond,
                    invariant,
                    body,
                    ..
                } => {
                    contains_owner_field(cond)
                        || contains_owner_field(invariant)
                        || block_contains_owner_field(body)
                }
            }) || block.tail.as_deref().is_some_and(contains_owner_field)
        }
        Expr::Bin { lhs, rhs, .. } => contains_owner_field(lhs) || contains_owner_field(rhs),
        Expr::Call { args, .. } => args.iter().any(contains_owner_field),
        Expr::Return { expr, .. }
        | Expr::Unary { expr, .. }
        | Expr::Try { expr, .. }
        | Expr::Lambda { body: expr, .. } => contains_owner_field(expr),
        Expr::Match {
            scrutinee, arms, ..
        } => {
            contains_owner_field(scrutinee)
                || arms.iter().any(|arm| contains_owner_field(&arm.expr))
        }
        Expr::If {
            cond,
            then_br,
            else_br,
            ..
        } => {
            contains_owner_field(cond)
                || contains_owner_field(then_br)
                || contains_owner_field(else_br)
        }
        Expr::Index { base, index, .. } => {
            contains_owner_field(base) || contains_owner_field(index)
        }
        Expr::Int(..) | Expr::Bool(..) | Expr::String(..) | Expr::Var(..) => false,
    }
}

fn block_contains_owner_field(block: &Block) -> bool {
    contains_owner_field(&Expr::Block {
        block: Box::new(block.clone()),
    })
}

fn is_address(ty: &Type, imported_address: bool) -> bool {
    matches!(ty, Type::Named { name, args } if args.is_empty() && (name == ADDRESS || imported_address && name == "Address"))
}

fn contains_address(ty: &Type, imported_address: bool) -> bool {
    match ty {
        Type::Named { args, .. } => {
            is_address(ty, imported_address)
                || args
                    .iter()
                    .any(|arg| contains_address(arg, imported_address))
        }
        Type::Option(inner)
        | Type::List(inner)
        | Type::Set(inner)
        | Type::Array(inner, _)
        | Type::Slice(inner) => contains_address(inner, imported_address),
        Type::Result(ok, error) | Type::Map(ok, error) => {
            contains_address(ok, imported_address) || contains_address(error, imported_address)
        }
        Type::Tuple(values) => values
            .iter()
            .any(|value| contains_address(value, imported_address)),
        Type::Fn { params, ret } => {
            params
                .iter()
                .any(|param| contains_address(param, imported_address))
                || contains_address(ret, imported_address)
        }
        Type::Int
        | Type::U8
        | Type::U64
        | Type::U128
        | Type::U256
        | Type::Bool
        | Type::String
        | Type::Bytes => false,
    }
}

fn function_span(function: &clg_ast::Func) -> Span {
    function.effect_span.unwrap_or(Span { start: 0, end: 0 })
}

#[cfg(test)]
mod tests {
    use super::*;
    use clg_parser::parse;

    const VALID: &str = r#"
        import std::eth::{Address}
        contract ConfigRegistry version 1 {
            state { owner: Address; value: U64; }
            event ValueChanged { value: U64; }
            init(owner: Address, value: U64) {
                state.owner = owner;
                state.value = value;
                0
            }
            mut function set_value(next: U64) -> U64
                require { std::eth::address::equals(std::eth::context::caller(), state.owner) }
            {
                state.value = next;
                emit ValueChanged(next);
                next
            }
        }
    "#;

    #[test]
    fn accepts_the_single_locked_owner_guard_shape() {
        let program = parse(VALID).expect("parse caller-aware contract");
        assert!(is_owner_guard(
            &program.contracts[0].functions[0].requires[0].expr
        ));
        validate_caller_authorization_profile(&program).expect("validate owner guard");
    }

    #[test]
    fn rejects_non_owner_address_placement_and_caller_escape() {
        for source in [
            VALID.replace("owner: Address", "administrator: Address"),
            VALID.replace(
                "event ValueChanged { value: U64; }",
                "event ValueChanged { account: Address; }",
            ),
            VALID.replace(
                "require { std::eth::address::equals(std::eth::context::caller(), state.owner) }",
                "require { std::eth::address::equals(state.owner, std::eth::context::caller()) }",
            ),
            VALID.replace("mut function set_value", "pure function set_value"),
            VALID.replace(
                "state.value = next;",
                "let caller = std::eth::context::caller();\n                state.value = next;",
            ),
        ] {
            let program = parse(&source).expect("parse forbidden caller shape");
            let error = validate_caller_authorization_profile(&program)
                .expect_err("reject forbidden caller shape");
            assert!(format!("{error:#}").contains("T834"));
        }
    }
}
