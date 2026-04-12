use std::collections::{BTreeSet, HashMap, HashSet};
use std::fs;
use std::path::{Component, Path, PathBuf};

use anyhow::{anyhow, Context, Result};
use ed25519_dalek::{Signature, VerifyingKey};
use serde::{Deserialize, Serialize};

use crate::commands::build::strict_trust_policy::{load_required_trust_policy_v0, TrustedSignerV0};
use crate::commands::build::CompilerMode;
use crate::commands::helpers::{
    canonical_json_bytes, make_single_json_error, sha256_hex, CommandError,
};
use crate::commands::release_defaults::{
    load_project_manifest_v1, RELEASE_DEFAULT_ADVISORY_PLACEHOLDER,
    RELEASE_DEFAULT_KEY_ID_PLACEHOLDER, STRICT_PROJECT_FILE,
};
use crate::commands::validation::{
    parse_utc_timestamp_components, validate_exact_semver, validate_package_id,
    validate_semver_requirement, validate_sha256_digest, validate_utc_rfc3339,
};
use crate::logging::{Logger, StageTimings};

const CANONICAL_PACKAGE_METADATA_FILE: &str = "clg.package-metadata.json";
const LEGACY_PACKAGE_METADATA_FILE: &str = "clg-packages.json";
const STRICT_LOCKFILE_FILE: &str = "clg.lock.json";
const ADVISORY_FILE: &str = "clg.advisories.json";
const RESOLVED_GRAPH_FILE: &str = "clg.resolved-graph.json";
const RESOLVED_GRAPH_HASH_FILE: &str = "clg.resolved-graph.sha256";

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PackageMetadataRoot {
    schema_version: u32,
    #[serde(default)]
    packages: Vec<PackageMetadataEntry>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PackageMetadataEntry {
    name: String,
    version: String,
    digest: String,
    artifact: PackageArtifactEntry,
    abi_id: String,
    #[serde(default)]
    dependencies: Vec<PackageRequirementEntry>,
    #[serde(default)]
    signature: Option<PackageSignatureEntry>,
    #[serde(default)]
    trust: Option<PackageTrustEntry>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PackageArtifactEntry {
    format: String,
    path: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PackageSignatureEntry {
    format: String,
    key_id: String,
    signed_at: String,
    signature: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PackageTrustEntry {
    trusted_anchor_ids: Vec<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct AdvisoryRoot {
    schema_version: u32,
    #[serde(default)]
    advisories: Vec<RawAdvisoryEntry>,
    #[serde(default)]
    signature: Option<RawAdvisorySignature>,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawAdvisoryEntry {
    id: String,
    package: String,
    affected: String,
    severity: String,
    action: String,
    minimum_safe_version: Option<String>,
    issued_at: String,
    expires_at: String,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawAdvisorySignature {
    key_id: String,
    signed_at: String,
    signature_format: String,
    signature: String,
}

#[derive(Clone)]
struct AdvisoryEntry {
    id: String,
    package: String,
    affected: ParsedRequirement,
    severity: String,
    action: AdvisoryAction,
    minimum_safe_version: Option<SemVer>,
    issued_at: UtcTimestamp,
    expires_at: UtcTimestamp,
}

#[derive(Clone, Copy, Eq, PartialEq)]
enum AdvisoryAction {
    Deny,
    Warn,
    ForceUpgrade,
}

type UtcTimestamp = (u16, u8, u8, u8, u8, u8);

#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct PackageRequirementEntry {
    name: String,
    requirement: String,
}

#[derive(Clone)]
struct ValidatedPackage {
    name: String,
    version: String,
    semver: SemVer,
    digest: String,
    artifact_path: String,
    abi_id: String,
    dependencies: Vec<PackageRequirementEntry>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
struct SemVer {
    major: u64,
    minor: u64,
    patch: u64,
}

impl SemVer {
    fn parse(raw: &str) -> Result<Self, PkgLockError> {
        let mut parts = raw.split('.');
        let major = parts
            .next()
            .ok_or_else(|| PkgLockError::new("C027", "missing major segment"))?
            .parse::<u64>()
            .map_err(|_| PkgLockError::new("C027", format!("invalid major segment in `{raw}`")))?;
        let minor = parts
            .next()
            .ok_or_else(|| PkgLockError::new("C027", "missing minor segment"))?
            .parse::<u64>()
            .map_err(|_| PkgLockError::new("C027", format!("invalid minor segment in `{raw}`")))?;
        let patch = parts
            .next()
            .ok_or_else(|| PkgLockError::new("C027", "missing patch segment"))?
            .parse::<u64>()
            .map_err(|_| PkgLockError::new("C027", format!("invalid patch segment in `{raw}`")))?;
        if parts.next().is_some() {
            return Err(PkgLockError::new(
                "C027",
                format!("too many semver segments in `{raw}`"),
            ));
        }
        Ok(Self {
            major,
            minor,
            patch,
        })
    }
}

#[derive(Clone, Copy)]
enum RequirementKind {
    Exact,
    Compatible,
    Patch,
}

#[derive(Clone)]
struct ParsedRequirement {
    raw: String,
    kind: RequirementKind,
    version: SemVer,
}

impl ParsedRequirement {
    fn parse(raw: &str) -> Result<Self, PkgLockError> {
        let trimmed = raw.trim();
        let (kind, version_str) = if let Some(rest) = trimmed.strip_prefix('=') {
            (RequirementKind::Exact, rest)
        } else if let Some(rest) = trimmed.strip_prefix('^') {
            (RequirementKind::Compatible, rest)
        } else if let Some(rest) = trimmed.strip_prefix('~') {
            (RequirementKind::Patch, rest)
        } else {
            (RequirementKind::Exact, trimmed)
        };
        let version = SemVer::parse(version_str).map_err(|_| {
            PkgLockError::new("C027", format!("invalid semver requirement `{trimmed}`"))
        })?;
        Ok(Self {
            raw: trimmed.to_string(),
            kind,
            version,
        })
    }

    fn matches(&self, candidate: &SemVer) -> bool {
        match self.kind {
            RequirementKind::Exact => candidate == &self.version,
            RequirementKind::Compatible => {
                if self.version.major > 0 {
                    candidate.major == self.version.major
                        && (candidate.minor, candidate.patch)
                            >= (self.version.minor, self.version.patch)
                } else if self.version.minor > 0 {
                    candidate.major == 0
                        && candidate.minor == self.version.minor
                        && candidate.patch >= self.version.patch
                } else {
                    candidate.major == 0
                        && candidate.minor == 0
                        && candidate.patch == self.version.patch
                }
            }
            RequirementKind::Patch => {
                candidate.major == self.version.major
                    && candidate.minor == self.version.minor
                    && candidate.patch >= self.version.patch
            }
        }
    }
}

#[derive(Debug, Serialize)]
struct StrictLockfileV1 {
    schema_version: u32,
    resolver_version: u32,
    roots: Vec<StrictLockRootV1>,
    packages: Vec<StrictLockedPackageV1>,
}

#[derive(Clone, Debug, Serialize)]
struct StrictLockRootV1 {
    name: String,
    dependencies: Vec<StrictLockRootDependencyV1>,
}

#[derive(Clone, Debug, Serialize)]
struct StrictLockRootDependencyV1 {
    name: String,
    requirement: String,
}

#[derive(Clone, Debug, Serialize)]
struct StrictLockedPackageV1 {
    id: String,
    name: String,
    version: String,
    digest: String,
    abi_id: String,
    dependencies: Vec<String>,
}

#[derive(Debug, Serialize)]
struct ResolvedGraphArtifactV1 {
    schema_version: u32,
    resolver_version: u32,
    roots: Vec<StrictLockRootV1>,
    packages: Vec<ResolvedGraphPackageV1>,
}

#[derive(Debug, Serialize)]
struct ResolvedGraphPackageV1 {
    id: String,
    dependencies: Vec<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ExistingStrictLockfileV1 {
    schema_version: u32,
    resolver_version: u32,
    roots: Vec<ExistingStrictRootV1>,
    #[serde(default, rename = "packages")]
    _packages: Vec<serde_json::Value>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ExistingStrictRootV1 {
    name: String,
    dependencies: Vec<ExistingStrictRootDependencyV1>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ExistingStrictRootDependencyV1 {
    name: String,
    requirement: String,
}

#[derive(Debug)]
struct PkgLockError {
    code: &'static str,
    message: String,
}

impl PkgLockError {
    fn new(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }

    fn code(&self) -> &'static str {
        self.code
    }
}

impl std::fmt::Display for PkgLockError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for PkgLockError {}

#[derive(Clone, Debug)]
struct AdvisoryPolicy {
    compiler_mode: CompilerMode,
    advisory_as_of: Option<UtcTimestamp>,
}

impl AdvisoryPolicy {
    fn strict(advisory_as_of: UtcTimestamp) -> Self {
        Self {
            compiler_mode: CompilerMode::Strict,
            advisory_as_of: Some(advisory_as_of),
        }
    }

    fn standard() -> Self {
        Self {
            compiler_mode: CompilerMode::Standard,
            advisory_as_of: None,
        }
    }

    fn permissive() -> Self {
        Self {
            compiler_mode: CompilerMode::Permissive,
            advisory_as_of: None,
        }
    }

    fn is_strict(&self) -> bool {
        self.compiler_mode == CompilerMode::Strict
    }
}
