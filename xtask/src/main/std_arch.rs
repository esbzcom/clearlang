fn run_std_arch_sync(root: &Path, raw_args: Vec<String>) -> Result<(), String> {
    let opts = parse_std_arch_sync_args(raw_args)?;
    let paths = std_arch_paths(root);

    let builtin_entries = collect_typed_signature_entries();
    let metadata = load_std_metadata_root(&paths.std_metadata_path)?;
    let codegen_ir_source = fs::read_to_string(&paths.codegen_ir_path)
        .map_err(|e| format!("read `{}`: {e}", paths.codegen_ir_path.display()))?;
    let codegen_symbols = extract_std_symbols_from_source(&codegen_ir_source)?;
    let host_capabilities = canonical_host_capability_set();

    if opts.refresh_lock {
        let generated = generate_catalog_from_runtime_sources(
            &builtin_entries,
            &metadata,
            &codegen_symbols,
            &host_capabilities,
        )?;
        let bytes = pretty_json_bytes(&generated)?;
        fs::write(&paths.catalog_lock_path, bytes)
            .map_err(|e| format!("write `{}`: {e}", paths.catalog_lock_path.display()))?;
    }

    let catalog_raw = fs::read_to_string(&paths.catalog_lock_path)
        .map_err(|e| format!("read `{}`: {e}", paths.catalog_lock_path.display()))?;
    let catalog: StdCatalogLockFile = serde_json::from_str(&catalog_raw)
        .map_err(|e| format!("parse `{}`: {e}", paths.catalog_lock_path.display()))?;
    let catalog = normalize_std_catalog_lock(catalog)?;

    let generated_metadata = std_metadata_from_catalog(&catalog);
    let generated_signatures = std_builtin_signatures_from_catalog(&catalog)?;

    if opts.write {
        fs::write(
            &paths.std_metadata_path,
            pretty_json_bytes(&generated_metadata)?,
        )
        .map_err(|e| format!("write `{}`: {e}", paths.std_metadata_path.display()))?;
        fs::write(
            &paths.std_signature_artifact_path,
            pretty_json_bytes(&generated_signatures)?,
        )
        .map_err(|e| format!("write `{}`: {e}", paths.std_signature_artifact_path.display()))?;
    }

    if let Some(out_dir) = opts.emit_artifact.as_ref() {
        fs::create_dir_all(out_dir).map_err(|e| format!("create `{}`: {e}", out_dir.display()))?;
        let metadata_path = out_dir.join("std-metadata-v2.json");
        let signatures_path = out_dir.join("std-builtin-signatures-v1.json");
        let sha_path = out_dir.join("std-arch-artifacts.sha256");
        let metadata_bytes = pretty_json_bytes(&generated_metadata)?;
        let signatures_bytes = pretty_json_bytes(&generated_signatures)?;
        fs::write(&metadata_path, &metadata_bytes)
            .map_err(|e| format!("write `{}`: {e}", metadata_path.display()))?;
        fs::write(&signatures_path, &signatures_bytes)
            .map_err(|e| format!("write `{}`: {e}", signatures_path.display()))?;
        let metadata_sha = format!("sha256:{}", hex::encode(Sha256::digest(&metadata_bytes)));
        let signatures_sha = format!("sha256:{}", hex::encode(Sha256::digest(&signatures_bytes)));
        let digest_lines = format!(
            "{}  {}\n{}  {}\n",
            metadata_sha,
            metadata_path.file_name().unwrap_or_default().to_string_lossy(),
            signatures_sha,
            signatures_path.file_name().unwrap_or_default().to_string_lossy()
        );
        fs::write(&sha_path, digest_lines)
            .map_err(|e| format!("write `{}`: {e}", sha_path.display()))?;
    }

    Ok(())
}

fn run_std_arch_conformance_check(root: &Path, _raw_args: Vec<String>) -> Result<(), String> {
    let paths = std_arch_paths(root);
    let catalog_raw = fs::read_to_string(&paths.catalog_lock_path)
        .map_err(|e| format!("read `{}`: {e}", paths.catalog_lock_path.display()))?;
    let catalog: StdCatalogLockFile = serde_json::from_str(&catalog_raw)
        .map_err(|e| format!("parse `{}`: {e}", paths.catalog_lock_path.display()))?;
    let catalog = normalize_std_catalog_lock(catalog)?;

    let expected_metadata = normalize_std_metadata_root(std_metadata_from_catalog(&catalog))?;
    let actual_metadata = load_std_metadata_root(&paths.std_metadata_path)?;
    let actual_metadata = normalize_std_metadata_root(actual_metadata)?;

    let coverage_symbols = extract_std_symbols_from_coverage_matrix(&paths.std_coverage_matrix_path)?;
    let catalog_value_symbols = catalog_value_symbol_set(&catalog);
    let codegen_ir_source = fs::read_to_string(&paths.codegen_ir_path)
        .map_err(|e| format!("read `{}`: {e}", paths.codegen_ir_path.display()))?;
    let codegen_symbols = extract_std_symbols_from_source(&codegen_ir_source)?;
    let expected_signatures = std_builtin_signatures_from_catalog(&catalog)?;
    let canonical_host_capabilities = canonical_host_capability_set();
    let actual_signatures = normalize_std_builtin_signature_file(StdBuiltinSignatureFile {
        schema_version: 1,
        symbols: collect_typed_signature_entries()
            .into_values()
            .map(|entry| StdBuiltinSignatureEntry {
                symbol: entry.symbol.clone(),
                module_class: classify_std_module(entry.module.as_str()).to_string(),
                arity: entry.arity,
                effect: entry.effect,
                route: if codegen_symbols.contains(entry.symbol.as_str()) {
                    "intrinsic".to_string()
                } else {
                    "package_import".to_string()
                },
                capability: if canonical_host_capabilities.contains(entry.symbol.as_str()) {
                    Some(entry.symbol.clone())
                } else {
                    None
                },
                deprecated_alias_of: legacy_alias_target(entry.symbol.as_str()).map(str::to_string),
            })
            .collect(),
    })?;
    let catalog_host_capabilities = catalog_host_capability_set(&catalog)?;

    let mut errors = Vec::new();
    let expected_metadata_json =
        serde_json::to_string(&expected_metadata).map_err(|e| format!("serialize metadata: {e}"))?;
    let actual_metadata_json =
        serde_json::to_string(&actual_metadata).map_err(|e| format!("serialize metadata: {e}"))?;
    if expected_metadata_json != actual_metadata_json {
        errors.push(format!(
            "std metadata drift: `{}` is not generated from `{}` (run `cargo run -p xtask -- std-arch-sync --write`)",
            paths.std_metadata_path.display(),
            paths.catalog_lock_path.display()
        ));
    }
    if expected_signatures != actual_signatures {
        errors.push(format!(
            "builtin signature drift: typer builtin surface does not match canonical catalog `{}`",
            paths.catalog_lock_path.display()
        ));
    }

    let missing_coverage = catalog_value_symbols
        .difference(&coverage_symbols)
        .cloned()
        .collect::<Vec<_>>();
    if !missing_coverage.is_empty() {
        errors.push(format!(
            "coverage matrix missing catalog symbols [{}]",
            preview_symbols(&missing_coverage)
        ));
    }
    let missing_policy_caps = catalog_host_capabilities
        .difference(&canonical_host_capabilities)
        .cloned()
        .collect::<Vec<_>>();
    if !missing_policy_caps.is_empty() {
        errors.push(format!(
            "catalog host capabilities missing from host-policy lock [{}]",
            preview_symbols(&missing_policy_caps)
        ));
    }
    let policy_only_caps = canonical_host_capabilities
        .difference(&catalog_host_capabilities)
        .cloned()
        .collect::<Vec<_>>();
    if !policy_only_caps.is_empty() {
        errors.push(format!(
            "host-policy capabilities not present in canonical std catalog [{}]",
            preview_symbols(&policy_only_caps)
        ));
    }

    for entry in &expected_signatures.symbols {
        let expected_route = entry.route.as_str();
        let actual_route = if codegen_symbols.contains(entry.symbol.as_str()) {
            "intrinsic"
        } else {
            "package_import"
        };
        if expected_route != actual_route {
            errors.push(format!(
                "route drift for `{}`: catalog route `{}` but codegen route `{}`",
                entry.symbol, expected_route, actual_route
            ));
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "std architecture conformance drift detected:\n - {}",
            errors.join("\n - ")
        ))
    }
}

fn run_std_first_production_readiness_check(
    root: &Path,
    raw_args: Vec<String>,
) -> Result<(), String> {
    if !raw_args.is_empty() {
        return Err(
            "std-first-production-readiness-check does not accept args".to_string(),
        );
    }
    let paths = std_arch_paths(root);
    let coverage_raw = fs::read_to_string(&paths.std_coverage_matrix_path)
        .map_err(|e| format!("read `{}`: {e}", paths.std_coverage_matrix_path.display()))?;
    let coverage_entries = parse_std_coverage_entries(&coverage_raw)?;
    let metadata = load_std_metadata_root(&paths.std_metadata_path)?;
    let readme_raw = fs::read_to_string(&paths.std_readme_path)
        .map_err(|e| format!("read `{}`: {e}", paths.std_readme_path.display()))?;
    let maturity = parse_std_contract_maturity_matrix(&readme_raw)?;
    let blockers = evaluate_std_first_production_readiness_for_specs(
        &first_production_std_specs(),
        &coverage_entries,
        &metadata,
        &maturity,
    );
    if blockers.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "std first-production readiness blockers:\n - {}",
            blockers.join("\n - ")
        ))
    }
}

fn parse_std_arch_sync_args(raw_args: Vec<String>) -> Result<StdArchSyncOpts, String> {
    let mut write = false;
    let mut refresh_lock = false;
    let mut emit_artifact: Option<PathBuf> = None;
    let mut idx = 0usize;
    while idx < raw_args.len() {
        match raw_args[idx].as_str() {
            "--write" => write = true,
            "--refresh-lock" => refresh_lock = true,
            "--emit-artifact" => {
                idx += 1;
                let value = raw_args
                    .get(idx)
                    .ok_or_else(|| "missing value for `--emit-artifact`".to_string())?;
                emit_artifact = Some(PathBuf::from(value));
            }
            other => {
                return Err(format!(
                    "unknown std-arch-sync arg `{other}` (supported: --write, --refresh-lock, --emit-artifact)"
                ));
            }
        }
        idx += 1;
    }
    Ok(StdArchSyncOpts {
        write,
        refresh_lock,
        emit_artifact,
    })
}

fn std_arch_paths(root: &Path) -> StdArchPaths {
    StdArchPaths {
        catalog_lock_path: root
            .join("docs")
            .join("design")
            .join("phase-26.6-std-catalog.lock.json"),
        std_metadata_path: root
            .join("crates")
            .join("cli")
            .join("assets")
            .join("std-metadata.json"),
        std_signature_artifact_path: root
            .join("docs")
            .join("design")
            .join("phase-26.6-std-builtin-signatures.v1.json"),
        std_coverage_matrix_path: root
            .join("docs")
            .join("std")
            .join("coverage-matrix.md"),
        std_readme_path: root.join("docs").join("std").join("README.md"),
        codegen_ir_path: root
            .join("crates")
            .join("codegen-wasm")
            .join("src")
            .join("ir")
            .join("mod.rs"),
    }
}

fn collect_typed_signature_entries() -> BTreeMap<String, BuiltinEntry26> {
    let mut out = BTreeMap::new();
    for (symbol, params, _ret, effect) in clg_typer::builtin_sigs() {
        let Some((module, _leaf)) = symbol.rsplit_once("::") else {
            continue;
        };
        let module = module.to_string();
        let effect = match effect {
            clg_ast::Effect::Pure => "pure",
            clg_ast::Effect::Mut => "mut",
            clg_ast::Effect::Io => "io",
            clg_ast::Effect::None => "pure",
        }
        .to_string();
        out.insert(
            symbol.clone(),
            BuiltinEntry26 {
                symbol,
                module,
                arity: params.len() as u32,
                effect,
            },
        );
    }
    for (symbol, arity, effect) in compatibility_typed_signature_entries() {
        let Some((module, _)) = symbol.rsplit_once("::") else {
            continue;
        };
        out.entry(symbol.to_string()).or_insert(BuiltinEntry26 {
            symbol: symbol.to_string(),
            module: module.to_string(),
            arity,
            effect: effect.to_string(),
        });
    }
    out
}

fn compatibility_typed_signature_entries() -> Vec<(&'static str, u32, &'static str)> {
    vec![
        ("std::array::len", 1, "pure"),
        ("std::slice::len", 1, "pure"),
        ("std::slice::get", 2, "pure"),
        ("std::slice::subslice", 3, "pure"),
        ("std::list::new", 0, "pure"),
        ("std::list::len", 1, "pure"),
        ("std::list::is_empty", 1, "pure"),
        ("std::list::get", 2, "pure"),
        ("std::list::push", 2, "pure"),
        ("std::list::insert", 3, "pure"),
        ("std::list::insert_checked", 3, "pure"),
        ("std::list::remove", 2, "pure"),
        ("std::list::remove_checked", 2, "pure"),
        ("std::list::remove_take", 2, "pure"),
        ("std::list::pop", 1, "pure"),
        ("std::list::can_mut", 1, "pure"),
        ("std::list::push_mut", 2, "mut"),
        ("std::list::insert_mut", 3, "mut"),
        ("std::list::remove_mut", 2, "mut"),
        ("std::list::pop_mut", 1, "mut"),
        ("std::set::new", 0, "pure"),
        ("std::set::len", 1, "pure"),
        ("std::set::is_empty", 1, "pure"),
        ("std::set::contains", 2, "pure"),
        ("std::set::insert", 2, "pure"),
        ("std::set::remove", 2, "pure"),
        ("std::set::subset", 2, "pure"),
        ("std::set::union", 2, "pure"),
        ("std::set::intersect", 2, "pure"),
        ("std::set::diff", 2, "pure"),
        ("std::set::can_mut", 1, "pure"),
        ("std::set::insert_mut", 2, "mut"),
        ("std::set::remove_mut", 2, "mut"),
        ("std::map::new", 0, "pure"),
        ("std::map::len", 1, "pure"),
        ("std::map::is_empty", 1, "pure"),
        ("std::map::contains", 2, "pure"),
        ("std::map::get", 2, "pure"),
        ("std::map::insert", 3, "pure"),
        ("std::map::insert_take", 3, "pure"),
        ("std::map::remove", 2, "pure"),
        ("std::map::remove_take", 2, "pure"),
        ("std::map::can_mut", 1, "pure"),
        ("std::map::insert_mut", 3, "mut"),
        ("std::map::remove_mut", 2, "mut"),
    ]
}

fn load_std_metadata_root(path: &Path) -> Result<StdMetadataRoot, String> {
    let raw = fs::read_to_string(path).map_err(|e| format!("read `{}`: {e}", path.display()))?;
    let parsed: StdMetadataRoot =
        serde_json::from_str(&raw).map_err(|e| format!("parse `{}`: {e}", path.display()))?;
    normalize_std_metadata_root(parsed)
}

fn normalize_std_metadata_root(mut root: StdMetadataRoot) -> Result<StdMetadataRoot, String> {
    if root.schema_version != 2 {
        return Err(format!(
            "unsupported std metadata schema version {} (expected 2)",
            root.schema_version
        ));
    }
    root.modules.sort_by(|a, b| a.path.cmp(&b.path));
    let mut seen_modules = BTreeSet::new();
    for module in &mut root.modules {
        if !seen_modules.insert(module.path.clone()) {
            return Err(format!("duplicate std metadata module `{}`", module.path));
        }
        module
            .exports
            .sort_by(|a, b| a.name.cmp(&b.name).then_with(|| a.kind.cmp(&b.kind)));
    }
    Ok(root)
}

fn generate_catalog_from_runtime_sources(
    builtin_entries: &BTreeMap<String, BuiltinEntry26>,
    metadata: &StdMetadataRoot,
    codegen_symbols: &BTreeSet<String>,
    host_capabilities: &BTreeSet<String>,
) -> Result<StdCatalogLockFile, String> {
    let mut modules: BTreeMap<String, Vec<StdCatalogExport>> = BTreeMap::new();
    for module in &metadata.modules {
        for export in &module.exports {
            if export.kind == "type" {
                modules
                    .entry(module.path.clone())
                    .or_default()
                    .push(StdCatalogExport {
                        name: export.name.clone(),
                        kind: "type".to_string(),
                        arity: None,
                        effect: None,
                        route: None,
                        capability: None,
                        deprecated_alias_of: None,
                        layout: export.layout.clone(),
                    });
            }
        }
    }

    for entry in builtin_entries.values() {
        let path = entry.module.clone();
        let leaf = entry
            .symbol
            .rsplit_once("::")
            .map(|(_, leaf)| leaf.to_string())
            .ok_or_else(|| format!("invalid builtin symbol `{}`", entry.symbol))?;
        let route = if codegen_symbols.contains(entry.symbol.as_str()) {
            "intrinsic".to_string()
        } else {
            "package_import".to_string()
        };
        modules.entry(path).or_default().push(StdCatalogExport {
            name: leaf,
            kind: "value".to_string(),
            arity: Some(entry.arity),
            effect: Some(entry.effect.clone()),
            route: Some(route),
            capability: if host_capabilities.contains(entry.symbol.as_str()) {
                Some(entry.symbol.clone())
            } else {
                None
            },
            deprecated_alias_of: legacy_alias_target(entry.symbol.as_str()).map(str::to_string),
            layout: None,
        });
    }

    let mut out_modules = Vec::new();
    for (path, mut exports) in modules {
        exports.sort_by(|a, b| a.name.cmp(&b.name).then_with(|| a.kind.cmp(&b.kind)));
        out_modules.push(StdCatalogModule {
            path: path.clone(),
            module_class: classify_std_module(path.as_str()).to_string(),
            exports,
        });
    }
    let lock = StdCatalogLockFile {
        schema_version: 1,
        modules: out_modules,
    };
    normalize_std_catalog_lock(lock)
}

fn std_metadata_from_catalog(catalog: &StdCatalogLockFile) -> StdMetadataRoot {
    let mut modules = Vec::new();
    for module in &catalog.modules {
        let mut exports = Vec::new();
        for export in &module.exports {
            exports.push(StdMetadataExport {
                name: export.name.clone(),
                kind: export.kind.clone(),
                layout: export.layout.clone(),
            });
        }
        modules.push(StdMetadataModule {
            path: module.path.clone(),
            exports,
        });
    }
    StdMetadataRoot {
        schema_version: 2,
        modules,
    }
}

fn std_builtin_signatures_from_catalog(
    catalog: &StdCatalogLockFile,
) -> Result<StdBuiltinSignatureFile, String> {
    let mut symbols = Vec::new();
    for module in &catalog.modules {
        for export in &module.exports {
            if export.kind != "value" {
                continue;
            }
            let symbol = format!("{}::{}", module.path, export.name);
            let arity = export
                .arity
                .ok_or_else(|| format!("catalog value `{symbol}` missing arity"))?;
            let effect = export
                .effect
                .clone()
                .ok_or_else(|| format!("catalog value `{symbol}` missing effect"))?;
            let route = export
                .route
                .clone()
                .ok_or_else(|| format!("catalog value `{symbol}` missing route"))?;
            symbols.push(StdBuiltinSignatureEntry {
                symbol,
                module_class: module.module_class.clone(),
                arity,
                effect,
                route,
                capability: export.capability.clone(),
                deprecated_alias_of: export.deprecated_alias_of.clone(),
            });
        }
    }
    normalize_std_builtin_signature_file(StdBuiltinSignatureFile {
        schema_version: 1,
        symbols,
    })
}

fn normalize_std_catalog_lock(mut lock: StdCatalogLockFile) -> Result<StdCatalogLockFile, String> {
    if lock.schema_version != 1 {
        return Err(format!(
            "unsupported std catalog lock schema_version {} (expected 1)",
            lock.schema_version
        ));
    }
    lock.modules.sort_by(|a, b| a.path.cmp(&b.path));
    let mut seen_modules = BTreeSet::new();
    for module in &mut lock.modules {
        if module.module_class != "pure_std" && module.module_class != "host_std" {
            return Err(format!(
                "invalid module_class `{}` for `{}`",
                module.module_class, module.path
            ));
        }
        if !seen_modules.insert(module.path.clone()) {
            return Err(format!("duplicate std catalog module `{}`", module.path));
        }
        module
            .exports
            .sort_by(|a, b| a.name.cmp(&b.name).then_with(|| a.kind.cmp(&b.kind)));
        let mut seen_exports = BTreeSet::new();
        for export in &module.exports {
            let key = format!("{}::{}:{}", module.path, export.name, export.kind);
            if !seen_exports.insert(key.clone()) {
                return Err(format!("duplicate std catalog export `{key}`"));
            }
            if export.kind != "value" && export.kind != "type" {
                return Err(format!("invalid export kind `{}` for `{}`", export.kind, key));
            }
            if export.kind == "value" {
                let symbol = format!("{}::{}", module.path, export.name);
                let effect = export
                    .effect
                    .as_ref()
                    .ok_or_else(|| format!("catalog value `{symbol}` missing effect"))?;
                if effect != "pure" && effect != "mut" && effect != "io" {
                    return Err(format!("invalid effect `{effect}` for `{symbol}`"));
                }
                let route = export
                    .route
                    .as_ref()
                    .ok_or_else(|| format!("catalog value `{symbol}` missing route"))?;
                if route != "intrinsic" && route != "package_import" {
                    return Err(format!("invalid route `{route}` for `{symbol}`"));
                }
                if export.arity.is_none() {
                    return Err(format!("catalog value `{symbol}` missing arity"));
                }
            }
        }
    }
    Ok(lock)
}

fn normalize_std_builtin_signature_file(
    mut file: StdBuiltinSignatureFile,
) -> Result<StdBuiltinSignatureFile, String> {
    if file.schema_version != 1 {
        return Err(format!(
            "unsupported std builtin signatures schema_version {} (expected 1)",
            file.schema_version
        ));
    }
    file.symbols.sort_by(|a, b| a.symbol.cmp(&b.symbol));
    let mut seen = BTreeSet::new();
    for entry in &file.symbols {
        if !seen.insert(entry.symbol.clone()) {
            return Err(format!("duplicate std builtin signature `{}`", entry.symbol));
        }
    }
    Ok(file)
}

fn extract_std_symbols_from_coverage_matrix(path: &Path) -> Result<BTreeSet<String>, String> {
    let raw = fs::read_to_string(path).map_err(|e| format!("read `{}`: {e}", path.display()))?;
    let entries = parse_std_coverage_entries(&raw)?;
    Ok(entries.into_iter().map(|entry| entry.symbol).collect())
}

fn parse_std_coverage_entries(raw: &str) -> Result<Vec<StdCoverageEntry>, String> {
    let mut entries = Vec::new();
    for line in raw.lines() {
        let trimmed = line.trim();
        if !trimmed.starts_with('|') {
            continue;
        }
        let columns = trimmed
            .split('|')
            .map(str::trim)
            .collect::<Vec<_>>();
        if columns.len() < 6 {
            continue;
        }
        let symbol_cell = columns[1];
        if symbol_cell.is_empty() || symbol_cell == "Symbol" || symbol_cell == "---" {
            continue;
        }
        for symbol in expand_coverage_symbol_cell(symbol_cell) {
            let Some((module_path, _leaf)) = symbol.rsplit_once("::") else {
                continue;
            };
            let module_path = module_path.to_string();
            entries.push(StdCoverageEntry {
                symbol,
                module_path,
                typed: columns[2].to_string(),
                runtime: columns[3].to_string(),
                proved: columns[4].to_string(),
            });
        }
    }
    Ok(entries)
}

fn expand_coverage_symbol_cell(cell: &str) -> Vec<String> {
    static COVERAGE_GROUP_PATTERN: std::sync::OnceLock<Regex> = std::sync::OnceLock::new();
    static COVERAGE_SYMBOL_PATTERN: std::sync::OnceLock<Regex> = std::sync::OnceLock::new();
    let normalized = cell.replace('`', "").trim().to_string();
    let group_pattern = COVERAGE_GROUP_PATTERN.get_or_init(|| {
        Regex::new(r"^(std::[A-Za-z0-9_]+(?:::[A-Za-z0-9_]+)*)::\{([^}]*)\}$")
            .expect("valid grouped coverage regex")
    });
    if let Some(captures) = group_pattern.captures(normalized.as_str()) {
        let module = captures.get(1).map(|m| m.as_str()).unwrap_or_default();
        let leaves = captures.get(2).map(|m| m.as_str()).unwrap_or_default();
        return leaves
            .split(',')
            .map(str::trim)
            .filter(|leaf| !leaf.is_empty())
            .map(|leaf| format!("{module}::{leaf}"))
            .collect();
    }

    let symbol_pattern = COVERAGE_SYMBOL_PATTERN.get_or_init(|| {
        Regex::new(r"^(std::[A-Za-z0-9_]+(?:::[A-Za-z0-9_]+)*)$")
            .expect("valid explicit coverage regex")
    });
    if symbol_pattern.is_match(normalized.as_str()) {
        return vec![normalized];
    }
    Vec::new()
}

fn parse_std_contract_maturity_matrix(raw: &str) -> Result<BTreeMap<String, String>, String> {
    static MATURITY_LINE_PATTERN: std::sync::OnceLock<Regex> = std::sync::OnceLock::new();
    let line_pattern = MATURITY_LINE_PATTERN.get_or_init(|| {
        Regex::new(r"^- (.+): `([^`]+)`").expect("valid maturity matrix regex")
    });
    let mut out = BTreeMap::new();
    for line in raw.lines() {
        let trimmed = line.trim();
        let Some(captures) = line_pattern.captures(trimmed) else {
            continue;
        };
        let label = captures.get(1).map(|m| m.as_str()).unwrap_or_default();
        let status = captures.get(2).map(|m| m.as_str()).unwrap_or_default();
        out.insert(
            label.replace('`', "").trim().to_string(),
            status.trim().to_string(),
        );
    }
    if out.is_empty() {
        return Err("std contract maturity matrix yielded zero entries".to_string());
    }
    Ok(out)
}

fn evaluate_std_first_production_readiness_for_specs(
    specs: &[FirstProductionStdSpec],
    coverage_entries: &[StdCoverageEntry],
    metadata: &StdMetadataRoot,
    maturity: &BTreeMap<String, String>,
) -> Vec<String> {
    let metadata_modules = metadata
        .modules
        .iter()
        .map(|module| module.path.clone())
        .collect::<BTreeSet<_>>();
    let mut blockers = Vec::new();

    for spec in specs {
        if spec.logical_surface {
            match maturity.get(spec.maturity_key) {
                Some(status) if status == "ready" => {}
                Some(status) => blockers.push(format!(
                    "{} maturity is `{}` in docs/std/README.md; logical built-in surface is not first-production ready",
                    spec.package, status
                )),
                None => blockers.push(format!(
                    "docs/std/README.md contract maturity matrix missing `{}` entry",
                    spec.maturity_key
                )),
            }
            continue;
        }

        let family_entries = coverage_entries
            .iter()
            .filter(|entry| {
                spec.coverage_prefixes.iter().any(|prefix| {
                    entry.symbol == *prefix || entry.symbol.starts_with(&format!("{prefix}::"))
                })
            })
            .cloned()
            .collect::<Vec<_>>();
        if family_entries.is_empty() {
            blockers.push(format!(
                "{} coverage is missing concrete rows in docs/std/coverage-matrix.md",
                spec.package
            ));
            continue;
        }

        let pending = family_entries
            .iter()
            .filter(|entry| {
                entry.proved != "deferred" && (entry.typed != "yes" || entry.runtime != "yes")
            })
            .map(|entry| {
                format!(
                    "{} (typed={}, runtime={}, proved={})",
                    entry.symbol, entry.typed, entry.runtime, entry.proved
                )
            })
            .collect::<Vec<_>>();
        if !pending.is_empty() {
            blockers.push(format!(
                "{} has first-production rows without implementation [{}]",
                spec.package,
                preview_symbols(&pending)
            ));
        }

        let required_modules = family_entries
            .iter()
            .filter(|entry| entry.proved != "deferred")
            .map(|entry| entry.module_path.clone())
            .collect::<BTreeSet<_>>();
        let missing_modules = required_modules
            .difference(&metadata_modules)
            .cloned()
            .collect::<Vec<_>>();
        if !missing_modules.is_empty() {
            blockers.push(format!(
                "{} metadata is missing required modules [{}]",
                spec.package,
                preview_symbols(&missing_modules)
            ));
        }
    }

    blockers
}

fn first_production_std_specs() -> Vec<FirstProductionStdSpec> {
    vec![
        FirstProductionStdSpec {
            package: "std::core",
            maturity_key: "std::core",
            coverage_prefixes: Vec::new(),
            logical_surface: true,
        },
        FirstProductionStdSpec {
            package: "std::str",
            maturity_key: "std::str",
            coverage_prefixes: vec!["std::str", "std::str_pattern"],
            logical_surface: false,
        },
        FirstProductionStdSpec {
            package: "std::bytes",
            maturity_key: "std::bytes",
            coverage_prefixes: vec!["std::bytes", "std::bytes_error"],
            logical_surface: false,
        },
        FirstProductionStdSpec {
            package: "std::int",
            maturity_key: "std::int",
            coverage_prefixes: vec!["std::u64", "std::u128", "std::u256", "std::checked", "std::int_error"],
            logical_surface: false,
        },
        FirstProductionStdSpec {
            package: "collections",
            maturity_key: "Collections catalog (std::list, std::set, std::map)",
            coverage_prefixes: vec!["std::list", "std::set", "std::map"],
            logical_surface: false,
        },
        FirstProductionStdSpec {
            package: "std::codec",
            maturity_key: "std::codec",
            coverage_prefixes: vec!["std::encoder", "std::decoder", "std::decode_error", "std::encode_error"],
            logical_surface: false,
        },
        FirstProductionStdSpec {
            package: "std::crypto",
            maturity_key: "std::crypto",
            coverage_prefixes: vec!["std::crypto"],
            logical_surface: false,
        },
        FirstProductionStdSpec {
            package: "std::host",
            maturity_key: "std::host",
            coverage_prefixes: vec!["std::host"],
            logical_surface: false,
        },
        FirstProductionStdSpec {
            package: "std::unit",
            maturity_key: "std::unit",
            coverage_prefixes: vec!["std::unit"],
            logical_surface: false,
        },
        FirstProductionStdSpec {
            package: "std::contract",
            maturity_key: "std::contract",
            coverage_prefixes: vec!["std::contract"],
            logical_surface: false,
        },
    ]
}

fn catalog_value_symbol_set(catalog: &StdCatalogLockFile) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    for module in &catalog.modules {
        for export in &module.exports {
            if export.kind == "value" {
                out.insert(format!("{}::{}", module.path, export.name));
            }
        }
    }
    out
}

fn catalog_host_capability_set(catalog: &StdCatalogLockFile) -> Result<BTreeSet<String>, String> {
    let mut out = BTreeSet::new();
    for module in &catalog.modules {
        for export in &module.exports {
            if let Some(capability) = &export.capability {
                let symbol = format!("{}::{}", module.path, export.name);
                if capability != &symbol {
                    return Err(format!(
                        "catalog capability for `{symbol}` must equal symbol, found `{capability}`"
                    ));
                }
                out.insert(capability.clone());
            }
        }
    }
    Ok(out)
}

fn canonical_host_capability_set() -> BTreeSet<String> {
    let policy = canonical_host_capability_policy_v1();
    policy
        .profiles
        .iter()
        .flat_map(|profile| profile.capabilities.iter().map(|rule| rule.capability.clone()))
        .collect()
}

fn classify_std_module(module: &str) -> &'static str {
    match module {
        "std::crypto" | "std::env" | "std::wasi" => "host_std",
        _ => "pure_std",
    }
}

fn legacy_alias_target(symbol: &str) -> Option<&'static str> {
    match symbol {
        "std::bytes::equals" => Some("std::bytes::eq"),
        "std::bytes::equals_ct" => Some("std::bytes::eq_ct"),
        "std::str::equals" => Some("std::str::eq"),
        "std::u64::add_wrap" => Some("std::u64::add_wrapping"),
        "std::u64::sub_wrap" => Some("std::u64::sub_wrapping"),
        "std::u64::mul_wrap" => Some("std::u64::mul_wrapping"),
        "std::u64::add_sat" => Some("std::u64::add_saturating"),
        "std::u64::sub_sat" => Some("std::u64::sub_saturating"),
        "std::u64::mul_sat" => Some("std::u64::mul_saturating"),
        "std::crypto::sha256" => Some("std::crypto::hash"),
        "std::crypto::hmac_sha256" => Some("std::crypto::hmac"),
        _ => None,
    }
}

#[derive(Clone, Debug)]
struct StdArchSyncOpts {
    write: bool,
    refresh_lock: bool,
    emit_artifact: Option<PathBuf>,
}

#[derive(Clone, Debug)]
struct StdArchPaths {
    catalog_lock_path: PathBuf,
    std_metadata_path: PathBuf,
    std_signature_artifact_path: PathBuf,
    std_coverage_matrix_path: PathBuf,
    std_readme_path: PathBuf,
    codegen_ir_path: PathBuf,
}

#[derive(Clone, Debug)]
struct BuiltinEntry26 {
    symbol: String,
    module: String,
    arity: u32,
    effect: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct StdCoverageEntry {
    symbol: String,
    module_path: String,
    typed: String,
    runtime: String,
    proved: String,
}

#[derive(Clone, Debug)]
struct FirstProductionStdSpec {
    package: &'static str,
    maturity_key: &'static str,
    coverage_prefixes: Vec<&'static str>,
    logical_surface: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct StdCatalogLockFile {
    schema_version: u32,
    modules: Vec<StdCatalogModule>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct StdCatalogModule {
    path: String,
    module_class: String,
    exports: Vec<StdCatalogExport>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct StdCatalogExport {
    name: String,
    kind: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    arity: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    effect: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    route: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    capability: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    deprecated_alias_of: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    layout: Option<serde_json::Value>,
}

#[derive(Clone, Debug, Deserialize, Serialize, Eq, PartialEq)]
struct StdBuiltinSignatureFile {
    schema_version: u32,
    symbols: Vec<StdBuiltinSignatureEntry>,
}

#[derive(Clone, Debug, Deserialize, Serialize, Eq, PartialEq)]
struct StdBuiltinSignatureEntry {
    symbol: String,
    module_class: String,
    arity: u32,
    effect: String,
    route: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    capability: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    deprecated_alias_of: Option<String>,
}
