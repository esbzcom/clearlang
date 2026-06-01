fn check_std_surface_drift(root: &Path, raw_args: Vec<String>) -> Result<(), String> {
    let opts = parse_std_surface_args(raw_args)?;
    let design_lock_path = root
        .join("docs")
        .join("design")
        .join("phase-21.0-std-core-package-surface.md");
    let std_metadata_path = root
        .join("crates")
        .join("cli")
        .join("assets")
        .join("std-metadata.json");
    let typer_builtins_path = root
        .join("crates")
        .join("typer")
        .join("src")
        .join("builtins.rs");
    let typer_collections_path = root
        .join("crates")
        .join("typer")
        .join("src")
        .join("check")
        .join("expr")
        .join("calls")
        .join("collections.rs");
    let codegen_ir_path = root
        .join("crates")
        .join("codegen-wasm")
        .join("src")
        .join("ir")
        .join("mod.rs");
    let binding_lock_path = root
        .join("docs")
        .join("design")
        .join("phase-21.0-std-binding-map.lock.json");

    let design_lock = fs::read_to_string(&design_lock_path)
        .map_err(|e| format!("read `{}`: {e}", design_lock_path.display()))?;
    let locked_symbols = parse_phase21_locked_symbols_from_design(&design_lock)?;
    let metadata_symbols = load_std_metadata_value_symbols(&std_metadata_path)?;

    let typer_builtins_source = fs::read_to_string(&typer_builtins_path)
        .map_err(|e| format!("read `{}`: {e}", typer_builtins_path.display()))?;
    let typer_collections_source = fs::read_to_string(&typer_collections_path)
        .map_err(|e| format!("read `{}`: {e}", typer_collections_path.display()))?;
    let typer_builtin_symbols = extract_std_symbols_from_source(&typer_builtins_source)?;
    let typer_collection_symbols = extract_std_symbols_from_source(&typer_collections_source)?;
    let typer_symbols = typer_builtin_symbols
        .union(&typer_collection_symbols)
        .filter(|symbol| is_callable_std_symbol(symbol))
        .cloned()
        .collect::<BTreeSet<_>>();

    let codegen_ir_source = fs::read_to_string(&codegen_ir_path)
        .map_err(|e| format!("read `{}`: {e}", codegen_ir_path.display()))?;
    let codegen_symbols_all = extract_std_symbols_from_source(&codegen_ir_source)?;
    let codegen_symbols = codegen_symbols_all
        .intersection(&locked_symbols)
        .cloned()
        .collect::<BTreeSet<_>>();

    let generated_binding_map = generate_std_binding_map(&locked_symbols, &codegen_symbols);

    if let Some(out_dir) = opts.emit_artifact.as_ref() {
        emit_std_binding_map_artifact(out_dir.as_path(), &generated_binding_map)?;
    }

    if opts.refresh_lock {
        let bytes = pretty_json_bytes(&generated_binding_map)?;
        fs::write(&binding_lock_path, bytes)
            .map_err(|e| format!("write `{}`: {e}", binding_lock_path.display()))?;
    }

    let mut drift_errors = Vec::new();
    collect_set_mismatch(
        "phase-21 design lock vs std metadata value surface",
        &locked_symbols,
        &metadata_symbols,
        &mut drift_errors,
    );
    collect_set_mismatch(
        "phase-21 design lock vs typer std call-check surface",
        &locked_symbols,
        &typer_symbols,
        &mut drift_errors,
    );
    collect_extras(
        "codegen intrinsic symbol surface",
        &locked_symbols,
        &codegen_symbols_all,
        &mut drift_errors,
    );

    let expected_binding_lock = fs::read_to_string(&binding_lock_path)
        .map_err(|e| format!("read `{}`: {e}", binding_lock_path.display()))?;
    let parsed_lock: StdBindingMapLockFile = serde_json::from_str(&expected_binding_lock)
        .map_err(|e| format!("parse `{}`: {e}", binding_lock_path.display()))?;
    let normalized_lock = normalize_binding_map_lock(parsed_lock)?;
    let lock_symbols_from_binding = normalized_lock
        .symbols
        .iter()
        .map(|entry| entry.symbol.clone())
        .collect::<BTreeSet<_>>();
    collect_set_mismatch(
        "phase-21 design lock vs std binding-map lock symbols",
        &locked_symbols,
        &lock_symbols_from_binding,
        &mut drift_errors,
    );

    let expected_routes = binding_routes_as_map(&normalized_lock);
    let generated_routes = binding_routes_as_map(&generated_binding_map);
    collect_route_mismatch(&expected_routes, &generated_routes, &mut drift_errors);

    if drift_errors.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "std surface drift detected:\n - {}",
            drift_errors.join("\n - ")
        ))
    }
}

fn emit_host_capability_policy_artifact(root: &Path, raw_args: Vec<String>) -> Result<(), String> {
    let opts = parse_host_capability_policy_args(raw_args)?;
    let lock_path = root
        .join("docs")
        .join("design")
        .join("phase-21.0-host-capability-policy.lock.json");
    let policy = canonical_host_capability_policy_v1();

    if opts.refresh_lock {
        let bytes = pretty_json_bytes(&policy)?;
        fs::write(&lock_path, bytes)
            .map_err(|e| format!("write `{}`: {e}", lock_path.display()))?;
    }

    let lock_raw = fs::read_to_string(&lock_path)
        .map_err(|e| format!("read `{}`: {e}", lock_path.display()))?;
    let lock: HostCapabilityPolicyFile = serde_json::from_str(&lock_raw)
        .map_err(|e| format!("parse `{}`: {e}", lock_path.display()))?;
    let normalized_lock = normalize_host_capability_policy(lock)?;
    let normalized_policy = normalize_host_capability_policy(policy)?;
    if normalized_lock != normalized_policy {
        return Err(format!(
            "host capability policy drift detected: `{}` does not match canonical Phase 21 policy",
            lock_path.display()
        ));
    }

    if let Some(out_dir) = opts.emit_artifact.as_ref() {
        fs::create_dir_all(out_dir).map_err(|e| format!("create `{}`: {e}", out_dir.display()))?;
        let json_path = out_dir.join("host-capability-policy-v1.json");
        let sha_path = out_dir.join("host-capability-policy-v1.sha256");
        let bytes = pretty_json_bytes(&normalized_policy)?;
        fs::write(&json_path, &bytes)
            .map_err(|e| format!("write `{}`: {e}", json_path.display()))?;
        let digest = format!("sha256:{}", hex::encode(Sha256::digest(&bytes)));
        fs::write(&sha_path, format!("{digest}\n"))
            .map_err(|e| format!("write `{}`: {e}", sha_path.display()))?;
    }

    Ok(())
}

fn parse_host_capability_policy_args(raw_args: Vec<String>) -> Result<HostPolicyOpts, String> {
    let mut emit_artifact: Option<PathBuf> = None;
    let mut refresh_lock = false;
    let mut idx = 0usize;
    while idx < raw_args.len() {
        match raw_args[idx].as_str() {
            "--emit-artifact" => {
                idx += 1;
                let value = raw_args
                    .get(idx)
                    .ok_or_else(|| "missing value for `--emit-artifact`".to_string())?;
                emit_artifact = Some(PathBuf::from(value));
            }
            "--refresh-lock" => {
                refresh_lock = true;
            }
            other => {
                return Err(format!(
                    "unknown host-capability-policy-artifact arg `{other}` (supported: --emit-artifact, --refresh-lock)"
                ));
            }
        }
        idx += 1;
    }
    Ok(HostPolicyOpts {
        emit_artifact,
        refresh_lock,
    })
}

fn canonical_host_capability_policy_v1() -> HostCapabilityPolicyFile {
    let base_caps = [
        "std::crypto::hash",
        "std::crypto::hmac",
        "std::crypto::verify",
        "std::env::chain_id",
        "std::env::random",
        "std::env::time",
        "std::wasi::print",
    ];
    let mut profiles = Vec::new();
    for profile in ["contract_static", "shared_app"] {
        let mut capabilities = base_caps
            .iter()
            .map(|capability| HostCapabilityRule {
                capability: (*capability).to_string(),
                strict_mode: if *capability == "std::env::time" || *capability == "std::env::random"
                {
                    "deny".to_string()
                } else {
                    "allow".to_string()
                },
                reason: if *capability == "std::env::time" || *capability == "std::env::random" {
                    "phase21_strict_determinism".to_string()
                } else {
                    "allowed_v0_surface".to_string()
                },
            })
            .collect::<Vec<_>>();
        capabilities.sort_by(|lhs, rhs| lhs.capability.cmp(&rhs.capability));
        profiles.push(HostCapabilityProfile {
            profile: profile.to_string(),
            capabilities,
        });
    }
    HostCapabilityPolicyFile {
        schema_version: 1,
        profiles,
    }
}

fn normalize_host_capability_policy(
    mut policy: HostCapabilityPolicyFile,
) -> Result<HostCapabilityPolicyFile, String> {
    if policy.schema_version != 1 {
        return Err(format!(
            "unsupported host capability policy schema_version {} (expected 1)",
            policy.schema_version
        ));
    }
    policy
        .profiles
        .sort_by(|lhs, rhs| lhs.profile.cmp(&rhs.profile));
    let mut seen_profiles = BTreeSet::new();
    for profile in &mut policy.profiles {
        if !seen_profiles.insert(profile.profile.clone()) {
            return Err(format!(
                "duplicate host capability policy profile `{}`",
                profile.profile
            ));
        }
        profile
            .capabilities
            .sort_by(|lhs, rhs| lhs.capability.cmp(&rhs.capability));
        let mut seen_caps = BTreeSet::new();
        for cap in &profile.capabilities {
            if cap.strict_mode != "allow" && cap.strict_mode != "deny" {
                return Err(format!(
                    "invalid strict_mode `{}` for capability `{}` in profile `{}`",
                    cap.strict_mode, cap.capability, profile.profile
                ));
            }
            if !seen_caps.insert(cap.capability.clone()) {
                return Err(format!(
                    "duplicate capability `{}` in profile `{}`",
                    cap.capability, profile.profile
                ));
            }
        }
    }
    Ok(policy)
}

fn parse_std_surface_args(raw_args: Vec<String>) -> Result<StdSurfaceDriftOpts, String> {
    let mut emit_artifact: Option<PathBuf> = None;
    let mut refresh_lock = false;
    let mut idx = 0usize;
    while idx < raw_args.len() {
        match raw_args[idx].as_str() {
            "--emit-artifact" => {
                idx += 1;
                let value = raw_args
                    .get(idx)
                    .ok_or_else(|| "missing value for `--emit-artifact`".to_string())?;
                emit_artifact = Some(PathBuf::from(value));
            }
            "--refresh-lock" => {
                refresh_lock = true;
            }
            other => {
                return Err(format!(
                    "unknown std-surface-drift-check arg `{other}` (supported: --emit-artifact, --refresh-lock)"
                ));
            }
        }
        idx += 1;
    }
    Ok(StdSurfaceDriftOpts {
        emit_artifact,
        refresh_lock,
    })
}

fn parse_phase21_locked_symbols_from_design(markdown: &str) -> Result<BTreeSet<String>, String> {
    let pattern = Regex::new(r"std::([a-z0-9_]+)::\{([^}]*)\}")
        .map_err(|e| format!("compile phase-21 lock regex: {e}"))?;
    let mut symbols = BTreeSet::new();
    for capture in pattern.captures_iter(markdown) {
        let module = capture
            .get(1)
            .map(|m| m.as_str())
            .ok_or_else(|| "phase-21 lock parse missing module capture".to_string())?;
        let entries = capture
            .get(2)
            .map(|m| m.as_str())
            .ok_or_else(|| "phase-21 lock parse missing export capture".to_string())?;
        for item in entries.split(',') {
            let name = item.trim();
            if name.is_empty() {
                continue;
            }
            symbols.insert(format!("std::{module}::{name}"));
        }
    }
    if symbols.is_empty() {
        Err("phase-21 design lock yielded zero std symbols".to_string())
    } else {
        Ok(symbols)
    }
}

fn load_std_metadata_value_symbols(path: &Path) -> Result<BTreeSet<String>, String> {
    let raw = fs::read_to_string(path).map_err(|e| format!("read `{}`: {e}", path.display()))?;
    let parsed: StdMetadataRoot =
        serde_json::from_str(&raw).map_err(|e| format!("parse `{}`: {e}", path.display()))?;
    let mut out = BTreeSet::new();
    for module in parsed.modules {
        for export in module.exports {
            if export.kind == "value" {
                out.insert(format!("{}::{}", module.path, export.name));
            }
        }
    }
    Ok(out)
}

fn extract_std_symbols_from_source(source: &str) -> Result<BTreeSet<String>, String> {
    let pattern = Regex::new(r#""(std::[A-Za-z0-9_]+(?:::[A-Za-z0-9_]+)+)""#)
        .map_err(|e| format!("compile std symbol regex: {e}"))?;
    let mut out = BTreeSet::new();
    for capture in pattern.captures_iter(source) {
        if let Some(symbol) = capture.get(1) {
            out.insert(symbol.as_str().to_string());
        }
    }
    Ok(out)
}

fn is_callable_std_symbol(symbol: &str) -> bool {
    let Some((_, leaf)) = symbol.rsplit_once("::") else {
        return false;
    };
    leaf.chars()
        .next()
        .map(|c| c.is_ascii_lowercase())
        .unwrap_or(false)
}

fn generate_std_binding_map(
    locked_symbols: &BTreeSet<String>,
    codegen_intrinsic_symbols: &BTreeSet<String>,
) -> StdBindingMapLockFile {
    let symbols = locked_symbols
        .iter()
        .map(|symbol| StdBindingRouteEntry {
            symbol: symbol.clone(),
            route: if codegen_intrinsic_symbols.contains(symbol) {
                "intrinsic".to_string()
            } else {
                "package_import".to_string()
            },
        })
        .collect();
    StdBindingMapLockFile {
        schema_version: 1,
        symbols,
    }
}

fn normalize_binding_map_lock(
    lock: StdBindingMapLockFile,
) -> Result<StdBindingMapLockFile, String> {
    if lock.schema_version != 1 {
        return Err(format!(
            "unsupported std binding-map lock schema_version {} (expected 1)",
            lock.schema_version
        ));
    }
    let mut normalized = BTreeMap::<String, String>::new();
    for entry in lock.symbols {
        if entry.route != "intrinsic" && entry.route != "package_import" {
            return Err(format!(
                "invalid std binding-map route `{}` for symbol `{}`",
                entry.route, entry.symbol
            ));
        }
        if normalized
            .insert(entry.symbol.clone(), entry.route.clone())
            .is_some()
        {
            return Err(format!(
                "duplicate std binding-map lock symbol `{}`",
                entry.symbol
            ));
        }
    }
    Ok(StdBindingMapLockFile {
        schema_version: 1,
        symbols: normalized
            .into_iter()
            .map(|(symbol, route)| StdBindingRouteEntry { symbol, route })
            .collect(),
    })
}

fn collect_set_mismatch(
    label: &str,
    expected: &BTreeSet<String>,
    actual: &BTreeSet<String>,
    errors: &mut Vec<String>,
) {
    let missing = expected
        .difference(actual)
        .cloned()
        .collect::<Vec<String>>();
    let extra = actual
        .difference(expected)
        .cloned()
        .collect::<Vec<String>>();
    if !missing.is_empty() || !extra.is_empty() {
        errors.push(format!(
            "{label}: missing [{}], extra [{}]",
            preview_symbols(&missing),
            preview_symbols(&extra)
        ));
    }
}

fn collect_extras(
    label: &str,
    expected_superset: &BTreeSet<String>,
    actual: &BTreeSet<String>,
    errors: &mut Vec<String>,
) {
    let extra = actual
        .difference(expected_superset)
        .cloned()
        .collect::<Vec<String>>();
    if !extra.is_empty() {
        errors.push(format!("{label}: unexpected [{}]", preview_symbols(&extra)));
    }
}

fn collect_route_mismatch(
    expected: &BTreeMap<String, String>,
    actual: &BTreeMap<String, String>,
    errors: &mut Vec<String>,
) {
    let mut mismatches = Vec::new();
    for symbol in expected.keys() {
        if let (Some(exp), Some(found)) = (expected.get(symbol), actual.get(symbol)) {
            if exp != found {
                mismatches.push(format!("{symbol}: expected `{exp}`, found `{found}`"));
            }
        }
    }
    if !mismatches.is_empty() {
        errors.push(format!(
            "std binding-map route drift [{}]",
            preview_symbols(&mismatches)
        ));
    }
}

fn preview_symbols(items: &[String]) -> String {
    if items.is_empty() {
        return "none".to_string();
    }
    const MAX_ITEMS: usize = 8;
    let mut shown = items
        .iter()
        .take(MAX_ITEMS)
        .cloned()
        .collect::<Vec<String>>();
    if items.len() > MAX_ITEMS {
        shown.push(format!("... +{}", items.len() - MAX_ITEMS));
    }
    shown.join(", ")
}

fn binding_routes_as_map(lock: &StdBindingMapLockFile) -> BTreeMap<String, String> {
    lock.symbols
        .iter()
        .map(|entry| (entry.symbol.clone(), entry.route.clone()))
        .collect()
}

