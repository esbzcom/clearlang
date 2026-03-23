use std::collections::{BTreeSet, HashMap, HashSet};
use std::fs;
use std::path::{Component, Path};

use serde::Deserialize;

use super::strict_validation::{
    validate_exact_semver, validate_package_id, validate_sha256_digest, validate_utc_rfc3339,
};
use crate::commands::validation::validate_semver_requirement;

pub(super) const STRICT_PACKAGE_METADATA_FILE: &str = "clg.package-metadata.json";
pub(super) const STRICT_PACKAGE_ABI_FILE: &str = "clg.package-abi.json";

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct StrictPackageContractV0 {
    pub(super) packages: Vec<StrictPackageMetadataEntry>,
    pub(super) contracts: Vec<StrictAbiContractEntry>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct StrictPackageMetadataEntry {
    pub(super) name: String,
    pub(super) version: String,
    pub(super) digest: String,
    pub(super) artifact_format: String,
    pub(super) artifact_path: String,
    pub(super) abi_id: String,
    pub(super) dependencies: Vec<StrictPackageDependencyRequirement>,
    pub(super) signature: Option<StrictPackageMetadataSignature>,
    pub(super) trusted_anchor_ids: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct StrictPackageDependencyRequirement {
    pub(super) name: String,
    pub(super) requirement: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct StrictPackageMetadataSignature {
    pub(super) format: String,
    pub(super) key_id: String,
    pub(super) signed_at: String,
    pub(super) signature: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct StrictAbiContractEntry {
    pub(super) abi_id: String,
    pub(super) package: String,
    pub(super) version: String,
    pub(super) imports: Vec<StrictAbiImportEntry>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct StrictAbiImportEntry {
    pub(super) symbol: String,
    pub(super) effect: String,
    pub(super) params: Vec<String>,
    pub(super) ret: String,
    pub(super) capability: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct StrictPackageContractError {
    code: &'static str,
    message: String,
}

impl StrictPackageContractError {
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

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawPackageMetadataRootV0 {
    schema_version: u32,
    packages: Vec<RawPackageMetadataEntryV0>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawPackageMetadataEntryV0 {
    name: String,
    version: String,
    digest: String,
    artifact: RawPackageArtifact,
    abi_id: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawPackageMetadataRootV1 {
    schema_version: u32,
    packages: Vec<RawPackageMetadataEntryV1>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawPackageMetadataEntryV1 {
    name: String,
    version: String,
    digest: String,
    artifact: RawPackageArtifact,
    abi_id: String,
    #[serde(default)]
    dependencies: Vec<RawPackageDependencyRequirementV1>,
    signature: RawPackageSignatureV1,
    trust: RawPackageTrustV1,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawPackageDependencyRequirementV1 {
    name: String,
    requirement: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawPackageSignatureV1 {
    format: String,
    key_id: String,
    signed_at: String,
    signature: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawPackageTrustV1 {
    trusted_anchor_ids: Vec<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawPackageArtifact {
    format: String,
    path: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawAbiRoot {
    schema_version: u32,
    contracts: Vec<RawAbiContract>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawAbiContract {
    abi_id: String,
    package: String,
    version: String,
    imports: Vec<RawAbiImport>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawAbiImport {
    symbol: String,
    effect: String,
    params: Vec<String>,
    ret: String,
    capability: Option<String>,
}

