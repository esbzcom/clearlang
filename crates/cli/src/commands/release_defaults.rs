use std::collections::{BTreeSet, HashSet};
use std::fs;
use std::path::{Component, Path};

use serde::Deserialize;

use crate::commands::modules::is_bundled_std_package_id;
use crate::commands::validation::{
    semver_requirement_matches_version, validate_exact_semver, validate_package_id,
    validate_semver_requirement,
};

pub(crate) const STRICT_PROJECT_FILE: &str = "clg.project.json";
pub(crate) const RELEASE_DEFAULT_ADVISORY_PLACEHOLDER: &str = "REQUIRED_RFC3339_UTC";
pub(crate) const RELEASE_DEFAULT_KEY_ID_PLACEHOLDER: &str = "REQUIRED_KEY_ID";

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ReleaseDefaultsV0 {
    pub(crate) advisory_as_of: String,
    pub(crate) key_id: String,
    pub(crate) out_dir: String,
    pub(crate) trust_policy: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct VerifyTrustAnchorsV1 {
    pub(crate) lean_checker: String,
    pub(crate) coq_checker: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ProjectManifestV1 {
    pub(crate) project: ProjectMetadataV1,
    pub(crate) dependencies: Vec<ProjectDependencyV1>,
    pub(crate) std: Option<ProjectStdConfigV2>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ProjectMetadataV1 {
    pub(crate) name: String,
    pub(crate) description: String,
    pub(crate) version: String,
    pub(crate) clg_version: String,
    pub(crate) entry: String,
    pub(crate) website: String,
    pub(crate) contact: ProjectContactV1,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ProjectContactV1 {
    pub(crate) name: String,
    pub(crate) email: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ProjectDependencyV1 {
    pub(crate) name: String,
    pub(crate) requirement: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ProjectStdConfigV2 {
    pub(crate) delivery: String,
    pub(crate) packages: Vec<ProjectStdPackageRequirementV2>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ProjectStdPackageRequirementV2 {
    pub(crate) package_id: String,
    pub(crate) version_requirement: String,
    pub(crate) verified_std_abi: ProjectStdAbiRequirementV2,
    pub(crate) registry: Option<String>,
    pub(crate) signer_policy: Option<String>,
    pub(crate) allow_compat_shims: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ProjectStdAbiRequirementV2 {
    pub(crate) major: u32,
    pub(crate) minor_min: u32,
    pub(crate) minor_max: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ReleaseDefaultsError {
    message: String,
}

impl ReleaseDefaultsError {
    pub(crate) fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }

    pub(crate) fn message(&self) -> &str {
        &self.message
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawProjectRootV0 {
    schema_version: u32,
    release_defaults: RawReleaseDefaultsV0,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawProjectRootV1 {
    schema_version: u32,
    project: RawProjectMetadataV1,
    #[serde(default)]
    dependencies: Vec<RawProjectDependencyV1>,
    release_defaults: RawReleaseDefaultsV0,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawProjectRootV2 {
    schema_version: u32,
    project: RawProjectMetadataV1,
    #[serde(default)]
    dependencies: Vec<RawProjectDependencyV1>,
    #[serde(default)]
    std: Option<RawProjectStdConfigV2>,
    release_defaults: RawReleaseDefaultsV0,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawProjectMetadataV1 {
    name: String,
    description: String,
    version: String,
    clg_version: String,
    entry: String,
    website: String,
    contact: RawProjectContactV1,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawProjectContactV1 {
    name: String,
    email: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawProjectDependencyV1 {
    name: String,
    requirement: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawProjectStdConfigV2 {
    delivery: String,
    #[serde(default)]
    packages: Vec<RawProjectStdPackageRequirementV2>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawProjectStdPackageRequirementV2 {
    package_id: String,
    version_requirement: String,
    verified_std_abi: RawProjectStdAbiRequirementV2,
    #[serde(default)]
    registry: Option<String>,
    #[serde(default)]
    signer_policy: Option<String>,
    #[serde(default)]
    allow_compat_shims: bool,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawProjectStdAbiRequirementV2 {
    major: u32,
    minor_min: u32,
    minor_max: u32,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawReleaseDefaultsV0 {
    advisory_as_of: String,
    key_id: String,
    out_dir: String,
    trust_policy: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawVerifyTrustPolicyV1 {
    schema_version: u32,
    trust_anchors: RawVerifyTrustAnchors,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawVerifyTrustAnchors {
    lean_checker: String,
    coq_checker: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct ParsedProjectManifest {
    release_defaults: ReleaseDefaultsV0,
    manifest_v1: Option<ProjectManifestV1>,
}

pub(crate) fn release_project_template_pretty_json() -> Vec<u8> {
    let clg_version_requirement = default_project_clg_version_requirement();
    serde_json::to_vec_pretty(&serde_json::json!({
        "schema_version": 1,
        "project": {
            "name": "app",
            "description": "ClearLang project",
            "version": "0.1.0",
            "clg_version": clg_version_requirement,
            "entry": "main.clear",
            "website": "https://example.com",
            "contact": {
                "name": "Project Maintainer",
                "email": "maintainer@example.com",
            }
        },
        "dependencies": [],
        "release_defaults": {
            "advisory_as_of": RELEASE_DEFAULT_ADVISORY_PLACEHOLDER,
            "key_id": RELEASE_DEFAULT_KEY_ID_PLACEHOLDER,
            "out_dir": "out/release",
            "trust_policy": "trust-policy.json",
        }
    }))
    .expect("serialize strict project template")
}

pub(crate) fn load_required_release_defaults_v0(
    root: &Path,
) -> Result<ReleaseDefaultsV0, ReleaseDefaultsError> {
    let path = root.join(STRICT_PROJECT_FILE);
    if !path.exists() {
        return Err(ReleaseDefaultsError::new(format!(
            "strict init/release defaults require `{}` at `{}`",
            STRICT_PROJECT_FILE,
            path.display()
        )));
    }
    if !path.is_file() {
        return Err(ReleaseDefaultsError::new(format!(
            "release defaults path `{}` exists but is not a file",
            path.display()
        )));
    }
    let content = fs::read_to_string(&path)
        .map_err(|err| ReleaseDefaultsError::new(format!("reading {}: {err}", path.display())))?;
    let parsed = parse_project_manifest(content.as_str(), &path)?;
    Ok(parsed.release_defaults)
}

pub(crate) fn load_project_manifest_v1(
    root: &Path,
) -> Result<Option<ProjectManifestV1>, ReleaseDefaultsError> {
    let path = root.join(STRICT_PROJECT_FILE);
    if !path.exists() {
        return Ok(None);
    }
    if !path.is_file() {
        return Err(ReleaseDefaultsError::new(format!(
            "project manifest path `{}` exists but is not a file",
            path.display()
        )));
    }
    let content = fs::read_to_string(&path)
        .map_err(|err| ReleaseDefaultsError::new(format!("reading {}: {err}", path.display())))?;
    let parsed = parse_project_manifest(content.as_str(), &path)?;
    Ok(parsed.manifest_v1)
}

fn parse_project_manifest(
    content: &str,
    path: &Path,
) -> Result<ParsedProjectManifest, ReleaseDefaultsError> {
    let raw_value: serde_json::Value = serde_json::from_str(content).map_err(|err| {
        ReleaseDefaultsError::new(format!(
            "project manifest `{}` is not valid JSON: {err}",
            path.display()
        ))
    })?;
    let schema_version = raw_value
        .get("schema_version")
        .and_then(|value| value.as_u64())
        .ok_or_else(|| {
            ReleaseDefaultsError::new(format!(
                "project manifest `{}` is missing integer `schema_version`",
                path.display()
            ))
        })?;

    match schema_version {
        0 => {
            let raw: RawProjectRootV0 = serde_json::from_value(raw_value).map_err(|err| {
                ReleaseDefaultsError::new(format!(
                    "project manifest `{}` is not valid schema v0 JSON: {err}",
                    path.display()
                ))
            })?;
            debug_assert_eq!(raw.schema_version, 0);
            let defaults = validate_release_defaults(path, raw.release_defaults)?;
            Ok(ParsedProjectManifest {
                release_defaults: defaults,
                manifest_v1: None,
            })
        }
        1 => {
            if raw_value.get("std").is_some() {
                return Err(ReleaseDefaultsError::new(format!(
                    "project manifest `{}` uses schema_version 1 but declares `std`; shared std requires schema_version 2",
                    path.display()
                )));
            }
            let raw: RawProjectRootV1 = serde_json::from_value(raw_value).map_err(|err| {
                ReleaseDefaultsError::new(format!(
                    "project manifest `{}` is not valid schema v1 JSON: {err}",
                    path.display()
                ))
            })?;
            debug_assert_eq!(raw.schema_version, 1);
            let defaults = validate_release_defaults(path, raw.release_defaults)?;
            let project = validate_project_metadata(path, raw.project)?;
            let dependencies = validate_manifest_dependencies(path, raw.dependencies)?;
            Ok(ParsedProjectManifest {
                release_defaults: defaults,
                manifest_v1: Some(ProjectManifestV1 {
                    project,
                    dependencies,
                    std: None,
                }),
            })
        }
        2 => {
            let raw: RawProjectRootV2 = serde_json::from_value(raw_value).map_err(|err| {
                ReleaseDefaultsError::new(format!(
                    "project manifest `{}` is not valid schema v2 JSON: {err}",
                    path.display()
                ))
            })?;
            debug_assert_eq!(raw.schema_version, 2);
            let defaults = validate_release_defaults(path, raw.release_defaults)?;
            let project = validate_project_metadata(path, raw.project)?;
            let dependencies = validate_manifest_dependencies(path, raw.dependencies)?;
            let std = validate_manifest_std(path, raw.std, dependencies.as_slice())?;
            Ok(ParsedProjectManifest {
                release_defaults: defaults,
                manifest_v1: Some(ProjectManifestV1 {
                    project,
                    dependencies,
                    std,
                }),
            })
        }
        other => Err(ReleaseDefaultsError::new(format!(
            "project manifest `{}` has unsupported schema_version {}; expected 0, 1, or 2",
            path.display(),
            other
        ))),
    }
}

fn validate_release_defaults(
    path: &Path,
    defaults: RawReleaseDefaultsV0,
) -> Result<ReleaseDefaultsV0, ReleaseDefaultsError> {
    validate_non_empty(
        path,
        "release_defaults.advisory_as_of",
        defaults.advisory_as_of.as_str(),
    )?;
    validate_non_empty(path, "release_defaults.key_id", defaults.key_id.as_str())?;
    validate_project_relative_path(path, "release_defaults.out_dir", defaults.out_dir.as_str())?;
    validate_project_relative_path(
        path,
        "release_defaults.trust_policy",
        defaults.trust_policy.as_str(),
    )?;
    Ok(ReleaseDefaultsV0 {
        advisory_as_of: defaults.advisory_as_of,
        key_id: defaults.key_id,
        out_dir: defaults.out_dir,
        trust_policy: defaults.trust_policy,
    })
}

fn validate_project_metadata(
    path: &Path,
    project: RawProjectMetadataV1,
) -> Result<ProjectMetadataV1, ReleaseDefaultsError> {
    validate_non_empty(path, "project.name", project.name.as_str())?;
    validate_non_empty(path, "project.description", project.description.as_str())?;
    validate_non_empty(path, "project.version", project.version.as_str())?;
    validate_non_empty(path, "project.clg_version", project.clg_version.as_str())?;
    validate_non_empty(path, "project.entry", project.entry.as_str())?;
    validate_non_empty(path, "project.website", project.website.as_str())?;
    validate_non_empty(path, "project.contact.name", project.contact.name.as_str())?;
    validate_non_empty(
        path,
        "project.contact.email",
        project.contact.email.as_str(),
    )?;

    validate_exact_semver(project.version.as_str()).map_err(|msg| {
        ReleaseDefaultsError::new(format!(
            "project manifest `{}` field `project.version` is invalid `{}`: {}",
            path.display(),
            project.version,
            msg
        ))
    })?;
    validate_semver_requirement(project.clg_version.as_str()).map_err(|msg| {
        ReleaseDefaultsError::new(format!(
            "project manifest `{}` field `project.clg_version` is invalid `{}`: {}",
            path.display(),
            project.clg_version,
            msg
        ))
    })?;
    validate_manifest_relative_path(path, "project.entry", project.entry.as_str())?;
    validate_website(path, project.website.as_str())?;
    validate_email(path, project.contact.email.as_str())?;

    Ok(ProjectMetadataV1 {
        name: project.name,
        description: project.description,
        version: project.version,
        clg_version: project.clg_version,
        entry: project.entry,
        website: project.website,
        contact: ProjectContactV1 {
            name: project.contact.name,
            email: project.contact.email,
        },
    })
}

fn validate_manifest_dependencies(
    path: &Path,
    raw_dependencies: Vec<RawProjectDependencyV1>,
) -> Result<Vec<ProjectDependencyV1>, ReleaseDefaultsError> {
    let mut dependencies = Vec::with_capacity(raw_dependencies.len());
    let mut seen_names = HashSet::with_capacity(raw_dependencies.len());
    let mut duplicate_names = BTreeSet::new();
    for dependency in raw_dependencies {
        validate_package_id(dependency.name.as_str()).map_err(|msg| {
            ReleaseDefaultsError::new(format!(
                "project manifest `{}` has invalid dependency name `{}`: {}",
                path.display(),
                dependency.name,
                msg
            ))
        })?;
        validate_semver_requirement(dependency.requirement.as_str()).map_err(|msg| {
            ReleaseDefaultsError::new(format!(
                "project manifest `{}` dependency `{}` has invalid requirement `{}`: {}",
                path.display(),
                dependency.name,
                dependency.requirement,
                msg
            ))
        })?;
        if !seen_names.insert(dependency.name.clone()) {
            duplicate_names.insert(dependency.name.clone());
        }
        dependencies.push(ProjectDependencyV1 {
            name: dependency.name,
            requirement: dependency.requirement,
        });
    }
    if let Some(first) = duplicate_names.iter().next() {
        return Err(ReleaseDefaultsError::new(format!(
            "project manifest `{}` has duplicate dependency `{}`",
            path.display(),
            first
        )));
    }
    dependencies.sort_by(|a, b| {
        a.name
            .cmp(&b.name)
            .then_with(|| a.requirement.cmp(&b.requirement))
    });
    Ok(dependencies)
}

fn validate_manifest_std(
    path: &Path,
    raw_std: Option<RawProjectStdConfigV2>,
    dependencies: &[ProjectDependencyV1],
) -> Result<Option<ProjectStdConfigV2>, ReleaseDefaultsError> {
    let Some(raw_std) = raw_std else {
        return Ok(None);
    };
    if raw_std.delivery != "embedded" && raw_std.delivery != "shared" {
        return Err(ReleaseDefaultsError::new(format!(
            "project manifest `{}` field `std.delivery` must be `embedded` or `shared`, got `{}`",
            path.display(),
            raw_std.delivery
        )));
    }
    if raw_std.delivery == "embedded" && !raw_std.packages.is_empty() {
        return Err(ReleaseDefaultsError::new(format!(
            "project manifest `{}` declares `std.delivery = embedded` but `std.packages[]` is not empty",
            path.display()
        )));
    }
    if raw_std.delivery == "shared" && raw_std.packages.is_empty() {
        return Err(ReleaseDefaultsError::new(format!(
            "project manifest `{}` declares `std.delivery = shared` but `std.packages[]` is empty",
            path.display()
        )));
    }

    let dependency_names = dependencies
        .iter()
        .map(|dependency| dependency.name.as_str())
        .collect::<HashSet<_>>();
    let mut packages = Vec::with_capacity(raw_std.packages.len());
    let mut seen_package_ids = HashSet::with_capacity(raw_std.packages.len());
    let mut duplicate_package_ids = BTreeSet::new();
    for package in raw_std.packages {
        if !is_bundled_std_package_id(package.package_id.as_str()) {
            return Err(ReleaseDefaultsError::new(format!(
                "project manifest `{}` has unsupported shared std package_id `{}`",
                path.display(),
                package.package_id
            )));
        }
        if !seen_package_ids.insert(package.package_id.clone()) {
            duplicate_package_ids.insert(package.package_id.clone());
        }
        if dependency_names.contains(package.package_id.as_str()) {
            return Err(ReleaseDefaultsError::new(format!(
                "project manifest `{}` cannot declare shared std package `{}` in both `dependencies[]` and `std.packages[]`",
                path.display(),
                package.package_id
            )));
        }
        validate_semver_requirement(package.version_requirement.as_str()).map_err(|msg| {
            ReleaseDefaultsError::new(format!(
                "project manifest `{}` shared std package `{}` has invalid version_requirement `{}`: {}",
                path.display(),
                package.package_id,
                package.version_requirement,
                msg
            ))
        })?;
        if package.verified_std_abi.major < 1 {
            return Err(ReleaseDefaultsError::new(format!(
                "project manifest `{}` shared std package `{}` must declare `verified_std_abi.major >= 1`",
                path.display(),
                package.package_id
            )));
        }
        if package.verified_std_abi.minor_max < package.verified_std_abi.minor_min {
            return Err(ReleaseDefaultsError::new(format!(
                "project manifest `{}` shared std package `{}` has invalid verified_std_abi range {}.{}..{}",
                path.display(),
                package.package_id,
                package.verified_std_abi.major,
                package.verified_std_abi.minor_min,
                package.verified_std_abi.minor_max
            )));
        }
        if let Some(registry) = package.registry.as_ref() {
            validate_non_empty(path, "std.packages[].registry", registry.as_str())?;
        }
        if let Some(signer_policy) = package.signer_policy.as_ref() {
            validate_non_empty(path, "std.packages[].signer_policy", signer_policy.as_str())?;
        }
        packages.push(ProjectStdPackageRequirementV2 {
            package_id: package.package_id,
            version_requirement: package.version_requirement,
            verified_std_abi: ProjectStdAbiRequirementV2 {
                major: package.verified_std_abi.major,
                minor_min: package.verified_std_abi.minor_min,
                minor_max: package.verified_std_abi.minor_max,
            },
            registry: package.registry,
            signer_policy: package.signer_policy,
            allow_compat_shims: package.allow_compat_shims,
        });
    }
    if let Some(first) = duplicate_package_ids.iter().next() {
        return Err(ReleaseDefaultsError::new(format!(
            "project manifest `{}` has duplicate shared std package `{}`",
            path.display(),
            first
        )));
    }
    packages.sort_by(|a, b| a.package_id.cmp(&b.package_id));
    Ok(Some(ProjectStdConfigV2 {
        delivery: raw_std.delivery,
        packages,
    }))
}

fn validate_website(path: &Path, website: &str) -> Result<(), ReleaseDefaultsError> {
    let normalized = website.trim();
    if !(normalized.starts_with("https://") || normalized.starts_with("http://")) {
        return Err(ReleaseDefaultsError::new(format!(
            "project manifest `{}` field `project.website` must start with `https://` or `http://`",
            path.display()
        )));
    }
    if normalized.chars().any(char::is_whitespace) {
        return Err(ReleaseDefaultsError::new(format!(
            "project manifest `{}` field `project.website` must not contain whitespace",
            path.display()
        )));
    }
    Ok(())
}

fn validate_email(path: &Path, email: &str) -> Result<(), ReleaseDefaultsError> {
    let normalized = email.trim();
    if normalized.chars().any(char::is_whitespace)
        || !normalized.contains('@')
        || normalized.starts_with('@')
        || normalized.ends_with('@')
    {
        return Err(ReleaseDefaultsError::new(format!(
            "project manifest `{}` field `project.contact.email` must be a valid email-like value",
            path.display()
        )));
    }
    Ok(())
}

pub(crate) fn load_verify_trust_policy_v1(
    path: &Path,
) -> Result<VerifyTrustAnchorsV1, ReleaseDefaultsError> {
    let bytes = fs::read(path).map_err(|err| {
        ReleaseDefaultsError::new(format!(
            "reading compile-time trust-anchor policy `{}`: {err}",
            path.display()
        ))
    })?;
    let policy: RawVerifyTrustPolicyV1 =
        serde_json::from_slice(bytes.as_slice()).map_err(|err| {
            ReleaseDefaultsError::new(format!(
                "parsing compile-time trust-anchor policy `{}`: {err}",
                path.display()
            ))
        })?;
    if policy.schema_version != 1 {
        return Err(ReleaseDefaultsError::new(format!(
            "unsupported compile-time trust-anchor policy schema_version {} (expected 1) in {}",
            policy.schema_version,
            path.display()
        )));
    }
    if policy.trust_anchors.lean_checker.trim().is_empty()
        || policy.trust_anchors.coq_checker.trim().is_empty()
    {
        return Err(ReleaseDefaultsError::new(format!(
            "compile-time trust-anchor policy `{}` must provide non-empty trust_anchors.lean_checker and trust_anchors.coq_checker",
            path.display()
        )));
    }
    Ok(VerifyTrustAnchorsV1 {
        lean_checker: policy.trust_anchors.lean_checker,
        coq_checker: policy.trust_anchors.coq_checker,
    })
}

pub(crate) fn is_release_defaults_placeholder(value: &str) -> bool {
    value == RELEASE_DEFAULT_ADVISORY_PLACEHOLDER || value == RELEASE_DEFAULT_KEY_ID_PLACEHOLDER
}

pub(crate) fn default_project_clg_version_requirement() -> String {
    let current_core = normalized_clg_core_version();
    let mut parts = current_core.split('.');
    let major = parts.next().and_then(|raw| raw.parse::<u64>().ok());
    let minor = parts.next().and_then(|raw| raw.parse::<u64>().ok());
    let patch = parts.next().and_then(|raw| raw.parse::<u64>().ok());
    if major.is_none() || minor.is_none() || patch.is_none() || parts.next().is_some() {
        return "^0.1.0".to_string();
    }
    format!("^{}.{}.0", major.unwrap_or(0), minor.unwrap_or(1))
}

pub(crate) fn enforce_project_clg_version_compatibility(
    manifest: &ProjectManifestV1,
) -> Result<(), ReleaseDefaultsError> {
    let current_full = env!("CARGO_PKG_VERSION");
    let current_core = normalized_clg_core_version();
    let requirement = manifest.project.clg_version.as_str();
    let is_match =
        semver_requirement_matches_version(requirement, current_core).map_err(|msg| {
            ReleaseDefaultsError::new(format!(
                "project manifest `{}` field `project.clg_version` compatibility check failed: {}",
                STRICT_PROJECT_FILE, msg
            ))
        })?;
    if !is_match {
        return Err(ReleaseDefaultsError::new(format!(
            "project manifest `{}` requires `project.clg_version = {}` but current `clg` version is `{}`",
            STRICT_PROJECT_FILE, requirement, current_full
        )));
    }
    Ok(())
}

fn normalized_clg_core_version() -> &'static str {
    let current_full = env!("CARGO_PKG_VERSION");
    current_full
        .split(['-', '+'])
        .next()
        .unwrap_or(current_full)
}

fn validate_non_empty(path: &Path, field: &str, value: &str) -> Result<(), ReleaseDefaultsError> {
    if value.trim().is_empty() {
        return Err(ReleaseDefaultsError::new(format!(
            "release defaults `{}` field `{}` must be non-empty",
            path.display(),
            field
        )));
    }
    Ok(())
}

fn validate_project_relative_path(
    path: &Path,
    field: &str,
    value: &str,
) -> Result<(), ReleaseDefaultsError> {
    validate_relative_path(path, field, value, "release defaults")
}

fn validate_manifest_relative_path(
    path: &Path,
    field: &str,
    value: &str,
) -> Result<(), ReleaseDefaultsError> {
    validate_relative_path(path, field, value, "project manifest")
}

fn validate_relative_path(
    path: &Path,
    field: &str,
    value: &str,
    contract: &str,
) -> Result<(), ReleaseDefaultsError> {
    validate_non_empty(path, field, value)?;
    let relative = Path::new(value);
    if relative.is_absolute() {
        return Err(ReleaseDefaultsError::new(format!(
            "{} `{}` field `{}` must be a relative path",
            contract,
            path.display(),
            field
        )));
    }
    for component in relative.components() {
        match component {
            Component::ParentDir => {
                return Err(ReleaseDefaultsError::new(format!(
                    "{} `{}` field `{}` must not contain `..`",
                    contract,
                    path.display(),
                    field
                )));
            }
            Component::Prefix(_) | Component::RootDir => {
                return Err(ReleaseDefaultsError::new(format!(
                    "{} `{}` field `{}` must be a relative path",
                    contract,
                    path.display(),
                    field
                )));
            }
            Component::CurDir | Component::Normal(_) => {}
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests;
