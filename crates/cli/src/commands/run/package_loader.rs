use std::collections::HashMap;
use std::fs;
use std::path::{Component, Path, PathBuf};

use serde::Deserialize;

use crate::commands::helpers::{canonical_json_bytes, sha256_hex};
use crate::commands::validation::{
    validate_exact_semver, validate_package_id, validate_sha256_digest,
};

const RUNTIME_LINK_FILE: &str = "clg.runtime-link.json";
const RUNTIME_LINK_HASH_FILE: &str = "clg.runtime-link.sha256";
const PACKAGE_STORE_INDEX_FILE: &str = "clg.package-store-index.json";

#[derive(Clone, Copy, Debug, Default)]
pub(super) struct RuntimeLoaderConfig {
    pub(super) allow_remote_fetch: bool,
}

#[derive(Clone, Debug)]
pub(super) struct LoadedRuntimePackage {
    pub(super) id: String,
    pub(super) digest: String,
    pub(super) abi_id: String,
    pub(super) resolved_path: PathBuf,
}

#[derive(Clone, Debug)]
pub(super) struct LoadedRuntimePackageSet {
    pub(super) resolver_version: u32,
    pub(super) packages: Vec<LoadedRuntimePackage>,
}

#[derive(Clone, Debug)]
pub(super) struct RuntimePackageLoaderError {
    code: &'static str,
    message: String,
}

impl RuntimePackageLoaderError {
    fn new(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }

    pub(super) fn code(&self) -> &'static str {
        self.code
    }

    pub(super) fn message(&self) -> &str {
        &self.message
    }
}

#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct RuntimeLinkRootV0 {
    schema_version: u32,
    resolver_version: u32,
    #[serde(default)]
    packages: Vec<RuntimeLinkPackageV0>,
    #[serde(default)]
    bindings: Vec<RuntimeLinkBindingV0>,
}

#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct RuntimeLinkPackageV0 {
    id: String,
    digest: String,
    artifact_path: String,
    abi_id: String,
}

#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct RuntimeLinkBindingV0 {
    import_module: String,
    import_name: String,
    provider_package_id: String,
    provider_symbol: String,
}

#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct PackageStoreIndexV0 {
    schema_version: u32,
    #[serde(default)]
    artifacts: Vec<PackageStoreArtifactV0>,
}

#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct PackageStoreArtifactV0 {
    id: String,
    digest: String,
    path: String,
}

pub(super) fn load_runtime_packages_from_local_store_if_present(
    root: &Path,
    config: RuntimeLoaderConfig,
) -> Result<Option<LoadedRuntimePackageSet>, RuntimePackageLoaderError> {
    if config.allow_remote_fetch {
        return Err(RuntimePackageLoaderError::new(
            "R012",
            "runtime loader remote fetch is disabled by default in phase 23.0.1; provide trusted local store/index inputs only",
        ));
    }

    let runtime_link_path = root.join(RUNTIME_LINK_FILE);
    if !runtime_link_path.exists() {
        return Ok(None);
    }
    if !runtime_link_path.is_file() {
        return Err(RuntimePackageLoaderError::new(
            "R012",
            format!(
                "runtime link artifact path `{}` exists but is not a file",
                runtime_link_path.display()
            ),
        ));
    }
    let runtime_link_hash_path = root.join(RUNTIME_LINK_HASH_FILE);
    if !runtime_link_hash_path.exists() {
        return Err(RuntimePackageLoaderError::new(
            "R017",
            format!(
                "runtime link hash companion `{}` is missing",
                runtime_link_hash_path.display()
            ),
        ));
    }
    if !runtime_link_hash_path.is_file() {
        return Err(RuntimePackageLoaderError::new(
            "R017",
            format!(
                "runtime link hash path `{}` exists but is not a file",
                runtime_link_hash_path.display()
            ),
        ));
    }

    let runtime_link_text = fs::read_to_string(&runtime_link_path).map_err(|err| {
        RuntimePackageLoaderError::new(
            "R012",
            format!("reading {}: {err}", runtime_link_path.display()),
        )
    })?;
    let runtime_link_value: serde_json::Value = serde_json::from_str(runtime_link_text.as_str())
        .map_err(|err| {
            RuntimePackageLoaderError::new(
                "R012",
                format!("parsing {}: {err}", runtime_link_path.display()),
            )
        })?;
    let runtime_link: RuntimeLinkRootV0 = serde_json::from_value(runtime_link_value.clone())
        .map_err(|err| {
            RuntimePackageLoaderError::new(
                "R012",
                format!(
                    "runtime link artifact `{}` does not match schema v0: {err}",
                    runtime_link_path.display()
                ),
            )
        })?;
    if runtime_link.schema_version != 0 {
        return Err(RuntimePackageLoaderError::new(
            "R012",
            format!(
                "runtime link artifact `{}` has unsupported schema_version {}; expected 0",
                runtime_link_path.display(),
                runtime_link.schema_version
            ),
        ));
    }

    enforce_runtime_link_deterministic_shape(&runtime_link).map_err(|msg| {
        RuntimePackageLoaderError::new(
            "R017",
            format!(
                "runtime link artifact `{}` violates deterministic ordering: {msg}",
                runtime_link_path.display()
            ),
        )
    })?;

    let canonical_link_hash = sha256_hex(canonical_json_bytes(&runtime_link_value).as_slice());
    let expected_hash = fs::read_to_string(&runtime_link_hash_path).map_err(|err| {
        RuntimePackageLoaderError::new(
            "R017",
            format!("reading {}: {err}", runtime_link_hash_path.display()),
        )
    })?;
    let expected_hash = expected_hash.trim();
    validate_runtime_link_hash(expected_hash).map_err(|msg| {
        RuntimePackageLoaderError::new(
            "R017",
            format!(
                "runtime link hash companion `{}` is invalid: {msg}",
                runtime_link_hash_path.display()
            ),
        )
    })?;
    if expected_hash != canonical_link_hash {
        return Err(RuntimePackageLoaderError::new(
            "R017",
            format!(
                "runtime link hash mismatch for `{}`: expected `{}`, got `{}`",
                runtime_link_path.display(),
                expected_hash,
                canonical_link_hash
            ),
        ));
    }

    let store_index = load_package_store_index(root)?;
    let mut store_by_id_digest: HashMap<(String, String), PackageStoreArtifactV0> =
        HashMap::with_capacity(store_index.artifacts.len());
    for artifact in &store_index.artifacts {
        validate_runtime_package_id(artifact.id.as_str()).map_err(|msg| {
            RuntimePackageLoaderError::new(
                "R012",
                format!("store artifact id `{}` is invalid: {msg}", artifact.id),
            )
        })?;
        validate_sha256_digest(artifact.digest.as_str()).map_err(|msg| {
            RuntimePackageLoaderError::new(
                "R013",
                format!(
                    "store artifact `{}` has invalid digest `{}`: {msg}",
                    artifact.id, artifact.digest
                ),
            )
        })?;
        validate_relative_artifact_path(artifact.path.as_str()).map_err(|msg| {
            RuntimePackageLoaderError::new(
                "R012",
                format!(
                    "store artifact `{}` has invalid path `{}`: {msg}",
                    artifact.id, artifact.path
                ),
            )
        })?;
        let key = (artifact.id.clone(), artifact.digest.clone());
        if store_by_id_digest.contains_key(&key) {
            return Err(RuntimePackageLoaderError::new(
                "R017",
                format!(
                    "store index `{}` contains duplicate artifact `(id={}, digest={})`",
                    PACKAGE_STORE_INDEX_FILE, artifact.id, artifact.digest
                ),
            ));
        }
        store_by_id_digest.insert(key, artifact.clone());
    }

    let mut loaded_packages = Vec::with_capacity(runtime_link.packages.len());
    for pkg in &runtime_link.packages {
        validate_runtime_package_id(pkg.id.as_str()).map_err(|msg| {
            RuntimePackageLoaderError::new(
                "R012",
                format!("runtime link package id `{}` is invalid: {msg}", pkg.id),
            )
        })?;
        validate_sha256_digest(pkg.digest.as_str()).map_err(|msg| {
            RuntimePackageLoaderError::new(
                "R013",
                format!(
                    "runtime link package `{}` has invalid digest `{}`: {msg}",
                    pkg.id, pkg.digest
                ),
            )
        })?;
        validate_relative_artifact_path(pkg.artifact_path.as_str()).map_err(|msg| {
            RuntimePackageLoaderError::new(
                "R012",
                format!(
                    "runtime link package `{}` has invalid artifact_path `{}`: {msg}",
                    pkg.id, pkg.artifact_path
                ),
            )
        })?;
        if pkg.abi_id.trim().is_empty() {
            return Err(RuntimePackageLoaderError::new(
                "R015",
                format!("runtime link package `{}` has empty abi_id", pkg.id),
            ));
        }

        let Some(store_artifact) = store_by_id_digest.get(&(pkg.id.clone(), pkg.digest.clone()))
        else {
            return Err(RuntimePackageLoaderError::new(
                "R012",
                format!(
                    "runtime package artifact `{}` with digest `{}` is missing from trusted local store/index",
                    pkg.id, pkg.digest
                ),
            ));
        };
        if store_artifact.path != pkg.artifact_path {
            return Err(RuntimePackageLoaderError::new(
                "R017",
                format!(
                    "runtime link package `{}` artifact path `{}` does not match store index path `{}`",
                    pkg.id, pkg.artifact_path, store_artifact.path
                ),
            ));
        }
        let resolved_path = root.join(store_artifact.path.as_str());
        if !resolved_path.exists() || !resolved_path.is_file() {
            return Err(RuntimePackageLoaderError::new(
                "R012",
                format!(
                    "runtime package artifact `{}` resolved to `{}` which is missing/unavailable",
                    pkg.id,
                    resolved_path.display()
                ),
            ));
        }
        loaded_packages.push(LoadedRuntimePackage {
            id: pkg.id.clone(),
            digest: pkg.digest.clone(),
            abi_id: pkg.abi_id.clone(),
            resolved_path,
        });
    }

    Ok(Some(LoadedRuntimePackageSet {
        resolver_version: runtime_link.resolver_version,
        packages: loaded_packages,
    }))
}

fn load_package_store_index(root: &Path) -> Result<PackageStoreIndexV0, RuntimePackageLoaderError> {
    let index_path = root.join(PACKAGE_STORE_INDEX_FILE);
    if !index_path.exists() {
        return Err(RuntimePackageLoaderError::new(
            "R012",
            format!(
                "trusted package store index `{}` is missing",
                index_path.display()
            ),
        ));
    }
    if !index_path.is_file() {
        return Err(RuntimePackageLoaderError::new(
            "R012",
            format!(
                "trusted package store index path `{}` exists but is not a file",
                index_path.display()
            ),
        ));
    }

    let text = fs::read_to_string(&index_path).map_err(|err| {
        RuntimePackageLoaderError::new("R012", format!("reading {}: {err}", index_path.display()))
    })?;
    let index: PackageStoreIndexV0 = serde_json::from_str(text.as_str()).map_err(|err| {
        RuntimePackageLoaderError::new("R012", format!("parsing {}: {err}", index_path.display()))
    })?;
    if index.schema_version != 0 {
        return Err(RuntimePackageLoaderError::new(
            "R012",
            format!(
                "trusted package store index `{}` has unsupported schema_version {}; expected 0",
                index_path.display(),
                index.schema_version
            ),
        ));
    }
    Ok(index)
}

fn validate_runtime_link_hash(value: &str) -> Result<(), String> {
    if value.len() != 64 {
        return Err("hash must have exactly 64 lowercase hex characters".to_string());
    }
    if !value
        .chars()
        .all(|ch| ch.is_ascii_digit() || ('a'..='f').contains(&ch))
    {
        return Err("hash must be lowercase hex (`0-9`, `a-f`)".to_string());
    }
    Ok(())
}

fn validate_runtime_package_id(value: &str) -> Result<(), String> {
    let Some((name, version)) = value.rsplit_once('@') else {
        return Err("expected `name@version` format".to_string());
    };
    validate_package_id(name)?;
    validate_exact_semver(version)
}

fn validate_relative_artifact_path(path: &str) -> Result<(), String> {
    if path.trim().is_empty() {
        return Err("artifact path is empty".to_string());
    }
    let path_ref = Path::new(path);
    if path_ref.is_absolute() {
        return Err("artifact path must be relative".to_string());
    }
    for component in path_ref.components() {
        match component {
            Component::ParentDir => {
                return Err(
                    "artifact path must not contain parent-directory traversal (`..`)".to_string(),
                )
            }
            Component::Prefix(_) | Component::RootDir => {
                return Err("artifact path must be relative".to_string())
            }
            Component::CurDir | Component::Normal(_) => {}
        }
    }
    Ok(())
}

fn enforce_runtime_link_deterministic_shape(link: &RuntimeLinkRootV0) -> Result<(), String> {
    let mut prev_pkg: Option<&str> = None;
    let mut seen_packages = std::collections::HashSet::with_capacity(link.packages.len());
    for pkg in &link.packages {
        if let Some(prev) = prev_pkg {
            if pkg.id.as_str() < prev {
                return Err(format!(
                    "packages must be sorted by id (found `{}` before `{}`)",
                    prev, pkg.id
                ));
            }
        }
        if !seen_packages.insert(pkg.id.clone()) {
            return Err(format!("duplicate package id `{}`", pkg.id));
        }
        prev_pkg = Some(pkg.id.as_str());
    }

    let mut prev_binding: Option<(&str, &str, &str, &str)> = None;
    let mut seen_bindings = std::collections::HashSet::with_capacity(link.bindings.len());
    for binding in &link.bindings {
        if binding.import_module.trim().is_empty()
            || binding.import_name.trim().is_empty()
            || binding.provider_package_id.trim().is_empty()
            || binding.provider_symbol.trim().is_empty()
        {
            return Err("binding fields must be non-empty".to_string());
        }
        let current = (
            binding.import_module.as_str(),
            binding.import_name.as_str(),
            binding.provider_package_id.as_str(),
            binding.provider_symbol.as_str(),
        );
        if let Some(prev) = prev_binding {
            if current < prev {
                return Err("bindings must be sorted by (import_module, import_name, provider_package_id, provider_symbol)".to_string());
            }
        }
        if !seen_bindings.insert(current) {
            return Err(format!(
                "duplicate binding ({}, {}, {}, {})",
                current.0, current.1, current.2, current.3
            ));
        }
        prev_binding = Some(current);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    fn write_file(path: &Path, text: &str) {
        fs::write(path, text).expect("write file");
    }

    #[test]
    fn returns_none_when_runtime_link_is_absent() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let result = load_runtime_packages_from_local_store_if_present(
            tmp.path(),
            RuntimeLoaderConfig::default(),
        )
        .expect("no runtime link should be accepted");
        assert!(result.is_none());
    }

    #[test]
    fn rejects_remote_fetch_in_phase_23_0_1() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let err = load_runtime_packages_from_local_store_if_present(
            tmp.path(),
            RuntimeLoaderConfig {
                allow_remote_fetch: true,
            },
        )
        .expect_err("remote fetch should be rejected");
        assert_eq!(err.code(), "R012");
    }

    #[test]
    fn loads_runtime_packages_from_local_store_index() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let root = tmp.path();
        let store_dir = root.join("store");
        fs::create_dir_all(&store_dir).expect("create store dir");
        write_file(
            store_dir.join("pkg-a.wasm").as_path(),
            "(module (func (export \"main\") (result i32) i32.const 0))",
        );
        let link_text = r#"{
  "schema_version": 0,
  "resolver_version": 1,
  "packages": [
    {
      "id": "pkg::a@1.0.0",
      "digest": "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
      "artifact_path": "store/pkg-a.wasm",
      "abi_id": "abi:pkg::a:1.0.0"
    }
  ],
  "bindings": []
}"#;
        write_file(root.join(RUNTIME_LINK_FILE).as_path(), link_text);
        let link_value: serde_json::Value = serde_json::from_str(link_text).expect("json");
        let hash = sha256_hex(canonical_json_bytes(&link_value).as_slice());
        write_file(
            root.join(RUNTIME_LINK_HASH_FILE).as_path(),
            format!("{hash}\n").as_str(),
        );
        write_file(
            root.join(PACKAGE_STORE_INDEX_FILE).as_path(),
            r#"{
  "schema_version": 0,
  "artifacts": [
    {
      "id": "pkg::a@1.0.0",
      "digest": "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
      "path": "store/pkg-a.wasm"
    }
  ]
}"#,
        );

        let loaded =
            load_runtime_packages_from_local_store_if_present(root, RuntimeLoaderConfig::default())
                .expect("load runtime packages")
                .expect("runtime-link file present");
        assert_eq!(loaded.resolver_version, 1);
        assert_eq!(loaded.packages.len(), 1);
        assert_eq!(loaded.packages[0].id, "pkg::a@1.0.0");
        assert_eq!(
            loaded.packages[0].resolved_path,
            root.join("store").join("pkg-a.wasm")
        );
    }

    #[test]
    fn rejects_runtime_link_hash_mismatch_with_r017() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let root = tmp.path();
        write_file(
            root.join(RUNTIME_LINK_FILE).as_path(),
            r#"{
  "schema_version": 0,
  "resolver_version": 1,
  "packages": [],
  "bindings": []
}"#,
        );
        write_file(root.join(RUNTIME_LINK_HASH_FILE).as_path(), &"a".repeat(64));
        write_file(
            root.join(PACKAGE_STORE_INDEX_FILE).as_path(),
            r#"{"schema_version":0,"artifacts":[]}"#,
        );
        let err =
            load_runtime_packages_from_local_store_if_present(root, RuntimeLoaderConfig::default())
                .expect_err("expected hash mismatch");
        assert_eq!(err.code(), "R017");
    }

    #[test]
    fn rejects_missing_artifact_in_store_with_r012() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let root = tmp.path();
        let link_text = r#"{
  "schema_version": 0,
  "resolver_version": 1,
  "packages": [
    {
      "id": "pkg::a@1.0.0",
      "digest": "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
      "artifact_path": "store/pkg-a.wasm",
      "abi_id": "abi:pkg::a:1.0.0"
    }
  ],
  "bindings": []
}"#;
        write_file(root.join(RUNTIME_LINK_FILE).as_path(), link_text);
        let link_value: serde_json::Value = serde_json::from_str(link_text).expect("json");
        let hash = sha256_hex(canonical_json_bytes(&link_value).as_slice());
        write_file(
            root.join(RUNTIME_LINK_HASH_FILE).as_path(),
            format!("{hash}\n").as_str(),
        );
        write_file(
            root.join(PACKAGE_STORE_INDEX_FILE).as_path(),
            r#"{"schema_version":0,"artifacts":[]}"#,
        );

        let err =
            load_runtime_packages_from_local_store_if_present(root, RuntimeLoaderConfig::default())
                .expect_err("expected missing artifact");
        assert_eq!(err.code(), "R012");
    }
}
