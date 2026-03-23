use std::collections::{BTreeSet, HashSet};
use std::fs;
use std::path::Path;

use serde::Deserialize;

use super::strict_validation::{
    validate_exact_semver, validate_package_id, validate_semver_requirement, validate_sha256_digest,
};

pub(super) const STRICT_LOCKFILE_FILE: &str = "clg.lock.json";

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct StrictLockfileV0 {
    pub(super) dependencies: Vec<StrictLockDependency>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct StrictLockDependency {
    pub(super) name: String,
    pub(super) version: String,
    pub(super) digest: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct StrictLockfileError {
    code: &'static str,
    message: String,
}

impl StrictLockfileError {
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
struct RawStrictLockfileV0 {
    schema_version: u32,
    dependencies: Vec<RawStrictDependency>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawStrictDependency {
    name: String,
    version: String,
    digest: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawStrictLockfileV1 {
    schema_version: u32,
    resolver_version: u32,
    roots: Vec<RawStrictRootV1>,
    packages: Vec<RawStrictPackageV1>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawStrictRootV1 {
    name: String,
    dependencies: Vec<RawStrictRootDependencyV1>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawStrictRootDependencyV1 {
    name: String,
    requirement: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawStrictPackageV1 {
    id: String,
    name: String,
    version: String,
    digest: String,
    abi_id: String,
    dependencies: Vec<String>,
}
