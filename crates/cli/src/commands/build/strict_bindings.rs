use crate::commands::modules::verified_std_abi_value_symbols;

#[derive(Clone, Debug, Eq, PartialEq)]
struct AbiLinkProfile {
    effect: String,
    params: Vec<String>,
    ret: String,
    capability: Option<String>,
}

#[derive(Clone, Debug)]
struct StrictExternalBindings {
    expected_profiles: std::collections::BTreeMap<String, AbiLinkProfile>,
    external_typer_sigs: Vec<ExternalBuiltinSig>,
    external_codegen_imports: Vec<ExternalImport>,
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
struct StrictGateViolation {
    code: &'static str,
    package: String,
    symbol: String,
    message: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct StrictImportMapArtifact {
    canonical_bytes: Vec<u8>,
    canonical_hash: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct StrictGateOutcome {
    linked_imports: Vec<(String, AbiLinkProfile)>,
    violations: Vec<StrictGateViolation>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct StrictResolvedImportBinding {
    profile: AbiLinkProfile,
    params_typed: Vec<Type>,
    ret_typed: Type,
}

fn strict_external_bindings_from_contract(
    package_contract: &StrictPackageContractV0,
) -> Result<StrictExternalBindings, String> {
    use std::collections::BTreeMap;

    let mut resolved: BTreeMap<String, (String, StrictResolvedImportBinding)> = BTreeMap::new();
    for contract in &package_contract.contracts {
        for import in &contract.imports {
            let profile = AbiLinkProfile {
                effect: import.effect.clone(),
                params: import
                    .params
                    .iter()
                    .map(|ty| normalize_type_contract(ty))
                    .collect(),
                ret: normalize_type_contract(&import.ret),
                capability: import.capability.clone(),
            };
            let params_typed: Vec<Type> = profile
                .params
                .iter()
                .map(|ty| parse_contract_type(ty))
                .collect::<Result<Vec<_>, _>>()
                .map_err(|msg| {
                    format!(
                        "strict ABI/link mismatch for symbol `{}`: invalid parameter type in strict package ABI: {}",
                        import.symbol, msg
                    )
                })?;
            let ret_typed = parse_contract_type(profile.ret.as_str()).map_err(|msg| {
                format!(
                    "strict ABI/link mismatch for symbol `{}`: invalid return type in strict package ABI: {}",
                    import.symbol, msg
                )
            })?;
            let binding = StrictResolvedImportBinding {
                profile,
                params_typed,
                ret_typed,
            };
            if let Some((existing_abi_id, existing_binding)) = resolved.get(&import.symbol) {
                if existing_binding.profile != binding.profile {
                    return Err(format!(
                        "strict ABI/link mismatch for symbol `{}`: conflicting ABI profiles between `{}` and `{}`",
                        import.symbol, existing_abi_id, contract.abi_id
                    ));
                }
                continue;
            }
            resolved.insert(import.symbol.clone(), (contract.abi_id.clone(), binding));
        }
    }

    let mut expected_profiles = BTreeMap::new();
    let mut external_typer_sigs = Vec::with_capacity(resolved.len());
    let mut external_codegen_imports = Vec::with_capacity(resolved.len());
    for (symbol, (_, binding)) in resolved {
        let effect = parse_contract_effect(binding.profile.effect.as_str())
            .map_err(|msg| format!("strict ABI/link mismatch for symbol `{}`: {}", symbol, msg))?;
        let params: Vec<Param> = binding
            .params_typed
            .iter()
            .enumerate()
            .map(|(idx, ty)| Param {
                kind: ParamKind::Borrow,
                name: format!("p{idx}"),
                ty: ty.clone(),
            })
            .collect();
        let (import_module, import_name) = split_symbol_import_target(symbol.as_str())?;

        expected_profiles.insert(symbol.clone(), binding.profile);
        external_typer_sigs.push(ExternalBuiltinSig {
            name: symbol.clone(),
            params,
            ret: binding.ret_typed,
            effect,
        });
        external_codegen_imports.push(ExternalImport {
            function: symbol,
            import_module,
            import_name,
        });
    }

    Ok(StrictExternalBindings {
        expected_profiles,
        external_typer_sigs,
        external_codegen_imports,
    })
}

fn split_symbol_import_target(symbol: &str) -> Result<(String, String), String> {
    if let Some((module, name)) = symbol.rsplit_once("::") {
        if module.trim().is_empty() || name.trim().is_empty() {
            return Err(format!(
                "strict ABI/link mismatch for symbol `{}`: symbol must use non-empty `module::name` segments",
                symbol
            ));
        }
        Ok((module.to_string(), name.to_string()))
    } else {
        Err(format!(
            "strict ABI/link mismatch for symbol `{}`: symbol must contain `::` and end with function name",
            symbol
        ))
    }
}

fn filter_precompiled_std_core_typer_overrides(
    external_typer_sigs: &[ExternalBuiltinSig],
) -> Vec<ExternalBuiltinSig> {
    external_typer_sigs
        .iter()
        .filter(|sig| !is_verified_std_abi_symbol(sig.name.as_str()))
        .cloned()
        .collect()
}

fn filter_precompiled_std_core_codegen_overrides(
    external_codegen_imports: &[ExternalImport],
) -> Vec<ExternalImport> {
    external_codegen_imports
        .iter()
        .filter(|import| {
            !is_verified_std_abi_symbol(import.function.as_str())
                || is_phase21_precompiled_std_core_locked_symbol(import.function.as_str())
        })
        .cloned()
        .collect()
}

fn is_phase21_precompiled_std_core_locked_symbol(symbol: &str) -> bool {
    matches!(symbol, "std::str::len" | "std::bytes::len")
}

fn is_verified_std_abi_symbol(symbol: &str) -> bool {
    verified_std_abi_value_symbols().contains(symbol)
}

fn precompiled_std_core_intrinsic_fallback_violations(
    std_core_link_mode: StdCoreLinkMode,
    ir: &IrModule,
    linked_import_resolution: &std::result::Result<Vec<(String, AbiLinkProfile)>, String>,
) -> Vec<StrictGateViolation> {
    if std_core_link_mode != StdCoreLinkMode::Precompiled {
        return Vec::new();
    }

    let locked_symbols: std::collections::BTreeSet<String> = ir
        .funcs
        .iter()
        .filter_map(|func| {
            let symbol = func.name.as_str();
            if is_phase21_precompiled_std_core_locked_symbol(symbol) {
                Some(symbol.to_string())
            } else {
                None
            }
        })
        .collect();
    if locked_symbols.is_empty() {
        return Vec::new();
    }

    let linked_symbols: std::collections::BTreeSet<String> = linked_import_resolution
        .as_ref()
        .map(|linked| linked.iter().map(|(symbol, _)| symbol.clone()).collect())
        .unwrap_or_default();

    locked_symbols
        .into_iter()
        .filter(|symbol| !linked_symbols.contains(symbol))
        .map(|symbol| StrictGateViolation {
            code: "C105",
            package: "std::core".to_string(),
            symbol: symbol.clone(),
            message: format!(
                "strict ABI/link mismatch for symbol `{symbol}`: precompiled std-core mode forbids intrinsic fallback; link this symbol via strict package ABI/import"
            ),
        })
        .collect()
}

fn parse_contract_effect(raw: &str) -> Result<Effect, String> {
    match raw {
        "pure" => Ok(Effect::Pure),
        "mut" => Ok(Effect::Mut),
        "io" => Ok(Effect::Io),
        "none" => Ok(Effect::None),
        other => Err(format!(
            "unsupported effect `{other}` in strict package ABI"
        )),
    }
}

fn parse_contract_type(raw: &str) -> Result<Type, String> {
    let value = raw.trim();
    if value.is_empty() {
        return Err("type is empty".to_string());
    }
    if value.contains('<')
        || value.contains('>')
        || value.contains('[')
        || value.contains(']')
        || value.contains('(')
        || value.contains(')')
        || value.contains(',')
        || value.contains(';')
    {
        return Err(format!(
            "type `{}` uses unsupported generic/compound syntax in strict package ABI v0",
            value
        ));
    }
    let ty = match value {
        "Int" => Type::Int,
        "Bool" => Type::Bool,
        "String" => Type::String,
        "Bytes" => Type::Bytes,
        "U8" => Type::U8,
        "U64" => Type::U64,
        "U128" => Type::U128,
        "U256" => Type::U256,
        other => {
            validate_named_type_path(other)?;
            Type::Named {
                name: other.to_string(),
                args: Vec::new(),
            }
        }
    };
    Ok(ty)
}

fn validate_named_type_path(value: &str) -> Result<(), String> {
    for segment in value.split("::") {
        if segment.is_empty() {
            return Err("contains empty `::` segment".to_string());
        }
        let mut chars = segment.chars();
        let first = chars.next().expect("segment non-empty");
        if !(first == '_' || first.is_ascii_alphabetic()) {
            return Err(format!(
                "segment `{segment}` must start with ASCII letter or `_`"
            ));
        }
        for ch in chars {
            if !(ch == '_' || ch.is_ascii_alphanumeric()) {
                return Err(format!(
                    "segment `{segment}` contains invalid character `{ch}`"
                ));
            }
        }
    }
    Ok(())
}

fn linked_external_import_profiles_from_ir(
    ir: &clg_ir::Module,
    external_imports: &[ExternalBuiltinSig],
) -> Result<Vec<(String, AbiLinkProfile)>, String> {
    use std::collections::BTreeSet;

    let resolved_profiles = resolved_external_import_profiles(external_imports)?;

    let mut linked_symbol_names = BTreeSet::new();
    for func in &ir.funcs {
        if resolved_profiles.contains_key(func.name.as_str()) {
            linked_symbol_names.insert(func.name.as_str());
        }
    }

    let mut linked = Vec::with_capacity(linked_symbol_names.len());
    for symbol in linked_symbol_names {
        let profile = resolved_profiles
            .get(symbol)
            .expect("linked symbol must exist in resolved map");
        linked.push((symbol.to_string(), profile.clone()));
    }
    Ok(linked)
}

fn resolved_external_import_profiles(
    external_imports: &[ExternalBuiltinSig],
) -> Result<std::collections::BTreeMap<String, AbiLinkProfile>, String> {
    use std::collections::BTreeMap;

    let mut resolved_profiles: BTreeMap<String, AbiLinkProfile> = BTreeMap::new();
    for import in external_imports {
        let profile = AbiLinkProfile {
            effect: effect_to_canonical(import.effect).to_string(),
            params: import
                .params
                .iter()
                .map(|param| type_to_canonical(&param.ty))
                .collect(),
            ret: type_to_canonical(&import.ret),
            capability: None,
        };
        if let Some(existing) = resolved_profiles.get(import.name.as_str()) {
            if !abi_signatures_match(existing, &profile) {
                return Err(format!(
                    "strict ABI/link mismatch for symbol `{}`: conflicting resolved external import signatures",
                    import.name
                ));
            }
            continue;
        }
        resolved_profiles.insert(import.name.clone(), profile);
    }
    Ok(resolved_profiles)
}

#[cfg(test)]
mod tests {
    use super::is_verified_std_abi_symbol;

    #[test]
    fn verified_std_abi_filter_matches_phase27_manifest_split() {
        assert!(
            is_verified_std_abi_symbol("std::host::env::chain_id"),
            "verified std abi symbols should remain compiler-owned"
        );
        assert!(
            !is_verified_std_abi_symbol("std::contract::address::from_bytes"),
            "external std package candidates should not be treated as compiler-owned"
        );
    }
}
