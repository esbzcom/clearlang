use std::collections::{BTreeSet, HashMap, HashSet};
use std::fs;
use std::path::{Component, Path, PathBuf};

use ed25519_dalek::{Signature, VerifyingKey};
use serde::Deserialize;

use crate::commands::build::strict_trust_policy::load_required_trust_policy_v0;
use crate::commands::helpers::{canonical_json_bytes, sha256_hex};
use crate::commands::modules::host_capability_policy::is_known_host_capability;
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
    pub(super) resolved_path: PathBuf,
}

#[derive(Clone, Debug)]
pub(super) struct LoadedRuntimePackageSet {
    pub(super) packages: Vec<LoadedRuntimePackage>,
    pub(super) bindings: Vec<LoadedRuntimeBinding>,
    pub(super) active_profile_capabilities: Option<Vec<String>>,
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
struct RawStrictLockfileV2 {
    schema_version: u32,
    resolver_version: u32,
    roots: Vec<RawStrictRootV1>,
    packages: Vec<RawStrictPackageV1>,
    std: RawStrictStdSectionV2,
}

#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawStrictStdSectionV2 {
    delivery: String,
    #[serde(default)]
    packages: Vec<RawSharedStdPackageV2>,
}

#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawSharedStdPackageV2 {
    package_id: String,
    version: String,
    verified_std_abi: RawSharedStdAbiV2,
    artifact: RawSharedStdArtifactV2,
    signature: RawSharedStdSignatureV2,
    provenance: RawSharedStdProvenanceV2,
    #[serde(default)]
    symbols: Vec<String>,
    #[serde(default)]
    dependencies: Vec<String>,
}

#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawSharedStdAbiV2 {
    major: u32,
    minor_min: u32,
    minor_max: u32,
}

#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawSharedStdArtifactV2 {
    format: String,
    path: String,
    digest: String,
    size_bytes: u64,
}

#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawSharedStdSignatureV2 {
    key_id: String,
    algorithm: String,
    signed_at: String,
    signature: String,
}

#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawSharedStdProvenanceV2 {
    statement_digest: String,
    statement_format: String,
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
    shared_std_by_id: HashMap<String, RuntimeSharedStdLockEntry>,
}

#[derive(Clone, Debug)]
struct RuntimeSharedStdLockEntry {
    artifact_path: String,
    digest: String,
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

