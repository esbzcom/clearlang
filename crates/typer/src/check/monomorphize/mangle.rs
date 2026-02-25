use anyhow::Result;
use clg_ast::Type;

use super::AliasMap;

#[derive(Clone, Copy, Debug, Default)]
pub(super) struct MangleConfig {
    pub shorten_max_len: Option<usize>,
}

impl MangleConfig {
    pub(super) fn from_env() -> Self {
        let shorten_max_len = std::env::var("CLG_MANGLE_MAX_LEN")
            .ok()
            .and_then(|raw| raw.trim().parse::<usize>().ok())
            .filter(|len| *len > 0);
        Self { shorten_max_len }
    }
}

pub(super) fn mangle_fn_name(name: &str, args: &[Type], aliases: &AliasMap) -> Result<String> {
    mangle_fn_name_with_config(name, args, aliases, MangleConfig::from_env())
}

pub(super) fn mangle_fn_name_with_config(
    name: &str,
    args: &[Type],
    aliases: &AliasMap,
    config: MangleConfig,
) -> Result<String> {
    if args.is_empty() {
        return Ok(name.to_string());
    }
    let mut parts = Vec::with_capacity(args.len());
    for arg in args {
        parts.push(mangle_type(arg, aliases)?);
    }
    Ok(shorten_if_needed(
        format!("{}${}", name, parts.join("$")),
        config,
    ))
}

pub(super) fn mangle_impl_method_name(
    trait_name: &str,
    self_ty: &Type,
    method: &str,
    aliases: &AliasMap,
) -> Result<String> {
    mangle_impl_method_name_with_config(
        trait_name,
        self_ty,
        method,
        aliases,
        MangleConfig::from_env(),
    )
}

pub(super) fn mangle_impl_method_name_with_config(
    trait_name: &str,
    self_ty: &Type,
    method: &str,
    aliases: &AliasMap,
    config: MangleConfig,
) -> Result<String> {
    let self_name = mangle_type(self_ty, aliases)?;
    Ok(shorten_if_needed(
        format!("impl${}${}${}", trait_name, self_name, method),
        config,
    ))
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
        Type::Fn { params, ret } => {
            let mut parts = Vec::with_capacity(params.len() + 2);
            parts.push(params.len().to_string());
            for param in params {
                parts.push(mangle_type(param, aliases)?);
            }
            parts.push(mangle_type(ret, aliases)?);
            format!("Fn${}", parts.join("$"))
        }
    })
}

fn shorten_if_needed(name: String, config: MangleConfig) -> String {
    let Some(max_len) = config.shorten_max_len else {
        return name;
    };
    if name.len() <= max_len {
        return name;
    }
    let suffix = format!("$h{:016x}", fnv1a64(name.as_bytes()));
    if max_len <= suffix.len() {
        return suffix.chars().take(max_len).collect();
    }
    let prefix = prefix_with_max_bytes(&name, max_len - suffix.len());
    format!("{}{}", prefix, suffix)
}

fn prefix_with_max_bytes(s: &str, max_bytes: usize) -> &str {
    if s.len() <= max_bytes {
        return s;
    }
    let mut end = 0usize;
    for (idx, ch) in s.char_indices() {
        let next = idx + ch.len_utf8();
        if next > max_bytes {
            break;
        }
        end = next;
    }
    &s[..end]
}

fn fnv1a64(bytes: &[u8]) -> u64 {
    let mut hash: u64 = 0xcbf29ce484222325;
    for b in bytes {
        hash ^= u64::from(*b);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}
