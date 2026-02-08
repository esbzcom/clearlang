use anyhow::Result;
use clg_ast::Type;

use super::AliasMap;

pub(super) fn mangle_fn_name(name: &str, args: &[Type], aliases: &AliasMap) -> Result<String> {
    if args.is_empty() {
        return Ok(name.to_string());
    }
    let mut parts = Vec::with_capacity(args.len());
    for arg in args {
        parts.push(mangle_type(arg, aliases)?);
    }
    Ok(format!("{}${}", name, parts.join("$")))
}

pub(super) fn mangle_impl_method_name(
    trait_name: &str,
    self_ty: &Type,
    method: &str,
    aliases: &AliasMap,
) -> Result<String> {
    let self_name = mangle_type(self_ty, aliases)?;
    Ok(format!("impl${}${}${}", trait_name, self_name, method))
}

pub(super) fn mangle_type(ty: &Type, aliases: &AliasMap) -> Result<String> {
    let _ = aliases;
    Ok(match ty {
        Type::Int => "Int".to_string(),
        Type::U8 => "U8".to_string(),
        Type::U64 => "U64".to_string(),
        Type::U128 => "U128".to_string(),
        Type::U256 => "U256".to_string(),
        Type::Bool => "Bool".to_string(),
        Type::String => "String".to_string(),
        Type::Bytes => "Bytes".to_string(),
        Type::Named { name, args } => {
            if args.is_empty() {
                name.clone()
            } else {
                let mut parts = Vec::with_capacity(args.len() + 2);
                parts.push(name.clone());
                parts.push(args.len().to_string());
                for arg in args {
                    parts.push(mangle_type(arg, aliases)?);
                }
                format!("N${}", parts.join("$"))
            }
        }
        Type::Option(inner) => format!("Option${}", mangle_type(inner, aliases)?),
        Type::Result(ok, err) => format!(
            "Result${}${}",
            mangle_type(ok, aliases)?,
            mangle_type(err, aliases)?
        ),
        Type::List(inner) => format!("List${}", mangle_type(inner, aliases)?),
        Type::Set(inner) => format!("Set${}", mangle_type(inner, aliases)?),
        Type::Map(k, v) => format!(
            "Map${}${}",
            mangle_type(k, aliases)?,
            mangle_type(v, aliases)?
        ),
        Type::Array(inner, _) => format!("Array${}", mangle_type(inner, aliases)?),
        Type::Slice(inner) => format!("Slice${}", mangle_type(inner, aliases)?),
        Type::Tuple(elements) => {
            let mut parts = Vec::with_capacity(elements.len() + 1);
            parts.push(elements.len().to_string());
            for elem in elements {
                parts.push(mangle_type(elem, aliases)?);
            }
            format!("Tuple${}", parts.join("$"))
        }
    })
}
