fn run_milestone2_supply_chain_gate(root: &Path, raw_args: Vec<String>) -> Result<(), String> {
    let mut self_test = false;
    let mut out_dir_override: Option<PathBuf> = None;
    let mut idx = 0usize;
    while idx < raw_args.len() {
        match raw_args[idx].as_str() {
            "--self-test" => {
                if self_test {
                    return Err(
                        "usage: cargo run -p xtask -- milestone2-supply-chain-gate [--self-test] [--out-dir <DIR>]".into(),
                    );
                }
                self_test = true;
            }
            "--out-dir" => {
                idx += 1;
                let value = raw_args
                    .get(idx)
                    .ok_or_else(|| "missing value for `--out-dir`".to_string())?;
                out_dir_override = Some(PathBuf::from(value));
            }
            _ => {
                return Err(
                    "usage: cargo run -p xtask -- milestone2-supply-chain-gate [--self-test] [--out-dir <DIR>]".into(),
                );
            }
        }
        idx += 1;
    }

    if self_test && out_dir_override.is_some() {
        return Err(
            "usage: cargo run -p xtask -- milestone2-supply-chain-gate [--self-test] [--out-dir <DIR>]".into(),
        );
    }

    if self_test {
        run_supply_chain_self_test(root)?;
        println!("milestone_2 supply-chain gate self-test passed");
        return Ok(());
    }

    let out_dir = out_dir_override
        .or_else(|| env::var("CLG_SUPPLY_CHAIN_OUT_DIR").ok().map(PathBuf::from))
        .unwrap_or_else(|| root.join("tmp").join("sbom"));
    fs::create_dir_all(&out_dir)
        .map_err(|e| format!("create supply-chain output dir `{}`: {e}", out_dir.display()))?;

    let metadata = load_cargo_metadata(root)?;
    let dep_report = build_dependency_report(&metadata)?;
    let pkg_report = build_package_artifact_report(root)?;
    let runtime_report = build_runtime_dependency_report(root)?;

    write_json_pretty(&out_dir.join("milestone2-sbom.json"), &dep_report.report)?;
    write_json_pretty(
        &out_dir.join("milestone2-package-artifact-report.json"),
        &pkg_report.report,
    )?;
    write_json_pretty(
        &out_dir.join("milestone2-runtime-dependency-report.json"),
        &runtime_report.report,
    )?;
    let summary = json!({
      "schema_version": 1,
      "dependency_violations": dep_report.violations.len(),
      "package_artifact_violations": pkg_report.violations.len(),
      "runtime_dependency_violations": runtime_report.violations.len(),
      "ok": dep_report.violations.is_empty() && pkg_report.violations.is_empty() && runtime_report.violations.is_empty()
    });
    write_json_pretty(&out_dir.join("milestone2-supply-chain-summary.json"), &summary)?;

    if !dep_report.violations.is_empty() {
        eprintln!("dependency license compliance violations detected");
        for entry in dep_report.violations.iter().take(10) {
            eprintln!("- {}", entry);
        }
    }
    if !pkg_report.violations.is_empty() {
        eprintln!("package metadata artifact compliance violations detected");
        for entry in pkg_report.violations.iter().take(10) {
            eprintln!("- {}", entry);
        }
    }
    if !runtime_report.violations.is_empty() {
        eprintln!("runtime dependency compliance violations detected");
        for entry in runtime_report.violations.iter().take(10) {
            eprintln!("- {}", entry);
        }
    }

    if dep_report.violations.is_empty()
        && pkg_report.violations.is_empty()
        && runtime_report.violations.is_empty()
    {
        println!("milestone_2 supply-chain gate passed");
        println!("artifacts written to {}", out_dir.display());
        Ok(())
    } else {
        Err("milestone_2 supply-chain gate failed".into())
    }
}

fn load_cargo_metadata(root: &Path) -> Result<Value, String> {
    if let Ok(path) = env::var("CLG_SUPPLY_CHAIN_METADATA_JSON") {
        return serde_json::from_slice(
            &fs::read(&path).map_err(|e| format!("read metadata override `{path}`: {e}"))?,
        )
        .map_err(|e| format!("parse metadata override `{path}`: {e}"));
    }
    let output = run_capture_output(
        Command::new("cargo")
            .arg("metadata")
            .arg("--format-version")
            .arg("1")
            .arg("--locked")
            .current_dir(root),
    )?;
    serde_json::from_slice(&output).map_err(|e| format!("parse cargo metadata output: {e}"))
}

fn build_dependency_report(metadata: &Value) -> Result<SupplyChainReport, String> {
    let mut package_map = std::collections::BTreeMap::<String, &Value>::new();
    if let Some(packages) = metadata.get("packages").and_then(Value::as_array) {
        for pkg in packages {
            if let Some(id) = pkg.get("id").and_then(Value::as_str) {
                package_map.insert(id.to_string(), pkg);
            }
        }
    }
    let workspace_members: std::collections::BTreeSet<String> = metadata
        .get("workspace_members")
        .and_then(Value::as_array)
        .map(|members| {
            members
                .iter()
                .filter_map(Value::as_str)
                .map(ToString::to_string)
                .collect()
        })
        .unwrap_or_default();

    let mut node_ids = Vec::<String>::new();
    if let Some(nodes) = metadata
        .get("resolve")
        .and_then(|v| v.get("nodes"))
        .and_then(Value::as_array)
    {
        for node in nodes {
            if let Some(id) = node.get("id").and_then(Value::as_str) {
                node_ids.push(id.to_string());
            }
        }
    }
    if node_ids.is_empty() {
        node_ids.extend(package_map.keys().cloned());
    }
    node_ids.sort();
    node_ids.dedup();

    let mut entries = Vec::<Value>::new();
    let mut violations = Vec::<Value>::new();
    for pkg_id in node_ids {
        let Some(pkg) = package_map.get(&pkg_id) else {
            continue;
        };
        let license = pkg
            .get("license")
            .and_then(Value::as_str)
            .unwrap_or("")
            .trim()
            .to_string();
        let license_file = pkg
            .get("license_file")
            .and_then(Value::as_str)
            .unwrap_or("")
            .trim()
            .to_string();
        let compliant = !license.is_empty() || !license_file.is_empty();
        let row = json!({
          "id": pkg_id,
          "name": pkg.get("name").and_then(Value::as_str),
          "version": pkg.get("version").and_then(Value::as_str),
          "source": pkg.get("source").and_then(Value::as_str),
          "license": if license.is_empty() { Value::Null } else { Value::String(license) },
          "license_file": if license_file.is_empty() { Value::Null } else { Value::String(license_file) },
          "workspace_member": workspace_members.contains(pkg.get("id").and_then(Value::as_str).unwrap_or("")),
          "compliant": compliant
        });
        if !compliant {
            violations.push(row.clone());
        }
        entries.push(row);
    }
    Ok(SupplyChainReport {
        report: json!({
          "schema_version": 1,
          "kind": "cargo_dependency_license_sbom",
          "packages": entries
        }),
        violations,
    })
}

fn build_package_artifact_report(root: &Path) -> Result<SupplyChainReport, String> {
    let mut metadata_files = collect_tracked_package_metadata_paths(root)?;
    metadata_files.extend(find_files_by_name(
        &root.join("tmp").join("std-core"),
        "clg.package-metadata.json",
    ));
    metadata_files.sort();
    metadata_files.dedup();

    let mut entries = Vec::<Value>::new();
    let mut violations = Vec::<Value>::new();
    for metadata_path in metadata_files.into_iter().filter(|path| path.exists()) {
        let data: Value = serde_json::from_slice(
            &fs::read(&metadata_path)
                .map_err(|e| format!("read package metadata `{}`: {e}", metadata_path.display()))?,
        )
        .map_err(|e| format!("parse package metadata `{}`: {e}", metadata_path.display()))?;
        let schema_version = data.get("schema_version").and_then(Value::as_i64);
        if let Some(packages) = data.get("packages").and_then(Value::as_array) {
            let mut sorted_packages = packages.clone();
            sorted_packages.sort_by(|a, b| {
                let an = a.get("name").and_then(Value::as_str).unwrap_or("");
                let av = a.get("version").and_then(Value::as_str).unwrap_or("");
                let bn = b.get("name").and_then(Value::as_str).unwrap_or("");
                let bv = b.get("version").and_then(Value::as_str).unwrap_or("");
                (an, av).cmp(&(bn, bv))
            });
            for pkg in sorted_packages {
                let digest = pkg
                    .get("digest")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .trim()
                    .to_string();
                let artifact_path = pkg
                    .get("artifact")
                    .and_then(|v| v.get("path"))
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .trim()
                    .to_string();
                let artifact_exists = if artifact_path.is_empty() {
                    false
                } else {
                    metadata_path
                        .parent()
                        .unwrap_or(root)
                        .join(&artifact_path)
                        .exists()
                };
                let compliant = matches!(schema_version, Some(0) | Some(1))
                    && digest.starts_with("sha256:")
                    && !artifact_path.is_empty()
                    && artifact_exists;
                let row = json!({
                  "metadata_file": normalize_rel_path(root, &metadata_path),
                  "name": pkg.get("name").and_then(Value::as_str),
                  "version": pkg.get("version").and_then(Value::as_str),
                  "schema_version": schema_version,
                  "digest": if digest.is_empty() { Value::Null } else { Value::String(digest) },
                  "artifact_path": if artifact_path.is_empty() { Value::Null } else { Value::String(artifact_path) },
                  "artifact_exists": artifact_exists,
                  "compliant": compliant
                });
                if !compliant {
                    violations.push(row.clone());
                }
                entries.push(row);
            }
        }
    }
    Ok(SupplyChainReport {
        report: json!({
          "schema_version": 1,
          "kind": "package_metadata_artifact_compliance",
          "entries": entries
        }),
        violations,
    })
}

fn build_runtime_dependency_report(root: &Path) -> Result<SupplyChainReport, String> {
    let mut runtime_files = collect_tracked_runtime_link_paths(root)?;
    runtime_files.extend(find_files_by_name(
        &root.join("tmp").join("perf"),
        "clg.runtime-link.json",
    ));
    runtime_files.sort();
    runtime_files.dedup();

    let mut entries = Vec::<Value>::new();
    let mut violations = Vec::<Value>::new();
    if runtime_files.is_empty() {
        let row = json!({
          "entry_kind": "runtime_link_presence",
          "runtime_link_file": Value::Null,
          "compliant": false,
          "reason": "missing_runtime_link_artifacts"
        });
        violations.push(row.clone());
        entries.push(row);
    }

    for runtime_link_path in runtime_files.into_iter().filter(|path| path.exists()) {
        let data: Value = serde_json::from_slice(
            &fs::read(&runtime_link_path).map_err(|e| {
                format!("read runtime-link artifact `{}`: {e}", runtime_link_path.display())
            })?,
        )
        .map_err(|e| format!("parse runtime-link artifact `{}`: {e}", runtime_link_path.display()))?;
        let schema_version = data.get("schema_version").and_then(Value::as_i64);
        let lock_path = runtime_link_path.parent().unwrap_or(root).join("clg.lock.json");
        let store_index_path = runtime_link_path
            .parent()
            .unwrap_or(root)
            .join("clg.package-store-index.json");
        let lock_map = read_lock_digests(&lock_path)?;
        let store_map = read_store_digests(&store_index_path)?;

        let mut package_ids = std::collections::BTreeSet::<String>::new();
        let mut packages = data
            .get("packages")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        packages.sort_by(|a, b| {
            a.get("id")
                .and_then(Value::as_str)
                .unwrap_or("")
                .cmp(b.get("id").and_then(Value::as_str).unwrap_or(""))
        });
        for pkg in packages {
            let package_id = pkg
                .get("id")
                .and_then(Value::as_str)
                .unwrap_or("")
                .trim()
                .to_string();
            if !package_id.is_empty() {
                package_ids.insert(package_id.clone());
            }
            let digest = pkg
                .get("digest")
                .and_then(Value::as_str)
                .unwrap_or("")
                .trim()
                .to_string();
            let artifact_path = pkg
                .get("artifact_path")
                .and_then(Value::as_str)
                .unwrap_or("")
                .trim()
                .to_string();
            let artifact_full = runtime_link_path.parent().unwrap_or(root).join(&artifact_path);
            let artifact_exists = !artifact_path.is_empty() && artifact_full.exists();
            let digest_matches_artifact = if artifact_exists && digest.starts_with("sha256:") {
                sha256_hex(&artifact_full)
                    .map(|sum| digest == format!("sha256:{sum}"))
                    .unwrap_or(false)
            } else {
                false
            };
            let lock_digest_matches = !package_id.is_empty()
                && lock_map
                    .get(&package_id)
                    .map(|d| d == &digest)
                    .unwrap_or(false);
            let store_digest_matches = !package_id.is_empty()
                && store_map
                    .get(&package_id)
                    .map(|d| d == &digest)
                    .unwrap_or(false);
            let compliant = matches!(schema_version, Some(0) | Some(1))
                && !package_id.is_empty()
                && digest.starts_with("sha256:")
                && !artifact_path.is_empty()
                && artifact_exists
                && digest_matches_artifact
                && lock_digest_matches
                && store_digest_matches;
            let row = json!({
              "entry_kind": "runtime_package",
              "runtime_link_file": normalize_rel_path(root, &runtime_link_path),
              "lockfile_present": lock_path.exists(),
              "store_index_present": store_index_path.exists(),
              "schema_version": schema_version,
              "package_id": if package_id.is_empty() { Value::Null } else { Value::String(package_id.clone()) },
              "digest": if digest.is_empty() { Value::Null } else { Value::String(digest.clone()) },
              "artifact_path": if artifact_path.is_empty() { Value::Null } else { Value::String(artifact_path) },
              "artifact_exists": artifact_exists,
              "digest_matches_artifact": digest_matches_artifact,
              "lock_digest_matches": lock_digest_matches,
              "store_digest_matches": store_digest_matches,
              "compliant": compliant
            });
            if !compliant {
                violations.push(row.clone());
            }
            entries.push(row);
        }

        let mut bindings = data
            .get("bindings")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        bindings.sort_by(|a, b| {
            let ak = (
                a.get("import_module").and_then(Value::as_str).unwrap_or(""),
                a.get("import_name").and_then(Value::as_str).unwrap_or(""),
                a.get("provider_package_id")
                    .and_then(Value::as_str)
                    .unwrap_or(""),
            );
            let bk = (
                b.get("import_module").and_then(Value::as_str).unwrap_or(""),
                b.get("import_name").and_then(Value::as_str).unwrap_or(""),
                b.get("provider_package_id")
                    .and_then(Value::as_str)
                    .unwrap_or(""),
            );
            ak.cmp(&bk)
        });
        for binding in bindings {
            let provider_package_id = binding
                .get("provider_package_id")
                .and_then(Value::as_str)
                .unwrap_or("")
                .trim()
                .to_string();
            let compliant =
                !provider_package_id.is_empty() && package_ids.contains(&provider_package_id);
            let row = json!({
              "entry_kind": "runtime_binding",
              "runtime_link_file": normalize_rel_path(root, &runtime_link_path),
              "import_module": binding.get("import_module").and_then(Value::as_str),
              "import_name": binding.get("import_name").and_then(Value::as_str),
              "provider_package_id": if provider_package_id.is_empty() { Value::Null } else { Value::String(provider_package_id) },
              "compliant": compliant
            });
            if !compliant {
                violations.push(row.clone());
            }
            entries.push(row);
        }
    }

    Ok(SupplyChainReport {
        report: json!({
          "schema_version": 1,
          "kind": "runtime_dependency_compliance",
          "entries": entries
        }),
        violations,
    })
}

fn collect_tracked_package_metadata_paths(root: &Path) -> Result<Vec<PathBuf>, String> {
    if let Ok(raw) = env::var("CLG_SUPPLY_CHAIN_TRACKED_METADATA") {
        let mut out = Vec::new();
        for line in raw.lines().map(str::trim).filter(|line| !line.is_empty()) {
            out.push(root.join(line));
        }
        return Ok(out);
    }
    match run_capture_output(
        Command::new("git")
            .arg("ls-files")
            .arg("--")
            .arg("**/clg.package-metadata.json")
            .current_dir(root),
    ) {
        Ok(stdout) => Ok(String::from_utf8_lossy(&stdout)
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty())
            .map(|rel| root.join(rel))
            .collect()),
        Err(_) => {
            let mut out = Vec::new();
            recurse_files(root, &mut out, "clg.package-metadata.json", &[".git", "target"]);
            Ok(out)
        }
    }
}

fn collect_tracked_runtime_link_paths(root: &Path) -> Result<Vec<PathBuf>, String> {
    if let Ok(raw) = env::var("CLG_SUPPLY_CHAIN_RUNTIME_LINKS") {
        let mut out = Vec::new();
        for line in raw.lines().map(str::trim).filter(|line| !line.is_empty()) {
            out.push(root.join(line));
        }
        return Ok(out);
    }
    match run_capture_output(
        Command::new("git")
            .arg("ls-files")
            .arg("--")
            .arg("**/clg.runtime-link.json")
            .current_dir(root),
    ) {
        Ok(stdout) => Ok(String::from_utf8_lossy(&stdout)
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty())
            .map(|rel| root.join(rel))
            .collect()),
        Err(_) => Ok(Vec::new()),
    }
}

fn find_files_by_name(base: &Path, name: &str) -> Vec<PathBuf> {
    let mut out = Vec::new();
    recurse_files(base, &mut out, name, &[]);
    out
}

fn recurse_files(base: &Path, out: &mut Vec<PathBuf>, target_name: &str, skip_dirs: &[&str]) {
    if !base.exists() || !base.is_dir() {
        return;
    }
    let entries = match fs::read_dir(base) {
        Ok(entries) => entries,
        Err(_) => return,
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
                continue;
            };
            if skip_dirs.contains(&name) {
                continue;
            }
            recurse_files(&path, out, target_name, skip_dirs);
            continue;
        }
        if path
            .file_name()
            .and_then(|name| name.to_str())
            .map(|name| name == target_name)
            .unwrap_or(false)
        {
            out.push(path);
        }
    }
}

fn normalize_rel_path(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .map(|rel| rel.to_string_lossy().replace('\\', "/"))
        .unwrap_or_else(|_| path.to_string_lossy().replace('\\', "/"))
}

fn read_lock_digests(path: &Path) -> Result<std::collections::BTreeMap<String, String>, String> {
    if !path.exists() {
        return Ok(std::collections::BTreeMap::new());
    }
    let data: Value = serde_json::from_slice(
        &fs::read(path).map_err(|e| format!("read lockfile `{}`: {e}", path.display()))?,
    )
    .map_err(|e| format!("parse lockfile `{}`: {e}", path.display()))?;
    let mut out = std::collections::BTreeMap::new();
    if let Some(packages) = data.get("packages").and_then(Value::as_array) {
        for pkg in packages {
            if let (Some(id), Some(digest)) = (
                pkg.get("id").and_then(Value::as_str),
                pkg.get("digest").and_then(Value::as_str),
            ) {
                out.insert(id.to_string(), digest.to_string());
            }
        }
    }
    Ok(out)
}

fn read_store_digests(path: &Path) -> Result<std::collections::BTreeMap<String, String>, String> {
    if !path.exists() {
        return Ok(std::collections::BTreeMap::new());
    }
    let data: Value = serde_json::from_slice(
        &fs::read(path).map_err(|e| format!("read store index `{}`: {e}", path.display()))?,
    )
    .map_err(|e| format!("parse store index `{}`: {e}", path.display()))?;
    let mut out = std::collections::BTreeMap::new();
    if let Some(artifacts) = data.get("artifacts").and_then(Value::as_array) {
        for artifact in artifacts {
            if let (Some(id), Some(digest)) = (
                artifact.get("id").and_then(Value::as_str),
                artifact.get("digest").and_then(Value::as_str),
            ) {
                out.insert(id.to_string(), digest.to_string());
            }
        }
    }
    Ok(out)
}

