fn write_resolved_graph_artifact(root: &Path, lockfile: &StrictLockfile) -> Result<String> {
    let path = root.join(RESOLVED_GRAPH_FILE);
    let hash_path = root.join(RESOLVED_GRAPH_HASH_FILE);
    let mut bytes = canonical_resolved_graph_bytes(lockfile)?;
    let hash = sha256_hex(bytes.as_slice());
    bytes.push(b'\n');
    fs::write(&path, bytes).with_context(|| format!("writing {}", path.display()))?;
    fs::write(&hash_path, format!("{hash}\n"))
        .with_context(|| format!("writing {}", hash_path.display()))?;
    Ok(hash)
}

fn canonical_resolved_graph_bytes(lockfile: &StrictLockfile) -> Result<Vec<u8>> {
    let mut packages = Vec::with_capacity(lockfile.packages().len());
    for pkg in lockfile.packages() {
        let mut deps = pkg.dependencies.clone();
        deps.sort();
        packages.push(ResolvedGraphPackageV1 {
            id: pkg.id.clone(),
            dependencies: deps,
        });
    }
    packages.sort_by(|a, b| a.id.cmp(&b.id));
    let artifact = ResolvedGraphArtifactV1 {
        schema_version: lockfile.schema_version(),
        resolver_version: lockfile.resolver_version(),
        roots: lockfile.roots().to_vec(),
        packages,
    };
    let value = serde_json::to_value(artifact).context("serializing resolved graph value")?;
    Ok(canonical_json_bytes(&value))
}

fn write_lockfile(path: &Path, lockfile: &StrictLockfile) -> Result<String> {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent).with_context(|| format!("creating {}", parent.display()))?;
        }
    }
    let mut bytes = canonical_lockfile_bytes(lockfile)?;
    let canonical_hash = sha256_hex(bytes.as_slice());
    bytes.push(b'\n');
    fs::write(path, bytes).with_context(|| format!("writing {}", path.display()))?;
    Ok(canonical_hash)
}

fn canonical_lockfile_bytes(lockfile: &StrictLockfile) -> Result<Vec<u8>> {
    let value = match lockfile {
        StrictLockfile::V1(lockfile) => {
            serde_json::to_value(lockfile).context("serializing strict lockfile v1 value")?
        }
        StrictLockfile::V2(lockfile) => {
            serde_json::to_value(lockfile).context("serializing strict lockfile v2 value")?
        }
    };
    Ok(canonical_json_bytes(&value))
}

