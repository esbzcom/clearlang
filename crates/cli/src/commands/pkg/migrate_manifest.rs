#[derive(Clone, Debug, Serialize)]
struct GeneratedManifestV1 {
    schema_version: u32,
    project: GeneratedProjectMetadataV1,
    dependencies: Vec<GeneratedDependencyV1>,
    release_defaults: GeneratedReleaseDefaultsV0,
}

#[derive(Clone, Debug, Serialize)]
struct GeneratedProjectMetadataV1 {
    name: String,
    description: String,
    version: String,
    clg_version: String,
    entry: String,
    website: String,
    contact: GeneratedContactV1,
}

#[derive(Clone, Debug, Serialize)]
struct GeneratedContactV1 {
    name: String,
    email: String,
}

#[derive(Clone, Debug, Serialize)]
struct GeneratedDependencyV1 {
    name: String,
    requirement: String,
}

#[derive(Clone, Debug, Serialize)]
struct GeneratedReleaseDefaultsV0 {
    advisory_as_of: String,
    key_id: String,
    out_dir: String,
    trust_policy: String,
}

pub struct RunMigrateManifestArgs {
    pub root: PathBuf,
    pub json_errors: bool,
    pub emit_stdout_summary: bool,
}

pub fn run_migrate_manifest(args: RunMigrateManifestArgs, logger: Logger) -> Result<()> {
    let RunMigrateManifestArgs {
        root,
        json_errors,
        emit_stdout_summary,
    } = args;
    let fail_pkg = |code: &'static str, message: String| -> Result<()> {
        if json_errors {
            let json = make_single_json_error(code, "build", message, &root, 0, 0, None);
            Err(CommandError::json(json).into())
        } else {
            Err(anyhow!(message))
        }
    };

    let mut timings = StageTimings::new();
    let manifest_path = root.join(STRICT_PROJECT_FILE);
    let metadata_path = root.join(CANONICAL_PACKAGE_METADATA_FILE);
    let lock_path = root.join(STRICT_LOCKFILE_FILE);
    let legacy_path = root.join(LEGACY_PACKAGE_METADATA_FILE);

    if manifest_path.exists() {
        return fail_pkg(
            "C109",
            format!(
                "manifest migration conflict: `{}` already exists; remove or migrate manually before rerunning `clg pkg migrate-manifest`",
                manifest_path.display()
            ),
        );
    }
    if legacy_path.exists() {
        return fail_pkg(
            "C109",
            format!(
                "manifest migration conflict: legacy metadata `{}` is not supported; migrate to canonical `{}` first",
                legacy_path.display(),
                CANONICAL_PACKAGE_METADATA_FILE
            ),
        );
    }
    if !metadata_path.exists() {
        return fail_pkg(
            "C109",
            format!(
                "manifest migration requires canonical package metadata `{}`",
                metadata_path.display()
            ),
        );
    }
    if !metadata_path.is_file() {
        return fail_pkg(
            "C109",
            format!(
                "canonical package metadata path `{}` exists but is not a file",
                metadata_path.display()
            ),
        );
    }

    let generated = {
        let _stage = timings.start(logger, "pkg_migrate_manifest_inputs");
        let roots = if lock_path.exists() {
            match load_root_inputs_for_update(lock_path.as_path()) {
                Ok(value) => value,
                Err(err) => return fail_pkg("C109", format!("manifest migration lock input: {err}")),
            }
        } else {
            let policy = AdvisoryPolicy::permissive();
            let lock = match load_lockfile_from_metadata_with_policy(
                metadata_path.as_path(),
                None,
                None,
                &policy,
            ) {
                Ok(value) => value,
                Err(err) => return fail_pkg(err.code(), err.to_string()),
            };
            lock.roots().to_vec()
        };
        match build_generated_manifest(root.as_path(), roots) {
            Ok(value) => value,
            Err(err) => return fail_pkg("C109", err),
        }
    };

    {
        let _stage = timings.start(logger, "pkg_migrate_manifest_write");
        write_generated_manifest(manifest_path.as_path(), &generated)?;
    }

    if emit_stdout_summary {
        println!(
            "wrote {} with {} dependency requirement(s)",
            manifest_path.display(),
            generated.dependencies.len()
        );
    }
    logger.summary(&timings);
    Ok(())
}

fn build_generated_manifest(
    root: &Path,
    mut roots: Vec<StrictLockRootV1>,
) -> Result<GeneratedManifestV1, String> {
    if roots.is_empty() {
        return Err("manifest migration could not infer dependency roots".to_string());
    }
    roots.sort_by(|a, b| a.name.cmp(&b.name));
    if roots.len() != 1 {
        let names = roots
            .iter()
            .map(|root| root.name.as_str())
            .collect::<Vec<_>>()
            .join(", ");
        return Err(format!(
            "manifest migration requires exactly one root for schema v1 project mapping; found {} roots: {}",
            roots.len(),
            names
        ));
    }
    let root_entry = roots.remove(0);

    let mut dependencies = root_entry
        .dependencies
        .into_iter()
        .map(|dep| GeneratedDependencyV1 {
            name: dep.name,
            requirement: dep.requirement,
        })
        .collect::<Vec<_>>();
    dependencies.sort_by(|a, b| {
        a.name
            .cmp(&b.name)
            .then_with(|| a.requirement.cmp(&b.requirement))
    });

    let inferred_name = infer_project_name(root, root_entry.name.as_str());
    Ok(GeneratedManifestV1 {
        schema_version: 1,
        project: GeneratedProjectMetadataV1 {
            name: inferred_name.clone(),
            description: format!("Migrated manifest for `{}`", inferred_name),
            version: "0.1.0".to_string(),
            clg_version: default_project_clg_version_requirement(),
            entry: "main.clear".to_string(),
            website: "https://example.com".to_string(),
            contact: GeneratedContactV1 {
                name: "Project Maintainer".to_string(),
                email: "maintainer@example.com".to_string(),
            },
        },
        dependencies,
        release_defaults: GeneratedReleaseDefaultsV0 {
            advisory_as_of: RELEASE_DEFAULT_ADVISORY_PLACEHOLDER.to_string(),
            key_id: RELEASE_DEFAULT_KEY_ID_PLACEHOLDER.to_string(),
            out_dir: "out/release".to_string(),
            trust_policy: "trust-policy.json".to_string(),
        },
    })
}

fn infer_project_name(root: &Path, root_name: &str) -> String {
    if root_name != "app" && !root_name.trim().is_empty() {
        return root_name.to_string();
    }
    root.file_name()
        .map(|name| name.to_string_lossy().trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "app".to_string())
}

fn write_generated_manifest(path: &Path, manifest: &GeneratedManifestV1) -> Result<()> {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent).with_context(|| format!("creating {}", parent.display()))?;
        }
    }
    let mut bytes =
        serde_json::to_vec_pretty(manifest).context("serializing generated project manifest")?;
    bytes.push(b'\n');
    fs::write(path, bytes).with_context(|| format!("writing {}", path.display()))?;
    Ok(())
}
