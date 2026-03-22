use std::collections::{BTreeSet, HashMap, HashSet};
use std::fs;
use std::path::{Component, Path, PathBuf};

use ed25519_dalek::{Signature, VerifyingKey};
use serde::Deserialize;

use crate::commands::build::strict_trust_policy::load_required_trust_policy_v0;
use crate::commands::helpers::{canonical_json_bytes, sha256_hex};
use crate::commands::validation::{
    parse_utc_timestamp_components, validate_exact_semver, validate_package_id,
    validate_semver_requirement, validate_sha256_digest,
};

const RUNTIME_LINK_FILE: &str = "clg.runtime-link.json";
const RUNTIME_LINK_HASH_FILE: &str = "clg.runtime-link.sha256";
const PACKAGE_STORE_INDEX_FILE: &str = "clg.package-store-index.json";
const RUNTIME_LOADER_POLICY_FILE: &str = "clg.runtime-loader.json";
const HOST_PROFILE_FILE: &str = "clg.host-profile.json";
const STRICT_LOCKFILE_FILE: &str = "clg.lock.json";
const STRICT_PACKAGE_SIGNATURES_FILE: &str = "clg.package-signatures.json";

#[derive(Clone, Debug, Default)]
pub(super) struct RuntimeLoaderConfig {
    pub(super) allow_remote_fetch: bool,
    pub(super) required_host_capabilities: Vec<String>,
}

#[derive(Clone, Debug)]
pub(super) struct RuntimeProductionHostProfile {
    pub(super) profile: String,
    pub(super) capabilities: Vec<String>,
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
    pub(super) bindings: Vec<LoadedRuntimeBinding>,
}

#[derive(Clone, Debug)]
pub(super) struct LoadedRuntimeBinding {
    pub(super) import_module: String,
    pub(super) import_name: String,
    pub(super) provider_package_id: String,
    pub(super) provider_symbol: String,
}

#[derive(Clone, Debug)]
pub(super) struct RuntimePackageLoaderError {
    code: &'static str,
    message: String,
}

impl RuntimePackageLoaderError {
    pub(super) fn new(code: &'static str, message: impl Into<String>) -> Self {
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

#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawStrictLockfileV0 {
    schema_version: u32,
    dependencies: Vec<RawStrictDependency>,
}

#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawStrictDependency {
    name: String,
    version: String,
    digest: String,
}

#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawStrictLockfileV1 {
    schema_version: u32,
    resolver_version: u32,
    roots: Vec<RawStrictRootV1>,
    packages: Vec<RawStrictPackageV1>,
}

#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawStrictRootV1 {
    name: String,
    dependencies: Vec<RawStrictRootDependencyV1>,
}

#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawStrictRootDependencyV1 {
    name: String,
    requirement: String,
}

#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawStrictPackageV1 {
    id: String,
    name: String,
    version: String,
    digest: String,
    abi_id: String,
    dependencies: Vec<String>,
}

#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawSignatureRootV0 {
    schema_version: u32,
    signatures: Vec<RawSignatureEntryV0>,
}

#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawSignatureEntryV0 {
    name: String,
    version: String,
    digest: String,
    key_id: String,
    signed_at: String,
    signature_format: String,
    signature: String,
}

#[derive(Clone, Debug)]
struct RuntimePackageSignatureEntry {
    digest: String,
    key_id: String,
    signed_at: String,
    signature: String,
}

#[derive(Clone, Debug)]
struct RuntimeLockfileEvidence {
    resolver_version: Option<u32>,
    digests_by_id: HashMap<String, String>,
}

#[derive(Clone, Debug)]
struct RuntimeAvailabilityPolicy {
    mirror_roots: Vec<PathBuf>,
    artifact_read_retries: u32,
}

impl Default for RuntimeAvailabilityPolicy {
    fn default() -> Self {
        Self {
            mirror_roots: Vec::new(),
            artifact_read_retries: 1,
        }
    }
}

#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawRuntimeLoaderPolicyV0 {
    schema_version: u32,
    #[serde(default = "default_runtime_loader_offline_mode")]
    offline_mode: bool,
    #[serde(default)]
    mirror_paths: Vec<String>,
    #[serde(default = "default_artifact_read_retries")]
    artifact_read_retries: u32,
}

#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawHostProfileV0 {
    schema_version: u32,
    profile: String,
    capabilities: Vec<String>,
}

fn default_runtime_loader_offline_mode() -> bool {
    true
}

fn default_artifact_read_retries() -> u32 {
    1
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

    let production_profile = load_runtime_production_profile_if_present(root)?;
    let runtime_link_path = root.join(RUNTIME_LINK_FILE);
    if !runtime_link_path.exists() {
        if let Some(profile) = production_profile.as_ref() {
            return Err(RuntimePackageLoaderError::new(
                "R012",
                format!(
                    "runtime loader fail-closed: production host profile `{profile}` requires `{}` to enable mandatory runtime trust checks",
                    RUNTIME_LINK_FILE,
                    profile = profile.profile
                ),
            ));
        }
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

    validate_required_runtime_host_capabilities(
        production_profile.as_ref(),
        config.required_host_capabilities.as_slice(),
    )?;

    let lockfile = load_runtime_lockfile_evidence(root)?;
    if let Some(expected_resolver_version) = lockfile.resolver_version {
        if expected_resolver_version != runtime_link.resolver_version {
            return Err(RuntimePackageLoaderError::new(
                "R013",
                format!(
                    "runtime-link resolver_version `{}` does not match lockfile resolver_version `{}`",
                    runtime_link.resolver_version, expected_resolver_version
                ),
            ));
        }
    }
    let signatures = load_runtime_package_signatures(root)?;
    let trust_policy = load_required_trust_policy_v0(root).map_err(|err| {
        RuntimePackageLoaderError::new(
            "R014",
            format!(
                "loading trust policy for runtime package verification failed: {}",
                err.message()
            ),
        )
    })?;
    let availability_policy = load_runtime_availability_policy(root)?;
    let trust_signers: HashMap<
        &str,
        &crate::commands::build::strict_trust_policy::TrustedSignerV0,
    > = trust_policy
        .trusted_signers
        .iter()
        .map(|signer| (signer.key_id.as_str(), signer))
        .collect();
    let revoked: std::collections::HashSet<&str> = trust_policy
        .revoked_key_ids
        .iter()
        .map(|key_id| key_id.as_str())
        .collect();

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

    let runtime_link_package_ids: std::collections::HashSet<&str> = runtime_link
        .packages
        .iter()
        .map(|pkg| pkg.id.as_str())
        .collect();
    for binding in &runtime_link.bindings {
        if !runtime_link_package_ids.contains(binding.provider_package_id.as_str()) {
            return Err(RuntimePackageLoaderError::new(
                "R015",
                format!(
                    "runtime link binding `{}`::`{}` references unknown provider_package_id `{}`",
                    binding.import_module, binding.import_name, binding.provider_package_id
                ),
            ));
        }
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
        match lockfile.digests_by_id.get(pkg.id.as_str()) {
            Some(locked_digest) if locked_digest == &pkg.digest => {}
            Some(locked_digest) => {
                return Err(RuntimePackageLoaderError::new(
                    "R013",
                    format!(
                        "runtime link package `{}` digest `{}` does not match lockfile digest `{}`",
                        pkg.id, pkg.digest, locked_digest
                    ),
                ));
            }
            None => {
                return Err(RuntimePackageLoaderError::new(
                    "R013",
                    format!(
                        "runtime link package `{}` is not pinned in `{}`",
                        pkg.id, STRICT_LOCKFILE_FILE
                    ),
                ));
            }
        }
        let signature = signatures.get(pkg.id.as_str()).ok_or_else(|| {
            RuntimePackageLoaderError::new(
                "R014",
                format!(
                    "runtime trust gate failed: package `{}` is missing signature entry in `{}`",
                    pkg.id, STRICT_PACKAGE_SIGNATURES_FILE
                ),
            )
        })?;
        if signature.digest != pkg.digest {
            return Err(RuntimePackageLoaderError::new(
                "R014",
                format!(
                    "runtime trust gate failed for `{}`: signature digest `{}` does not match runtime-link digest `{}`",
                    pkg.id, signature.digest, pkg.digest
                ),
            ));
        }
        let signer = trust_signers
            .get(signature.key_id.as_str())
            .copied()
            .ok_or_else(|| {
                RuntimePackageLoaderError::new(
                    "R014",
                    format!(
                        "runtime trust gate failed for `{}`: signer `{}` is not trusted",
                        pkg.id, signature.key_id
                    ),
                )
            })?;
        if revoked.contains(signature.key_id.as_str()) {
            return Err(RuntimePackageLoaderError::new(
                "R014",
                format!(
                    "runtime trust gate failed for `{}`: signer `{}` is revoked",
                    pkg.id, signature.key_id
                ),
            ));
        }
        let signed_at =
            parse_utc_timestamp_components(signature.signed_at.as_str()).map_err(|msg| {
                RuntimePackageLoaderError::new(
                    "R014",
                    format!(
                        "runtime trust gate failed for `{}`: signed_at `{}` is invalid: {msg}",
                        pkg.id, signature.signed_at
                    ),
                )
            })?;
        let signer_not_before =
            parse_utc_timestamp_components(signer.not_before.as_str()).map_err(|msg| {
                RuntimePackageLoaderError::new(
                    "R014",
                    format!(
                        "runtime trust gate failed for `{}`: signer `{}` not_before is invalid: {msg}",
                        pkg.id, signer.key_id
                    ),
                )
            })?;
        let signer_not_after =
            parse_utc_timestamp_components(signer.not_after.as_str()).map_err(|msg| {
                RuntimePackageLoaderError::new(
                    "R014",
                    format!(
                        "runtime trust gate failed for `{}`: signer `{}` not_after is invalid: {msg}",
                        pkg.id, signer.key_id
                    ),
                )
            })?;
        if signed_at < signer_not_before || signed_at >= signer_not_after {
            return Err(RuntimePackageLoaderError::new(
                "R014",
                format!(
                    "runtime trust gate failed for `{}`: signer `{}` is not valid at signed_at `{}`",
                    pkg.id, signer.key_id, signature.signed_at
                ),
            ));
        }
        let verifying =
            verifying_key_from_hex_public_key(signer.public_key.as_str()).map_err(|msg| {
                RuntimePackageLoaderError::new(
                    "R014",
                    format!(
                    "runtime trust gate failed for `{}`: signer `{}` public key is invalid: {msg}",
                    pkg.id, signer.key_id
                ),
                )
            })?;
        let signature_bytes = decode_signature(signature.signature.as_str()).map_err(|msg| {
            RuntimePackageLoaderError::new(
                "R014",
                format!(
                    "runtime trust gate failed for `{}`: signature is invalid: {msg}",
                    pkg.id
                ),
            )
        })?;
        let (name, version) = split_runtime_package_id(pkg.id.as_str()).map_err(|msg| {
            RuntimePackageLoaderError::new(
                "R014",
                format!("runtime trust gate failed for `{}`: {msg}", pkg.id),
            )
        })?;
        let payload = canonical_signature_payload_v0(
            name,
            version,
            signature.digest.as_str(),
            signature.signed_at.as_str(),
        );
        verifying
            .verify_strict(payload.as_bytes(), &signature_bytes)
            .map_err(|_| {
                RuntimePackageLoaderError::new(
                    "R014",
                    format!(
                        "runtime trust gate failed for `{}`: signature verification failed for signer `{}`",
                        pkg.id, signer.key_id
                    ),
                )
            })?;

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
        let resolved_path = resolve_runtime_artifact_with_availability_policy(
            root,
            pkg.id.as_str(),
            pkg.digest.as_str(),
            store_artifact.path.as_str(),
            &availability_policy,
        )?;
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
        bindings: runtime_link
            .bindings
            .iter()
            .map(|binding| LoadedRuntimeBinding {
                import_module: binding.import_module.clone(),
                import_name: binding.import_name.clone(),
                provider_package_id: binding.provider_package_id.clone(),
                provider_symbol: binding.provider_symbol.clone(),
            })
            .collect(),
    }))
}

fn load_runtime_lockfile_evidence(
    root: &Path,
) -> Result<RuntimeLockfileEvidence, RuntimePackageLoaderError> {
    let path = root.join(STRICT_LOCKFILE_FILE);
    if !path.exists() {
        return Err(RuntimePackageLoaderError::new(
            "R013",
            format!(
                "runtime trust gate requires `{}` at `{}`",
                STRICT_LOCKFILE_FILE,
                path.display()
            ),
        ));
    }
    if !path.is_file() {
        return Err(RuntimePackageLoaderError::new(
            "R013",
            format!(
                "runtime lockfile path `{}` exists but is not a file",
                path.display()
            ),
        ));
    }
    let content = fs::read_to_string(&path).map_err(|err| {
        RuntimePackageLoaderError::new(
            "R013",
            format!(
                "reading runtime lockfile `{}` failed: {err}",
                path.display()
            ),
        )
    })?;
    let value: serde_json::Value = serde_json::from_str(content.as_str()).map_err(|err| {
        RuntimePackageLoaderError::new(
            "R013",
            format!(
                "runtime lockfile `{}` is not valid JSON (expected schema v0/v1): {err}",
                path.display()
            ),
        )
    })?;
    let schema_version = value
        .get("schema_version")
        .and_then(|v| v.as_u64())
        .ok_or_else(|| {
            RuntimePackageLoaderError::new(
                "R013",
                format!(
                    "runtime lockfile `{}` is missing required `schema_version`",
                    path.display()
                ),
            )
        })?;

    match schema_version {
        0 => {
            let raw: RawStrictLockfileV0 = serde_json::from_value(value).map_err(|err| {
                RuntimePackageLoaderError::new(
                    "R013",
                    format!(
                        "runtime lockfile `{}` does not match schema v0 (`dependencies[]`): {err}",
                        path.display()
                    ),
                )
            })?;
            debug_assert_eq!(raw.schema_version, 0);
            let mut digests_by_id = HashMap::with_capacity(raw.dependencies.len());
            for dep in raw.dependencies {
                validate_package_id(dep.name.as_str()).map_err(|msg| {
                    RuntimePackageLoaderError::new(
                        "R013",
                        format!(
                            "runtime lockfile `{}` has invalid dependency name `{}`: {msg}",
                            path.display(),
                            dep.name
                        ),
                    )
                })?;
                validate_exact_semver(dep.version.as_str()).map_err(|msg| {
                    RuntimePackageLoaderError::new(
                        "R013",
                        format!(
                            "runtime lockfile `{}` has invalid dependency version `{}` for `{}`: {msg}",
                            path.display(),
                            dep.version,
                            dep.name
                        ),
                    )
                })?;
                validate_sha256_digest(dep.digest.as_str()).map_err(|msg| {
                    RuntimePackageLoaderError::new(
                        "R013",
                        format!(
                            "runtime lockfile `{}` has invalid dependency digest `{}` for `{}`: {msg}",
                            path.display(),
                            dep.digest,
                            dep.name
                        ),
                    )
                })?;
                let id = format!("{}@{}", dep.name, dep.version);
                if let Some(existing) = digests_by_id.insert(id.clone(), dep.digest.clone()) {
                    return Err(RuntimePackageLoaderError::new(
                        "R013",
                        format!(
                            "runtime lockfile `{}` has duplicate package id `{}` (digests: `{}` vs `{}`)",
                            path.display(),
                            id,
                            existing,
                            dep.digest
                        ),
                    ));
                }
            }
            Ok(RuntimeLockfileEvidence {
                resolver_version: None,
                digests_by_id,
            })
        }
        1 => {
            let raw: RawStrictLockfileV1 = serde_json::from_value(value).map_err(|err| {
                RuntimePackageLoaderError::new(
                    "R013",
                    format!(
                        "runtime lockfile `{}` does not match schema v1 (`resolver_version`, `roots[]`, `packages[]`): {err}",
                        path.display()
                    ),
                )
            })?;
            debug_assert_eq!(raw.schema_version, 1);
            if raw.resolver_version != 1 {
                return Err(RuntimePackageLoaderError::new(
                    "R013",
                    format!(
                        "runtime lockfile `{}` has unsupported resolver_version {}; expected 1",
                        path.display(),
                        raw.resolver_version
                    ),
                ));
            }
            for root in &raw.roots {
                if root.name.trim().is_empty() {
                    return Err(RuntimePackageLoaderError::new(
                        "R013",
                        format!(
                            "runtime lockfile `{}` has root with empty name",
                            path.display()
                        ),
                    ));
                }
                for dep in &root.dependencies {
                    validate_package_id(dep.name.as_str()).map_err(|msg| {
                        RuntimePackageLoaderError::new(
                            "R013",
                            format!(
                                "runtime lockfile `{}` root `{}` has invalid dependency name `{}`: {msg}",
                                path.display(),
                                root.name,
                                dep.name
                            ),
                        )
                    })?;
                    validate_semver_requirement(dep.requirement.as_str()).map_err(|msg| {
                        RuntimePackageLoaderError::new(
                            "R013",
                            format!(
                                "runtime lockfile `{}` root `{}` dependency `{}` has invalid requirement `{}`: {msg}",
                                path.display(),
                                root.name,
                                dep.name,
                                dep.requirement
                            ),
                        )
                    })?;
                }
            }

            let mut digests_by_id = HashMap::with_capacity(raw.packages.len());
            for pkg in raw.packages {
                validate_package_id(pkg.name.as_str()).map_err(|msg| {
                    RuntimePackageLoaderError::new(
                        "R013",
                        format!(
                            "runtime lockfile `{}` has invalid package name `{}`: {msg}",
                            path.display(),
                            pkg.name
                        ),
                    )
                })?;
                validate_exact_semver(pkg.version.as_str()).map_err(|msg| {
                    RuntimePackageLoaderError::new(
                        "R013",
                        format!(
                            "runtime lockfile `{}` has invalid package version `{}` for `{}`: {msg}",
                            path.display(),
                            pkg.version,
                            pkg.name
                        ),
                    )
                })?;
                validate_sha256_digest(pkg.digest.as_str()).map_err(|msg| {
                    RuntimePackageLoaderError::new(
                        "R013",
                        format!(
                            "runtime lockfile `{}` has invalid package digest `{}` for `{}`: {msg}",
                            path.display(),
                            pkg.digest,
                            pkg.name
                        ),
                    )
                })?;
                if pkg.abi_id.trim().is_empty() {
                    return Err(RuntimePackageLoaderError::new(
                        "R013",
                        format!(
                            "runtime lockfile `{}` package `{}` has empty abi_id",
                            path.display(),
                            pkg.id
                        ),
                    ));
                }
                let expected_id = format!("{}@{}", pkg.name, pkg.version);
                if pkg.id != expected_id {
                    return Err(RuntimePackageLoaderError::new(
                        "R013",
                        format!(
                            "runtime lockfile `{}` package id `{}` does not match `name@version` (`{}`)",
                            path.display(),
                            pkg.id,
                            expected_id
                        ),
                    ));
                }
                for dep in &pkg.dependencies {
                    validate_runtime_package_id(dep.as_str()).map_err(|msg| {
                        RuntimePackageLoaderError::new(
                            "R013",
                            format!(
                                "runtime lockfile `{}` package `{}` has invalid dependency id `{}`: {msg}",
                                path.display(),
                                pkg.id,
                                dep
                            ),
                        )
                    })?;
                }
                if let Some(existing) = digests_by_id.insert(pkg.id.clone(), pkg.digest.clone()) {
                    return Err(RuntimePackageLoaderError::new(
                        "R013",
                        format!(
                            "runtime lockfile `{}` has duplicate package id `{}` (digests: `{}` vs `{}`)",
                            path.display(),
                            pkg.id,
                            existing,
                            pkg.digest
                        ),
                    ));
                }
            }
            Ok(RuntimeLockfileEvidence {
                resolver_version: Some(raw.resolver_version),
                digests_by_id,
            })
        }
        other => Err(RuntimePackageLoaderError::new(
            "R013",
            format!(
                "runtime lockfile `{}` has unsupported schema_version {}; expected 0 or 1",
                path.display(),
                other
            ),
        )),
    }
}

fn load_runtime_package_signatures(
    root: &Path,
) -> Result<HashMap<String, RuntimePackageSignatureEntry>, RuntimePackageLoaderError> {
    let path = root.join(STRICT_PACKAGE_SIGNATURES_FILE);
    if !path.exists() {
        return Err(RuntimePackageLoaderError::new(
            "R014",
            format!(
                "runtime trust gate requires `{}` at `{}`",
                STRICT_PACKAGE_SIGNATURES_FILE,
                path.display()
            ),
        ));
    }
    if !path.is_file() {
        return Err(RuntimePackageLoaderError::new(
            "R014",
            format!(
                "runtime package signatures path `{}` exists but is not a file",
                path.display()
            ),
        ));
    }
    let content = fs::read_to_string(&path).map_err(|err| {
        RuntimePackageLoaderError::new(
            "R014",
            format!(
                "reading runtime package signatures `{}` failed: {err}",
                path.display()
            ),
        )
    })?;
    let raw: RawSignatureRootV0 = serde_json::from_str(content.as_str()).map_err(|err| {
        RuntimePackageLoaderError::new(
            "R014",
            format!(
                "runtime package signatures `{}` do not match schema v0 (`schema_version`, `signatures[]`): {err}",
                path.display()
            ),
        )
    })?;
    if raw.schema_version != 0 {
        return Err(RuntimePackageLoaderError::new(
            "R014",
            format!(
                "runtime package signatures `{}` have unsupported schema_version {}; expected 0",
                path.display(),
                raw.schema_version
            ),
        ));
    }
    let mut entries = HashMap::with_capacity(raw.signatures.len());
    for entry in raw.signatures {
        validate_package_id(entry.name.as_str()).map_err(|msg| {
            RuntimePackageLoaderError::new(
                "R014",
                format!(
                    "runtime package signatures `{}` package `{}` is invalid: {msg}",
                    path.display(),
                    entry.name
                ),
            )
        })?;
        validate_exact_semver(entry.version.as_str()).map_err(|msg| {
            RuntimePackageLoaderError::new(
                "R014",
                format!(
                    "runtime package signatures `{}` package `{}` has invalid version `{}`: {msg}",
                    path.display(),
                    entry.name,
                    entry.version
                ),
            )
        })?;
        validate_sha256_digest(entry.digest.as_str()).map_err(|msg| {
            RuntimePackageLoaderError::new(
                "R014",
                format!(
                    "runtime package signatures `{}` package `{}` has invalid digest `{}`: {msg}",
                    path.display(),
                    entry.name,
                    entry.digest
                ),
            )
        })?;
        if entry.key_id.trim().is_empty() {
            return Err(RuntimePackageLoaderError::new(
                "R014",
                format!(
                    "runtime package signatures `{}` package `{}` has empty key_id",
                    path.display(),
                    entry.name
                ),
            ));
        }
        parse_utc_timestamp_components(entry.signed_at.as_str()).map_err(|msg| {
            RuntimePackageLoaderError::new(
                "R014",
                format!(
                    "runtime package signatures `{}` package `{}` has invalid signed_at `{}`: {msg}",
                    path.display(),
                    entry.name,
                    entry.signed_at
                ),
            )
        })?;
        if entry.signature_format != "ed25519" {
            return Err(RuntimePackageLoaderError::new(
                "R014",
                format!(
                    "runtime package signatures `{}` package `{}` has unsupported signature_format `{}`; expected `ed25519`",
                    path.display(),
                    entry.name,
                    entry.signature_format
                ),
            ));
        }
        decode_signature(entry.signature.as_str()).map_err(|msg| {
            RuntimePackageLoaderError::new(
                "R014",
                format!(
                    "runtime package signatures `{}` package `{}` has invalid signature: {msg}",
                    path.display(),
                    entry.name
                ),
            )
        })?;
        let id = format!("{}@{}", entry.name, entry.version);
        let value = RuntimePackageSignatureEntry {
            digest: entry.digest,
            key_id: entry.key_id,
            signed_at: entry.signed_at,
            signature: entry.signature,
        };
        if entries.insert(id.clone(), value).is_some() {
            return Err(RuntimePackageLoaderError::new(
                "R014",
                format!(
                    "runtime package signatures `{}` have duplicate package id `{}`",
                    path.display(),
                    id
                ),
            ));
        }
    }
    Ok(entries)
}

fn load_runtime_availability_policy(
    root: &Path,
) -> Result<RuntimeAvailabilityPolicy, RuntimePackageLoaderError> {
    let path = root.join(RUNTIME_LOADER_POLICY_FILE);
    if !path.exists() {
        return Ok(RuntimeAvailabilityPolicy::default());
    }
    if !path.is_file() {
        return Err(RuntimePackageLoaderError::new(
            "R012",
            format!(
                "runtime loader policy path `{}` exists but is not a file",
                path.display()
            ),
        ));
    }
    let content = fs::read_to_string(&path).map_err(|err| {
        RuntimePackageLoaderError::new(
            "R012",
            format!(
                "reading runtime loader policy `{}` failed: {err}",
                path.display()
            ),
        )
    })?;
    let raw: RawRuntimeLoaderPolicyV0 = serde_json::from_str(content.as_str()).map_err(|err| {
        RuntimePackageLoaderError::new(
            "R012",
            format!(
                "runtime loader policy `{}` does not match schema v0 (`schema_version`, `offline_mode`, `mirror_paths`, `artifact_read_retries`): {err}",
                path.display()
            ),
        )
    })?;
    if raw.schema_version != 0 {
        return Err(RuntimePackageLoaderError::new(
            "R012",
            format!(
                "runtime loader policy `{}` has unsupported schema_version {}; expected 0",
                path.display(),
                raw.schema_version
            ),
        ));
    }
    if !raw.offline_mode {
        return Err(RuntimePackageLoaderError::new(
            "R012",
            format!(
                "runtime loader policy `{}` sets `offline_mode=false`, but remote fetching is not supported in phase 23.0.4",
                path.display()
            ),
        ));
    }
    if raw.artifact_read_retries == 0 {
        return Err(RuntimePackageLoaderError::new(
            "R012",
            format!(
                "runtime loader policy `{}` must set `artifact_read_retries` to >= 1",
                path.display()
            ),
        ));
    }
    if raw.artifact_read_retries > 8 {
        return Err(RuntimePackageLoaderError::new(
            "R012",
            format!(
                "runtime loader policy `{}` must keep `artifact_read_retries` <= 8 for deterministic runtime bounds",
                path.display()
            ),
        ));
    }

    let mut seen_paths = std::collections::HashSet::with_capacity(raw.mirror_paths.len());
    let mut mirror_roots = Vec::with_capacity(raw.mirror_paths.len());
    for value in raw.mirror_paths {
        validate_relative_artifact_path(value.as_str()).map_err(|msg| {
            RuntimePackageLoaderError::new(
                "R012",
                format!(
                    "runtime loader policy `{}` has invalid mirror path `{}`: {msg}",
                    path.display(),
                    value
                ),
            )
        })?;
        if !seen_paths.insert(value.clone()) {
            return Err(RuntimePackageLoaderError::new(
                "R012",
                format!(
                    "runtime loader policy `{}` has duplicate mirror path `{}`",
                    path.display(),
                    value
                ),
            ));
        }
        mirror_roots.push(PathBuf::from(value));
    }

    Ok(RuntimeAvailabilityPolicy {
        mirror_roots,
        artifact_read_retries: raw.artifact_read_retries,
    })
}

fn load_runtime_production_profile_if_present(
    root: &Path,
) -> Result<Option<RuntimeProductionHostProfile>, RuntimePackageLoaderError> {
    let path = root.join(HOST_PROFILE_FILE);
    if !path.exists() {
        return Ok(None);
    }
    if !path.is_file() {
        return Err(RuntimePackageLoaderError::new(
            "R016",
            format!(
                "runtime host profile path `{}` exists but is not a file",
                path.display()
            ),
        ));
    }
    let content = fs::read_to_string(&path).map_err(|err| {
        RuntimePackageLoaderError::new(
            "R016",
            format!(
                "reading runtime host profile `{}` failed: {err}",
                path.display()
            ),
        )
    })?;
    let raw: RawHostProfileV0 = serde_json::from_str(content.as_str()).map_err(|err| {
        RuntimePackageLoaderError::new(
            "R016",
            format!(
                "runtime host profile `{}` does not match schema v0 (`schema_version`, `profile`, `capabilities`): {err}",
                path.display()
            ),
        )
    })?;
    if raw.schema_version != 0 {
        return Err(RuntimePackageLoaderError::new(
            "R016",
            format!(
                "runtime host profile `{}` has unsupported schema_version {}; expected 0",
                path.display(),
                raw.schema_version
            ),
        ));
    }
    match raw.profile.as_str() {
        "contract_static" | "shared_app" => {
            let capabilities =
                validate_runtime_host_profile_capabilities(path.as_path(), raw.capabilities.as_slice())?;
            Ok(Some(RuntimeProductionHostProfile {
                profile: raw.profile,
                capabilities,
            }))
        }
        _ => Err(RuntimePackageLoaderError::new(
            "R016",
            format!(
                "runtime host profile `{}` has unsupported profile `{}`; expected `contract_static` or `shared_app`",
                path.display(),
                raw.profile
            ),
        )),
    }
}

fn validate_runtime_host_profile_capabilities(
    path: &Path,
    capabilities: &[String],
) -> Result<Vec<String>, RuntimePackageLoaderError> {
    let mut sorted = capabilities.to_vec();
    sorted.sort();

    let mut seen = HashSet::with_capacity(sorted.len());
    let mut duplicates = BTreeSet::new();
    for capability in &sorted {
        if !seen.insert(capability.clone()) {
            duplicates.insert(capability.clone());
        }
    }
    if let Some(duplicate) = duplicates.iter().next() {
        return Err(RuntimePackageLoaderError::new(
            "R016",
            format!(
                "runtime host profile `{}` has duplicate capability `{}`",
                path.display(),
                duplicate
            ),
        ));
    }

    for capability in &sorted {
        if capability.trim().is_empty() {
            return Err(RuntimePackageLoaderError::new(
                "R016",
                format!(
                    "runtime host profile `{}` contains an empty capability id",
                    path.display()
                ),
            ));
        }
        if !is_allowed_runtime_host_capability(capability.as_str()) {
            return Err(RuntimePackageLoaderError::new(
                "R016",
                format!(
                    "runtime host profile `{}` has unsupported capability `{}`",
                    path.display(),
                    capability
                ),
            ));
        }
    }
    Ok(sorted)
}

fn validate_required_runtime_host_capabilities(
    profile: Option<&RuntimeProductionHostProfile>,
    required_capabilities: &[String],
) -> Result<(), RuntimePackageLoaderError> {
    let Some(profile) = profile else {
        return Ok(());
    };

    let configured: HashSet<&str> = profile.capabilities.iter().map(String::as_str).collect();
    let mut missing = BTreeSet::new();
    for required in required_capabilities {
        if !configured.contains(required.as_str()) {
            missing.insert(required.clone());
        }
    }
    if missing.is_empty() {
        return Ok(());
    }
    let missing_list = missing.iter().cloned().collect::<Vec<_>>().join(", ");
    Err(RuntimePackageLoaderError::new(
        "R016",
        format!(
            "runtime host profile `{}` is missing required capabilities for active module imports: {}",
            profile.profile, missing_list
        ),
    ))
}

fn is_allowed_runtime_host_capability(capability: &str) -> bool {
    matches!(
        capability,
        "std::crypto::hash"
            | "std::crypto::hmac"
            | "std::crypto::verify"
            | "std::env::time"
            | "std::env::random"
            | "std::env::chain_id"
            | "std::wasi::print"
    )
}

fn resolve_runtime_artifact_with_availability_policy(
    root: &Path,
    package_id: &str,
    expected_digest: &str,
    artifact_path: &str,
    policy: &RuntimeAvailabilityPolicy,
) -> Result<PathBuf, RuntimePackageLoaderError> {
    let mut candidates = Vec::with_capacity(1 + policy.mirror_roots.len());
    candidates.push(root.join(artifact_path));
    for mirror_root in &policy.mirror_roots {
        candidates.push(root.join(mirror_root).join(artifact_path));
    }

    let mut saw_digest_mismatch = false;
    let mut available_digest_mismatches = Vec::new();
    for candidate in &candidates {
        for _ in 0..policy.artifact_read_retries {
            if !candidate.exists() || !candidate.is_file() {
                continue;
            }
            let bytes = match fs::read(candidate) {
                Ok(value) => value,
                Err(_) => continue,
            };
            let actual_digest = format!("sha256:{}", sha256_hex(bytes.as_slice()));
            if actual_digest != expected_digest {
                saw_digest_mismatch = true;
                available_digest_mismatches.push(format!(
                    "{}=>{}",
                    candidate.display(),
                    actual_digest
                ));
                break;
            }
            return Ok(candidate.clone());
        }
    }

    if saw_digest_mismatch {
        let details = available_digest_mismatches.join(", ");
        return Err(RuntimePackageLoaderError::new(
            "R013",
            format!(
                "runtime package artifact `{}` digest mismatch across configured availability roots; expected `{}` ({})",
                package_id, expected_digest, details
            ),
        ));
    }

    let attempted = candidates
        .iter()
        .map(|path| path.display().to_string())
        .collect::<Vec<_>>()
        .join(", ");
    Err(RuntimePackageLoaderError::new(
        "R012",
        format!(
            "runtime package artifact `{}` was unavailable at all configured roots: {}",
            package_id, attempted
        ),
    ))
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

fn split_runtime_package_id(value: &str) -> Result<(&str, &str), String> {
    let Some((name, version)) = value.rsplit_once('@') else {
        return Err("expected `name@version` format".to_string());
    };
    Ok((name, version))
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

fn verifying_key_from_hex_public_key(value: &str) -> Result<VerifyingKey, &'static str> {
    const PREFIX: &str = "hex:";
    if !value.starts_with(PREFIX) {
        return Err("public_key must start with `hex:`");
    }
    let key_hex = &value[PREFIX.len()..];
    let key_raw = hex::decode(key_hex).map_err(|_| "public_key is not valid hex")?;
    let key_bytes: [u8; 32] = key_raw
        .try_into()
        .map_err(|_| "public_key must be exactly 32 bytes")?;
    VerifyingKey::from_bytes(&key_bytes).map_err(|_| "public_key bytes are invalid")
}

fn decode_signature(value: &str) -> Result<Signature, &'static str> {
    let raw = hex::decode(value).map_err(|_| "signature is not valid hex")?;
    Signature::try_from(raw.as_slice()).map_err(|_| "signature must be exactly 64 bytes")
}

fn canonical_signature_payload_v0(
    name: &str,
    version: &str,
    digest: &str,
    signed_at: &str,
) -> String {
    format!("clg-package-signature-v0\n{name}\n{version}\n{digest}\n{signed_at}\n")
}
#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::{Signer, SigningKey};
    use std::path::Path;

    fn write_file(path: &Path, text: &str) {
        fs::write(path, text).expect("write file");
    }

    fn write_baseline_runtime_artifacts(root: &Path, artifact_text: &str) {
        let store_dir = root.join("store");
        fs::create_dir_all(&store_dir).expect("create store dir");
        write_file(store_dir.join("pkg-a.wasm").as_path(), artifact_text);

        let digest = format!("sha256:{}", sha256_hex(artifact_text.as_bytes()));
        let runtime_link = serde_json::json!({
            "schema_version": 0,
            "resolver_version": 1,
            "packages": [
                {
                    "id": "pkg::a@1.0.0",
                    "digest": digest,
                    "artifact_path": "store/pkg-a.wasm",
                    "abi_id": "abi:pkg::a:1.0.0"
                }
            ],
            "bindings": [
                {
                    "import_module": "pkg::a",
                    "import_name": "add",
                    "provider_package_id": "pkg::a@1.0.0",
                    "provider_symbol": "add"
                }
            ]
        });
        write_file(
            root.join(RUNTIME_LINK_FILE).as_path(),
            serde_json::to_string_pretty(&runtime_link)
                .expect("serialize runtime-link")
                .as_str(),
        );
        write_file(
            root.join(RUNTIME_LINK_HASH_FILE).as_path(),
            format!(
                "{}\n",
                sha256_hex(canonical_json_bytes(&runtime_link).as_slice())
            )
            .as_str(),
        );

        write_file(
            root.join(PACKAGE_STORE_INDEX_FILE).as_path(),
            serde_json::to_string_pretty(&serde_json::json!({
                "schema_version": 0,
                "artifacts": [
                    {
                        "id": "pkg::a@1.0.0",
                        "digest": digest,
                        "path": "store/pkg-a.wasm"
                    }
                ]
            }))
            .expect("serialize store index")
            .as_str(),
        );

        write_file(
            root.join(STRICT_LOCKFILE_FILE).as_path(),
            serde_json::to_string_pretty(&serde_json::json!({
                "schema_version": 1,
                "resolver_version": 1,
                "roots": [],
                "packages": [
                    {
                        "id": "pkg::a@1.0.0",
                        "name": "pkg::a",
                        "version": "1.0.0",
                        "digest": digest,
                        "abi_id": "abi:pkg::a:1.0.0",
                        "dependencies": []
                    }
                ]
            }))
            .expect("serialize lockfile")
            .as_str(),
        );

        let signing = SigningKey::from_bytes(&[7u8; 32]);
        let signed_at = "2026-06-01T00:00:00Z";
        let payload = canonical_signature_payload_v0("pkg::a", "1.0.0", digest.as_str(), signed_at);
        let signature = hex::encode(signing.sign(payload.as_bytes()).to_bytes());

        write_file(
            root.join("clg.trust-policy.json").as_path(),
            serde_json::to_string_pretty(&serde_json::json!({
                "schema_version": 0,
                "trusted_signers": [
                    {
                        "key_id": "k1",
                        "scheme": "ed25519",
                        "public_key": format!("hex:{}", hex::encode(signing.verifying_key().to_bytes())),
                        "not_before": "2026-01-01T00:00:00Z",
                        "not_after": "2027-01-01T00:00:00Z"
                    }
                ],
                "revoked_key_ids": []
            }))
            .expect("serialize trust policy")
            .as_str(),
        );

        write_file(
            root.join(STRICT_PACKAGE_SIGNATURES_FILE).as_path(),
            serde_json::to_string_pretty(&serde_json::json!({
                "schema_version": 0,
                "signatures": [
                    {
                        "name": "pkg::a",
                        "version": "1.0.0",
                        "digest": digest,
                        "key_id": "k1",
                        "signed_at": signed_at,
                        "signature_format": "ed25519",
                        "signature": signature
                    }
                ]
            }))
            .expect("serialize signature file")
            .as_str(),
        );
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
    fn production_host_profile_requires_runtime_link_artifact() {
        let tmp = tempfile::tempdir().expect("tempdir");
        write_file(
            tmp.path().join(HOST_PROFILE_FILE).as_path(),
            r#"{
  "schema_version": 0,
  "profile": "contract_static",
  "capabilities": ["std::wasi::print"]
}"#,
        );
        let err = load_runtime_packages_from_local_store_if_present(
            tmp.path(),
            RuntimeLoaderConfig::default(),
        )
        .expect_err("missing runtime-link should fail closed in production profile");
        assert_eq!(err.code(), "R012");
        assert!(err.message().contains("fail-closed"));
    }

    #[test]
    fn runtime_host_profile_rejects_unsupported_capability_with_r016() {
        let tmp = tempfile::tempdir().expect("tempdir");
        write_file(
            tmp.path().join(HOST_PROFILE_FILE).as_path(),
            r#"{
  "schema_version": 0,
  "profile": "contract_static",
  "capabilities": ["std::env::unknown"]
}"#,
        );
        let err = load_runtime_packages_from_local_store_if_present(
            tmp.path(),
            RuntimeLoaderConfig::default(),
        )
        .expect_err("unsupported capability should fail");
        assert_eq!(err.code(), "R016");
        assert!(err.message().contains("unsupported capability"));
    }

    #[test]
    fn runtime_host_profile_rejects_duplicate_capability_with_r016() {
        let tmp = tempfile::tempdir().expect("tempdir");
        write_file(
            tmp.path().join(HOST_PROFILE_FILE).as_path(),
            r#"{
  "schema_version": 0,
  "profile": "contract_static",
  "capabilities": ["std::crypto::hash", "std::crypto::hash", "std::wasi::print", "std::wasi::print"]
}"#,
        );
        let err = load_runtime_packages_from_local_store_if_present(
            tmp.path(),
            RuntimeLoaderConfig::default(),
        )
        .expect_err("duplicate capability should fail");
        assert_eq!(err.code(), "R016");
        assert!(err.message().contains("duplicate capability"));
        assert!(err.message().contains("std::crypto::hash"));
    }

    #[test]
    fn runtime_host_profile_rejects_empty_capability_with_r016() {
        let tmp = tempfile::tempdir().expect("tempdir");
        write_file(
            tmp.path().join(HOST_PROFILE_FILE).as_path(),
            r#"{
  "schema_version": 0,
  "profile": "shared_app",
  "capabilities": ["", "std::wasi::print"]
}"#,
        );
        let err = load_runtime_packages_from_local_store_if_present(
            tmp.path(),
            RuntimeLoaderConfig::default(),
        )
        .expect_err("empty capability should fail");
        assert_eq!(err.code(), "R016");
        assert!(err.message().contains("empty capability id"));
    }

    #[test]
    fn runtime_host_profile_missing_required_capability_with_r016() {
        let tmp = tempfile::tempdir().expect("tempdir");
        write_baseline_runtime_artifacts(tmp.path(), "artifact");
        write_file(
            tmp.path().join(HOST_PROFILE_FILE).as_path(),
            r#"{
  "schema_version": 0,
  "profile": "contract_static",
  "capabilities": ["std::wasi::print"]
}"#,
        );
        let err = load_runtime_packages_from_local_store_if_present(
            tmp.path(),
            RuntimeLoaderConfig {
                allow_remote_fetch: false,
                required_host_capabilities: vec!["std::env::time".to_string()],
            },
        )
        .expect_err("missing required capability should fail");
        assert_eq!(err.code(), "R016");
        assert!(err.message().contains("missing required capabilities"));
        assert!(err.message().contains("std::env::time"));
    }

    #[test]
    fn rejects_remote_fetch_in_phase_23_0_1() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let err = load_runtime_packages_from_local_store_if_present(
            tmp.path(),
            RuntimeLoaderConfig {
                allow_remote_fetch: true,
                required_host_capabilities: Vec::new(),
            },
        )
        .expect_err("remote fetch should be rejected");
        assert_eq!(err.code(), "R012");
    }

    #[test]
    fn loads_runtime_packages_from_local_store_index() {
        let tmp = tempfile::tempdir().expect("tempdir");
        write_baseline_runtime_artifacts(
            tmp.path(),
            "(module (func (export \"main\") (result i32) i32.const 0))",
        );
        let loaded = load_runtime_packages_from_local_store_if_present(
            tmp.path(),
            RuntimeLoaderConfig::default(),
        )
        .expect("load runtime packages")
        .expect("runtime-link present");
        assert_eq!(loaded.resolver_version, 1);
        assert_eq!(loaded.packages.len(), 1);
        assert_eq!(loaded.packages[0].id, "pkg::a@1.0.0");
    }

    #[test]
    fn rejects_runtime_link_hash_mismatch_with_r017() {
        let tmp = tempfile::tempdir().expect("tempdir");
        write_baseline_runtime_artifacts(tmp.path(), "artifact");
        write_file(
            tmp.path().join(RUNTIME_LINK_HASH_FILE).as_path(),
            &"a".repeat(64),
        );

        let err = load_runtime_packages_from_local_store_if_present(
            tmp.path(),
            RuntimeLoaderConfig::default(),
        )
        .expect_err("expected hash mismatch");
        assert_eq!(err.code(), "R017");
    }

    #[test]
    fn rejects_lockfile_digest_mismatch_with_r013() {
        let tmp = tempfile::tempdir().expect("tempdir");
        write_baseline_runtime_artifacts(tmp.path(), "artifact");
        write_file(
            tmp.path().join(STRICT_LOCKFILE_FILE).as_path(),
            r#"{
  "schema_version": 1,
  "resolver_version": 1,
  "roots": [],
  "packages": [
    {
      "id": "pkg::a@1.0.0",
      "name": "pkg::a",
      "version": "1.0.0",
      "digest": "sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
      "abi_id": "abi:pkg::a:1.0.0",
      "dependencies": []
    }
  ]
}"#,
        );

        let err = load_runtime_packages_from_local_store_if_present(
            tmp.path(),
            RuntimeLoaderConfig::default(),
        )
        .expect_err("expected lock digest mismatch");
        assert_eq!(err.code(), "R013");
    }

    #[test]
    fn rejects_lockfile_v1_invalid_root_requirement_with_r013() {
        let tmp = tempfile::tempdir().expect("tempdir");
        write_baseline_runtime_artifacts(tmp.path(), "artifact");
        write_file(
            tmp.path().join(STRICT_LOCKFILE_FILE).as_path(),
            r#"{
  "schema_version": 1,
  "resolver_version": 1,
  "roots": [
    {
      "name": "app",
      "dependencies": [{ "name": "pkg::a", "requirement": "latest" }]
    }
  ],
  "packages": [
    {
      "id": "pkg::a@1.0.0",
      "name": "pkg::a",
      "version": "1.0.0",
      "digest": "sha256:c7c5c1d70c5dec441d7d17042f8bbf5be4c4bf4a4c8f89f46b5f58211a711000",
      "abi_id": "abi:pkg::a:1.0.0",
      "dependencies": []
    }
  ]
}"#,
        );

        let err = load_runtime_packages_from_local_store_if_present(
            tmp.path(),
            RuntimeLoaderConfig::default(),
        )
        .expect_err("expected invalid root requirement");
        assert_eq!(err.code(), "R013");
        assert!(err.message().contains("invalid requirement"));
    }

    #[test]
    fn rejects_lockfile_v1_invalid_dependency_id_with_r013() {
        let tmp = tempfile::tempdir().expect("tempdir");
        write_baseline_runtime_artifacts(tmp.path(), "artifact");
        write_file(
            tmp.path().join(STRICT_LOCKFILE_FILE).as_path(),
            r#"{
  "schema_version": 1,
  "resolver_version": 1,
  "roots": [],
  "packages": [
    {
      "id": "pkg::a@1.0.0",
      "name": "pkg::a",
      "version": "1.0.0",
      "digest": "sha256:c7c5c1d70c5dec441d7d17042f8bbf5be4c4bf4a4c8f89f46b5f58211a711000",
      "abi_id": "abi:pkg::a:1.0.0",
      "dependencies": ["not_a_package_id"]
    }
  ]
}"#,
        );

        let err = load_runtime_packages_from_local_store_if_present(
            tmp.path(),
            RuntimeLoaderConfig::default(),
        )
        .expect_err("expected invalid package dependency id");
        assert_eq!(err.code(), "R013");
        assert!(err.message().contains("invalid dependency id"));
    }

    #[test]
    fn rejects_invalid_signature_with_r014() {
        let tmp = tempfile::tempdir().expect("tempdir");
        write_baseline_runtime_artifacts(tmp.path(), "artifact");
        write_file(
            tmp.path().join(STRICT_PACKAGE_SIGNATURES_FILE).as_path(),
            r#"{
  "schema_version": 0,
  "signatures": [
    {
      "name": "pkg::a",
      "version": "1.0.0",
      "digest": "sha256:c7c5c1d70c5dec441d7d17042f8bbf5be4c4bf4a4c8f89f46b5f58211a711000",
      "key_id": "k1",
      "signed_at": "2026-06-01T00:00:00Z",
      "signature_format": "ed25519",
      "signature": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
    }
  ]
}"#,
        );

        let err = load_runtime_packages_from_local_store_if_present(
            tmp.path(),
            RuntimeLoaderConfig::default(),
        )
        .expect_err("expected signature verification failure");
        assert_eq!(err.code(), "R014");
    }

    #[test]
    fn rejects_missing_artifact_in_store_with_r012() {
        let tmp = tempfile::tempdir().expect("tempdir");
        write_baseline_runtime_artifacts(tmp.path(), "artifact");
        write_file(
            tmp.path().join(PACKAGE_STORE_INDEX_FILE).as_path(),
            r#"{"schema_version":0,"artifacts":[]}"#,
        );

        let err = load_runtime_packages_from_local_store_if_present(
            tmp.path(),
            RuntimeLoaderConfig::default(),
        )
        .expect_err("expected missing artifact");
        assert_eq!(err.code(), "R012");
    }

    #[test]
    fn loads_from_mirror_when_primary_artifact_is_unavailable() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let root = tmp.path();
        write_baseline_runtime_artifacts(root, "artifact");

        fs::remove_file(root.join("store").join("pkg-a.wasm")).expect("remove primary artifact");
        let mirror_store = root.join("mirror-a").join("store");
        fs::create_dir_all(&mirror_store).expect("create mirror store");
        write_file(mirror_store.join("pkg-a.wasm").as_path(), "artifact");
        write_file(
            root.join(RUNTIME_LOADER_POLICY_FILE).as_path(),
            r#"{
  "schema_version": 0,
  "offline_mode": true,
  "mirror_paths": ["mirror-a"],
  "artifact_read_retries": 2
}"#,
        );

        let loaded =
            load_runtime_packages_from_local_store_if_present(root, RuntimeLoaderConfig::default())
                .expect("load runtime packages")
                .expect("runtime-link present");
        assert_eq!(
            loaded.packages[0].resolved_path,
            root.join("mirror-a").join("store").join("pkg-a.wasm")
        );
    }

    #[test]
    fn rejects_runtime_loader_policy_with_offline_mode_false() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let root = tmp.path();
        write_baseline_runtime_artifacts(root, "artifact");
        write_file(
            root.join(RUNTIME_LOADER_POLICY_FILE).as_path(),
            r#"{
  "schema_version": 0,
  "offline_mode": false,
  "mirror_paths": [],
  "artifact_read_retries": 1
}"#,
        );

        let err =
            load_runtime_packages_from_local_store_if_present(root, RuntimeLoaderConfig::default())
                .expect_err("offline_mode=false should fail");
        assert_eq!(err.code(), "R012");
        assert!(err.message().contains("offline_mode=false"));
    }

    #[test]
    fn rejects_unknown_binding_provider_package_with_r015() {
        let tmp = tempfile::tempdir().expect("tempdir");
        write_baseline_runtime_artifacts(tmp.path(), "artifact");
        let digest = format!("sha256:{}", sha256_hex("artifact".as_bytes()));
        let runtime_link = serde_json::json!({
            "schema_version": 0,
            "resolver_version": 1,
            "packages": [
                {
                    "id": "pkg::a@1.0.0",
                    "digest": digest,
                    "artifact_path": "store/pkg-a.wasm",
                    "abi_id": "abi:pkg::a:1.0.0"
                }
            ],
            "bindings": [
                {
                    "import_module": "pkg::x",
                    "import_name": "add",
                    "provider_package_id": "pkg::x@1.0.0",
                    "provider_symbol": "add"
                }
            ]
        });
        write_file(
            tmp.path().join(RUNTIME_LINK_FILE).as_path(),
            serde_json::to_string_pretty(&runtime_link)
                .expect("serialize runtime-link")
                .as_str(),
        );
        write_file(
            tmp.path().join(RUNTIME_LINK_HASH_FILE).as_path(),
            format!(
                "{}\n",
                sha256_hex(canonical_json_bytes(&runtime_link).as_slice())
            )
            .as_str(),
        );

        let err = load_runtime_packages_from_local_store_if_present(
            tmp.path(),
            RuntimeLoaderConfig::default(),
        )
        .expect_err("expected provider mismatch");
        assert_eq!(err.code(), "R015");
    }
}
