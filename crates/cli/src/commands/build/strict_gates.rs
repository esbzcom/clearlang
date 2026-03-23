fn evaluate_strict_gates(
    expected_profiles: &std::collections::BTreeMap<String, AbiLinkProfile>,
    linked_imports: &[(String, AbiLinkProfile)],
    host_profile: &StrictHostProfileV0,
) -> Vec<StrictGateViolation> {
    use std::collections::{HashMap, HashSet};

    let symbol_package_index: HashMap<&str, String> = expected_profiles
        .keys()
        .map(|symbol| (symbol.as_str(), package_id_from_symbol(symbol)))
        .collect();

    let host_caps: HashSet<&str> = host_profile
        .capabilities
        .iter()
        .map(String::as_str)
        .collect();
    let mut diagnostics = Vec::new();
    for (symbol, linked_profile) in linked_imports {
        let Some(expected) = expected_profiles.get(symbol) else {
            diagnostics.push(StrictGateViolation {
                code: "C105",
                package: package_id_from_symbol(symbol),
                symbol: symbol.clone(),
                message: format!(
                    "strict ABI/link mismatch: linked import `{}` is not declared in strict package ABI",
                    symbol
                ),
            });
            continue;
        };

        if !abi_signatures_match(expected, linked_profile) {
            diagnostics.push(StrictGateViolation {
                code: "C105",
                package: symbol_package_index
                    .get(symbol.as_str())
                    .cloned()
                    .unwrap_or_else(|| package_id_from_symbol(symbol)),
                symbol: symbol.clone(),
                message: format!(
                    "strict ABI/link mismatch for symbol `{}`: expected effect/signature `{}({}) -> {}` but resolved `{}({}) -> {}`",
                    symbol,
                    expected.effect,
                    expected.params.join(", "),
                    expected.ret,
                    linked_profile.effect,
                    linked_profile.params.join(", "),
                    linked_profile.ret
                ),
            });
            continue;
        }

        if let Some(capability) = expected.capability.as_deref() {
            if !strict_capability_allowed_for_profile(host_profile.profile.as_str(), capability) {
                diagnostics.push(StrictGateViolation {
                    code: "C106",
                    package: symbol_package_index
                        .get(symbol.as_str())
                        .cloned()
                        .unwrap_or_else(|| package_id_from_symbol(symbol)),
                    symbol: symbol.clone(),
                    message: format!(
                        "strict runtime capability mismatch: symbol `{}` requires capability `{}` denied by strict deterministic policy for profile `{}`",
                        symbol, capability, host_profile.profile
                    ),
                });
                continue;
            }
            if !host_caps.contains(capability) {
                diagnostics.push(StrictGateViolation {
                    code: "C106",
                    package: symbol_package_index
                        .get(symbol.as_str())
                        .cloned()
                        .unwrap_or_else(|| package_id_from_symbol(symbol)),
                    symbol: symbol.clone(),
                    message: format!(
                        "strict runtime capability mismatch: symbol `{}` requires capability `{}` not present in host profile `{}`",
                        symbol, capability, host_profile.profile
                    ),
                });
            }
        }
    }

    sort_strict_gate_violations(diagnostics.as_mut_slice());
    diagnostics
}

fn strict_capability_allowed_for_profile(profile: &str, capability: &str) -> bool {
    match (profile, capability) {
        // Phase 21 lock: strict mode denies nondeterministic env capabilities.
        ("contract_static", "std::env::time")
        | ("contract_static", "std::env::random")
        | ("shared_app", "std::env::time")
        | ("shared_app", "std::env::random") => false,
        _ => true,
    }
}

fn sort_strict_gate_violations(violations: &mut [StrictGateViolation]) {
    violations.sort_by(|lhs, rhs| {
        lhs.code
            .cmp(rhs.code)
            .then(lhs.package.cmp(&rhs.package))
            .then(lhs.symbol.cmp(&rhs.symbol))
            .then(lhs.message.cmp(&rhs.message))
    });
}

fn strict_gate_outcome_from_linked_import_resolution(
    expected_profiles: &std::collections::BTreeMap<String, AbiLinkProfile>,
    host_profile: &StrictHostProfileV0,
    linked_import_resolution: std::result::Result<Vec<(String, AbiLinkProfile)>, String>,
) -> StrictGateOutcome {
    let (linked_imports, mut violations) = match linked_import_resolution {
        Ok(linked_imports) => (linked_imports, Vec::new()),
        Err(message) => (
            Vec::new(),
            vec![StrictGateViolation {
                code: "C105",
                package: "_".to_string(),
                symbol: "_".to_string(),
                message,
            }],
        ),
    };
    violations.extend(evaluate_strict_gates(
        expected_profiles,
        linked_imports.as_slice(),
        host_profile,
    ));
    sort_strict_gate_violations(violations.as_mut_slice());
    StrictGateOutcome {
        linked_imports,
        violations,
    }
}

fn package_id_from_symbol(symbol: &str) -> String {
    let mut segments = symbol.split("::");
    let first = segments.next().unwrap_or_default();
    let second = segments.next().unwrap_or_default();
    if first.is_empty() {
        "_".to_string()
    } else if second.is_empty() {
        first.to_string()
    } else {
        format!("{first}::{second}")
    }
}

#[cfg(test)]
fn strict_abi_link_violation(
    package_contract: &StrictPackageContractV0,
    linked_imports: &[ExternalBuiltinSig],
) -> Option<String> {
    let bindings = match strict_external_bindings_from_contract(package_contract) {
        Ok(value) => value,
        Err(message) => return Some(message),
    };
    let linked = match resolved_external_import_profiles(linked_imports) {
        Ok(value) => value.into_iter().collect::<Vec<_>>(),
        Err(message) => return Some(message),
    };
    let diagnostics = evaluate_strict_gates(
        &bindings.expected_profiles,
        linked.as_slice(),
        &StrictHostProfileV0 {
            profile: "contract_static".to_string(),
            capabilities: Vec::new(),
        },
    );
    diagnostics
        .into_iter()
        .find(|diag| diag.code == "C105")
        .map(|diag| diag.message)
}

#[cfg(test)]
fn strict_runtime_capability_violation(
    package_contract: &StrictPackageContractV0,
    host_profile: &StrictHostProfileV0,
    linked_imports: &[ExternalBuiltinSig],
) -> Option<String> {
    let bindings = match strict_external_bindings_from_contract(package_contract) {
        Ok(value) => value,
        Err(_) => return None,
    };
    let linked = match resolved_external_import_profiles(linked_imports) {
        Ok(value) => value.into_iter().collect::<Vec<_>>(),
        Err(_) => return None,
    };
    let diagnostics =
        evaluate_strict_gates(&bindings.expected_profiles, linked.as_slice(), host_profile);
    diagnostics
        .into_iter()
        .find(|diag| diag.code == "C106")
        .map(|diag| diag.message)
}

fn abi_signatures_match(expected: &AbiLinkProfile, actual: &AbiLinkProfile) -> bool {
    expected.effect == actual.effect
        && expected.params == actual.params
        && expected.ret == actual.ret
}

fn effect_to_canonical(effect: Effect) -> &'static str {
    match effect {
        Effect::Pure => "pure",
        Effect::Mut => "mut",
        Effect::Io => "io",
        Effect::None => "none",
    }
}

fn type_to_canonical(ty: &Type) -> String {
    match ty {
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
                let args = args
                    .iter()
                    .map(type_to_canonical)
                    .collect::<Vec<_>>()
                    .join(",");
                format!("{name}<{args}>")
            }
        }
        Type::Option(inner) => format!("Option<{}>", type_to_canonical(inner)),
        Type::Result(ok, err) => format!(
            "Result<{},{}>",
            type_to_canonical(ok),
            type_to_canonical(err)
        ),
        Type::List(inner) => format!("List<{}>", type_to_canonical(inner)),
        Type::Set(inner) => format!("Set<{}>", type_to_canonical(inner)),
        Type::Map(key, value) => format!(
            "Map<{},{}>",
            type_to_canonical(key),
            type_to_canonical(value)
        ),
        Type::Array(inner, len) => match len {
            Some(len) => format!("[{}; {}]", type_to_canonical(inner), len),
            None => format!("[{}]", type_to_canonical(inner)),
        },
        Type::Slice(inner) => format!("Slice<{}>", type_to_canonical(inner)),
        Type::Tuple(items) => format!(
            "({})",
            items
                .iter()
                .map(type_to_canonical)
                .collect::<Vec<_>>()
                .join(",")
        ),
        Type::Fn { params, ret } => format!(
            "Fn({})->{}",
            params
                .iter()
                .map(type_to_canonical)
                .collect::<Vec<_>>()
                .join(","),
            type_to_canonical(ret)
        ),
    }
}

fn normalize_type_contract(raw: &str) -> String {
    raw.chars()
        .filter(|ch| !ch.is_ascii_whitespace())
        .collect::<String>()
}

