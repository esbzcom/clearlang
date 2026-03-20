use std::collections::{BTreeSet, HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};

use crate::commands::helpers::{
    canonical_json_bytes, make_single_json_error, sha256_hex, CommandError,
};
use crate::commands::validation::{
    validate_exact_semver, validate_package_id, validate_semver_requirement,
    validate_sha256_digest, validate_utc_rfc3339,
};
use crate::logging::{Logger, StageTimings};

const CANONICAL_PACKAGE_METADATA_FILE: &str = "clg.package-metadata.json";
const STRICT_LOCKFILE_FILE: &str = "clg.lock.json";
const ADVISORY_FILE: &str = "clg.advisories.json";
const RESOLVED_GRAPH_FILE: &str = "clg.resolved-graph.json";
const RESOLVED_GRAPH_HASH_FILE: &str = "clg.resolved-graph.sha256";

#[derive(Deserialize)]
struct PackageMetadataRoot {
    schema_version: u32,
    #[serde(default)]
    packages: Vec<PackageMetadataEntry>,
}

#[derive(Deserialize)]
struct PackageMetadataEntry {
    name: String,
    version: String,
    digest: String,
    #[serde(default)]
    artifact: Option<PackageArtifactEntry>,
    abi_id: String,
    #[serde(default)]
    dependencies: Vec<PackageRequirementEntry>,
}

#[derive(Deserialize)]
struct PackageArtifactEntry {
    #[serde(default)]
    format: String,
    #[serde(default)]
    path: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct AdvisoryRoot {
    schema_version: u32,
    #[serde(default)]
    advisories: Vec<RawAdvisoryEntry>,
}

#[derive(Deserialize)]
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

#[derive(Clone)]
struct AdvisoryEntry {
    id: String,
    package: String,
    affected: ParsedRequirement,
    severity: String,
    action: AdvisoryAction,
    minimum_safe_version: Option<SemVer>,
}

#[derive(Clone, Copy, Eq, PartialEq)]
enum AdvisoryAction {
    Deny,
    Warn,
    ForceUpgrade,
}

#[derive(Clone, Deserialize)]
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

pub fn run_lock(
    generate: bool,
    update: bool,
    root: PathBuf,
    json_errors: bool,
    logger: Logger,
) -> Result<()> {
    let fail_pkg = |code: &'static str, message: String| -> Result<()> {
        if json_errors {
            let json = make_single_json_error(code, "build", message, &root, 0, 0, None);
            Err(CommandError::json(json).into())
        } else {
            Err(anyhow!(message))
        }
    };

    if generate == update {
        return fail_pkg(
            "C027",
            "pkg lock requires exactly one of `--generate` or `--update`".to_string(),
        );
    }

    let mut timings = StageTimings::new();
    let metadata_path = root.join(CANONICAL_PACKAGE_METADATA_FILE);
    let lockfile_path = root.join(STRICT_LOCKFILE_FILE);

    if generate && lockfile_path.exists() {
        return fail_pkg(
            "C027",
            format!(
                "lockfile `{}` already exists; use `clg pkg lock --update`",
                lockfile_path.display()
            ),
        );
    }
    if update && !lockfile_path.exists() {
        return fail_pkg(
            "C027",
            format!(
                "lockfile `{}` does not exist; use `clg pkg lock --generate`",
                lockfile_path.display()
            ),
        );
    }

    let lockfile = {
        let _stage = timings.start(logger, "pkg_load_metadata");
        let root_inputs = if update {
            match load_root_inputs_for_update(lockfile_path.as_path()) {
                Ok(v) => Some(v),
                Err(err) => return fail_pkg(err.code(), err.to_string()),
            }
        } else {
            None
        };
        match load_lockfile_from_metadata(metadata_path.as_path(), root_inputs) {
            Ok(value) => value,
            Err(err) => return fail_pkg(err.code(), err.to_string()),
        }
    };

    let canonical_hash = {
        let _stage = timings.start(logger, "pkg_write_lockfile");
        write_lockfile(lockfile_path.as_path(), &lockfile)?
    };
    let resolved_graph_hash = {
        let _stage = timings.start(logger, "pkg_write_resolved_graph");
        write_resolved_graph_artifact(root.as_path(), &lockfile)?
    };

    println!(
        "wrote {} with {} pinned package(s) [sha256:{}]",
        lockfile_path.display(),
        lockfile.packages.len(),
        canonical_hash
    );
    println!(
        "wrote {} [sha256:{}]",
        root.join(RESOLVED_GRAPH_FILE).display(),
        resolved_graph_hash
    );
    logger.summary(&timings);
    Ok(())
}

fn load_root_inputs_for_update(path: &Path) -> Result<Vec<StrictLockRootV1>, PkgLockError> {
    let content = fs::read_to_string(path)
        .map_err(|err| PkgLockError::new("C111", format!("reading {}: {err}", path.display())))?;
    let raw: ExistingStrictLockfileV1 = serde_json::from_str(content.as_str()).map_err(|err| {
        PkgLockError::new(
            "C111",
            format!("parsing existing lockfile {}: {err}", path.display()),
        )
    })?;
    if raw.schema_version != 1 {
        return Err(PkgLockError::new(
            "C111",
            format!(
                "existing lockfile `{}` must use schema_version 1",
                path.display()
            ),
        ));
    }
    if raw.resolver_version != 1 {
        return Err(PkgLockError::new(
            "C111",
            format!(
                "existing lockfile `{}` must use resolver_version 1",
                path.display()
            ),
        ));
    }

    let mut roots = Vec::with_capacity(raw.roots.len());
    let mut root_names = HashSet::new();
    let mut root_name_dupes = BTreeSet::new();
    for root in raw.roots {
        if root.name.trim().is_empty() {
            return Err(PkgLockError::new(
                "C111",
                format!(
                    "existing lockfile `{}` has root with empty name",
                    path.display()
                ),
            ));
        }
        if !root_names.insert(root.name.clone()) {
            root_name_dupes.insert(root.name.clone());
        }
        let mut dependencies = Vec::with_capacity(root.dependencies.len());
        let mut dep_names = HashSet::new();
        let mut dep_dupes = BTreeSet::new();
        for dep in root.dependencies {
            validate_package_id(dep.name.as_str()).map_err(|msg| {
                PkgLockError::new(
                    "C111",
                    format!(
                        "existing lockfile `{}` root `{}` has invalid dependency name `{}`: {msg}",
                        path.display(),
                        root.name,
                        dep.name
                    ),
                )
            })?;
            validate_semver_requirement(dep.requirement.as_str()).map_err(|msg| {
                PkgLockError::new(
                    "C111",
                    format!(
                        "existing lockfile `{}` root `{}` dependency `{}` has invalid requirement `{}`: {msg}",
                        path.display(),
                        root.name,
                        dep.name,
                        dep.requirement
                    ),
                )
            })?;
            if !dep_names.insert(dep.name.clone()) {
                dep_dupes.insert(dep.name.clone());
            }
            dependencies.push(StrictLockRootDependencyV1 {
                name: dep.name,
                requirement: dep.requirement,
            });
        }
        if let Some(dupe) = dep_dupes.iter().next() {
            return Err(PkgLockError::new(
                "C111",
                format!(
                    "existing lockfile `{}` root `{}` has duplicate dependency `{}`",
                    path.display(),
                    root.name,
                    dupe
                ),
            ));
        }
        dependencies.sort_by(|a, b| a.name.cmp(&b.name));
        roots.push(StrictLockRootV1 {
            name: root.name,
            dependencies,
        });
    }
    if let Some(dupe) = root_name_dupes.iter().next() {
        return Err(PkgLockError::new(
            "C111",
            format!(
                "existing lockfile `{}` has duplicate root name `{}`",
                path.display(),
                dupe
            ),
        ));
    }
    roots.sort_by(|a, b| a.name.cmp(&b.name));
    if roots.is_empty() {
        return Err(PkgLockError::new(
            "C111",
            format!("existing lockfile `{}` has empty `roots`", path.display()),
        ));
    }
    Ok(roots)
}

fn load_advisories(root: &Path) -> Result<Vec<AdvisoryEntry>, PkgLockError> {
    let advisory_path = root.join(ADVISORY_FILE);
    if !advisory_path.exists() {
        return Ok(Vec::new());
    }
    if !advisory_path.is_file() {
        return Err(PkgLockError::new(
            "C117",
            format!(
                "advisory input path `{}` exists but is not a file",
                advisory_path.display()
            ),
        ));
    }

    let content = fs::read_to_string(&advisory_path).map_err(|err| {
        PkgLockError::new(
            "C117",
            format!("reading {}: {err}", advisory_path.display()),
        )
    })?;
    let raw: AdvisoryRoot = serde_json::from_str(content.as_str()).map_err(|err| {
        PkgLockError::new(
            "C117",
            format!("parsing {}: {err}", advisory_path.display()),
        )
    })?;
    if raw.schema_version != 1 {
        return Err(PkgLockError::new(
            "C117",
            format!(
                "unsupported advisory schema_version {} in `{}`; expected 1",
                raw.schema_version,
                advisory_path.display()
            ),
        ));
    }

    let mut seen_ids = HashSet::with_capacity(raw.advisories.len());
    let mut duplicate_ids = BTreeSet::new();
    let mut advisories = Vec::with_capacity(raw.advisories.len());
    for item in raw.advisories {
        if item.id.trim().is_empty() {
            return Err(PkgLockError::new(
                "C117",
                format!(
                    "advisory entry in `{}` has empty id",
                    advisory_path.display()
                ),
            ));
        }
        if !seen_ids.insert(item.id.clone()) {
            duplicate_ids.insert(item.id.clone());
        }
        validate_package_id(item.package.as_str()).map_err(|msg| {
            PkgLockError::new(
                "C117",
                format!(
                    "advisory `{}` in `{}` has invalid package `{}`: {msg}",
                    item.id,
                    advisory_path.display(),
                    item.package
                ),
            )
        })?;
        validate_semver_requirement(item.affected.as_str()).map_err(|msg| {
            PkgLockError::new(
                "C117",
                format!(
                    "advisory `{}` in `{}` has invalid affected range `{}`: {msg}",
                    item.id,
                    advisory_path.display(),
                    item.affected
                ),
            )
        })?;
        let affected = ParsedRequirement::parse(item.affected.as_str()).map_err(|_| {
            PkgLockError::new(
                "C117",
                format!(
                    "advisory `{}` in `{}` has invalid affected range `{}`",
                    item.id,
                    advisory_path.display(),
                    item.affected
                ),
            )
        })?;
        if item.severity.trim().is_empty() {
            return Err(PkgLockError::new(
                "C117",
                format!(
                    "advisory `{}` in `{}` has empty severity",
                    item.id,
                    advisory_path.display()
                ),
            ));
        }
        validate_utc_rfc3339("issued_at", item.issued_at.as_str()).map_err(|msg| {
            PkgLockError::new(
                "C117",
                format!(
                    "advisory `{}` in `{}` has invalid issued_at `{}`: {msg}",
                    item.id,
                    advisory_path.display(),
                    item.issued_at
                ),
            )
        })?;
        validate_utc_rfc3339("expires_at", item.expires_at.as_str()).map_err(|msg| {
            PkgLockError::new(
                "C117",
                format!(
                    "advisory `{}` in `{}` has invalid expires_at `{}`: {msg}",
                    item.id,
                    advisory_path.display(),
                    item.expires_at
                ),
            )
        })?;
        let action = match item.action.as_str() {
            "deny" => AdvisoryAction::Deny,
            "warn" => AdvisoryAction::Warn,
            "force_upgrade" => AdvisoryAction::ForceUpgrade,
            other => {
                return Err(PkgLockError::new(
                    "C117",
                    format!(
                        "advisory `{}` in `{}` has unsupported action `{}`",
                        item.id,
                        advisory_path.display(),
                        other
                    ),
                ))
            }
        };
        let minimum_safe_version = match (action, item.minimum_safe_version) {
            (AdvisoryAction::ForceUpgrade, Some(version)) => {
                validate_exact_semver(version.as_str()).map_err(|msg| {
                    PkgLockError::new(
                        "C117",
                        format!(
                            "advisory `{}` in `{}` has invalid minimum_safe_version `{}`: {msg}",
                            item.id,
                            advisory_path.display(),
                            version
                        ),
                    )
                })?;
                Some(SemVer::parse(version.as_str()).map_err(|_| {
                    PkgLockError::new(
                        "C117",
                        format!(
                            "advisory `{}` in `{}` has invalid minimum_safe_version `{}`",
                            item.id,
                            advisory_path.display(),
                            version
                        ),
                    )
                })?)
            }
            (AdvisoryAction::ForceUpgrade, None) => {
                return Err(PkgLockError::new(
                    "C117",
                    format!(
                        "advisory `{}` in `{}` requires `minimum_safe_version` for `force_upgrade` action",
                        item.id,
                        advisory_path.display()
                    ),
                ))
            }
            (_, version_opt) => {
                if let Some(version) = version_opt {
                    validate_exact_semver(version.as_str()).map_err(|msg| {
                        PkgLockError::new(
                            "C117",
                            format!(
                                "advisory `{}` in `{}` has invalid minimum_safe_version `{}`: {msg}",
                                item.id,
                                advisory_path.display(),
                                version
                            ),
                        )
                    })?;
                }
                None
            }
        };
        advisories.push(AdvisoryEntry {
            id: item.id,
            package: item.package,
            affected,
            severity: item.severity,
            action,
            minimum_safe_version,
        });
    }

    if let Some(duplicate) = duplicate_ids.iter().next() {
        return Err(PkgLockError::new(
            "C117",
            format!(
                "advisory input `{}` has duplicate advisory id `{}`",
                advisory_path.display(),
                duplicate
            ),
        ));
    }
    advisories.sort_by(|a, b| a.package.cmp(&b.package).then_with(|| a.id.cmp(&b.id)));
    Ok(advisories)
}

fn load_lockfile_from_metadata(
    path: &Path,
    root_inputs: Option<Vec<StrictLockRootV1>>,
) -> Result<StrictLockfileV1, PkgLockError> {
    if !path.exists() {
        return Err(PkgLockError::new(
            "C027",
            format!("canonical package metadata `{}` is missing", path.display()),
        ));
    }
    if !path.is_file() {
        return Err(PkgLockError::new(
            "C027",
            format!(
                "canonical package metadata path `{}` exists but is not a file",
                path.display()
            ),
        ));
    }
    let advisory_root = path.parent().unwrap_or(Path::new("."));
    let advisories = load_advisories(advisory_root)?;

    let content = fs::read_to_string(path)
        .map_err(|err| PkgLockError::new("C027", format!("reading {}: {err}", path.display())))?;
    let raw: PackageMetadataRoot = serde_json::from_str(content.as_str())
        .map_err(|err| PkgLockError::new("C027", format!("parsing {}: {err}", path.display())))?;
    if !matches!(raw.schema_version, 0 | 1) {
        return Err(PkgLockError::new(
            "C027",
            format!(
                "unsupported package metadata schema_version {} in `{}`; expected 0 or 1",
                raw.schema_version,
                path.display()
            ),
        ));
    }

    let mut seen_package_ids = HashSet::with_capacity(raw.packages.len());
    let mut duplicate_package_ids = BTreeSet::new();
    let mut packages = Vec::with_capacity(raw.packages.len());
    for pkg in raw.packages {
        validate_package_id(pkg.name.as_str()).map_err(|msg| {
            PkgLockError::new(
                "C027",
                format!("invalid package name `{}`: {msg}", pkg.name),
            )
        })?;
        validate_exact_semver(pkg.version.as_str()).map_err(|msg| {
            PkgLockError::new(
                "C027",
                format!(
                    "invalid package version `{}` for `{}`: {msg}",
                    pkg.version, pkg.name
                ),
            )
        })?;
        let semver = SemVer::parse(pkg.version.as_str())?;
        validate_sha256_digest(pkg.digest.as_str()).map_err(|msg| {
            PkgLockError::new(
                "C027",
                format!("invalid digest `{}` for `{}`: {msg}", pkg.digest, pkg.name),
            )
        })?;
        let artifact_path = match pkg.artifact {
            Some(artifact) => {
                if !artifact.format.is_empty() && artifact.format != "wasm" {
                    return Err(PkgLockError::new(
                        "C027",
                        format!(
                            "invalid artifact format `{}` for `{}`; expected `wasm`",
                            artifact.format, pkg.name
                        ),
                    ));
                }
                artifact.path
            }
            None => String::new(),
        };
        if pkg.abi_id.trim().is_empty() {
            return Err(PkgLockError::new(
                "C027",
                format!("invalid abi_id for `{}`: abi_id is empty", pkg.name),
            ));
        }
        let package_id = format!("{}@{}", pkg.name, pkg.version);
        if !seen_package_ids.insert(package_id.clone()) {
            duplicate_package_ids.insert(package_id);
            continue;
        }

        let mut dependency_seen = HashSet::with_capacity(pkg.dependencies.len());
        let mut dependency_dupes = BTreeSet::new();
        for dep in &pkg.dependencies {
            validate_package_id(dep.name.as_str()).map_err(|msg| {
                PkgLockError::new(
                    "C027",
                    format!(
                        "invalid dependency name `{}` for package `{}`: {msg}",
                        dep.name, pkg.name
                    ),
                )
            })?;
            validate_semver_requirement(dep.requirement.as_str()).map_err(|msg| {
                PkgLockError::new(
                    "C027",
                    format!(
                        "invalid dependency requirement `{}` for package `{}` dependency `{}`: {msg}",
                        dep.requirement, pkg.name, dep.name
                    ),
                )
            })?;
            if !dependency_seen.insert(dep.name.clone()) {
                dependency_dupes.insert(dep.name.clone());
            }
        }
        if let Some(first) = dependency_dupes.iter().next() {
            return Err(PkgLockError::new(
                "C027",
                format!(
                    "duplicate dependency name `{}` in package `{}` in canonical package metadata",
                    first, pkg.name
                ),
            ));
        }

        let mut dependencies = pkg.dependencies;
        dependencies.sort_by(|a, b| a.name.cmp(&b.name));
        packages.push(ValidatedPackage {
            name: pkg.name,
            version: pkg.version,
            semver,
            digest: pkg.digest,
            artifact_path,
            abi_id: pkg.abi_id,
            dependencies,
        });
    }
    if let Some(first) = duplicate_package_ids.iter().next() {
        return Err(PkgLockError::new(
            "C027",
            format!(
                "duplicate package id `{}` in canonical package metadata",
                first
            ),
        ));
    }

    packages.sort_by(|a, b| {
        a.name
            .cmp(&b.name)
            .then_with(|| a.version.cmp(&b.version))
            .then_with(|| a.digest.cmp(&b.digest))
    });
    let mut catalog: HashMap<String, Vec<ValidatedPackage>> = HashMap::new();
    for pkg in packages {
        catalog.entry(pkg.name.clone()).or_default().push(pkg);
    }
    for candidates in catalog.values_mut() {
        candidates.sort_by(|a, b| {
            b.semver
                .cmp(&a.semver)
                .then_with(|| a.digest.cmp(&b.digest))
                .then_with(|| a.artifact_path.cmp(&b.artifact_path))
        });
    }

    for candidates in catalog.values() {
        for pkg in candidates {
            for dep in &pkg.dependencies {
                if !catalog.contains_key(dep.name.as_str()) {
                    return Err(PkgLockError::new(
                        "C027",
                        format!(
                            "package `{}` references dependency `{}` which is not present in canonical package metadata",
                            pkg.name, dep.name
                        ),
                    ));
                }
            }
        }
    }

    let roots = root_inputs.unwrap_or_else(|| derive_root_inputs_for_generate(&catalog));
    let selected = solve_deterministic_versions(&catalog, roots.as_slice(), advisories.as_slice())?;
    if let Some(cycle) = detect_resolved_cycle(&selected) {
        return Err(PkgLockError::new(
            "C112",
            format!("deterministic transitive dependency cycle detected: {cycle}"),
        ));
    }

    let mut selected_names: Vec<&str> = selected.keys().map(|name| name.as_str()).collect();
    selected_names.sort();
    let mut locked_packages = Vec::with_capacity(selected_names.len());
    for name in selected_names {
        let pkg = selected
            .get(name)
            .expect("selected package name should map to a package");
        let mut dependency_ids = Vec::with_capacity(pkg.dependencies.len());
        for dep in &pkg.dependencies {
            let dep_pkg = selected.get(dep.name.as_str()).ok_or_else(|| {
                PkgLockError::new(
                    "C113",
                    format!(
                        "deterministic semver solver found no satisfiable version set for `{}`",
                        dep.name
                    ),
                )
            })?;
            dependency_ids.push(format!("{}@{}", dep_pkg.name, dep_pkg.version));
        }
        dependency_ids.sort();
        locked_packages.push(StrictLockedPackageV1 {
            id: format!("{}@{}", pkg.name, pkg.version),
            name: pkg.name.clone(),
            version: pkg.version.clone(),
            digest: pkg.digest.clone(),
            abi_id: pkg.abi_id.clone(),
            dependencies: dependency_ids,
        });
    }
    locked_packages.sort_by(|a, b| a.id.cmp(&b.id));

    Ok(StrictLockfileV1 {
        schema_version: 1,
        resolver_version: 1,
        roots,
        packages: locked_packages,
    })
}

fn derive_root_inputs_for_generate(
    catalog: &HashMap<String, Vec<ValidatedPackage>>,
) -> Vec<StrictLockRootV1> {
    let mut referenced = HashSet::new();
    for candidates in catalog.values() {
        for pkg in candidates {
            for dep in &pkg.dependencies {
                referenced.insert(dep.name.clone());
            }
        }
    }
    let mut root_names: Vec<String> = catalog
        .keys()
        .filter(|name| !referenced.contains(*name))
        .cloned()
        .collect();
    if root_names.is_empty() {
        root_names = catalog.keys().cloned().collect();
    }
    root_names.sort();

    let mut dependencies = Vec::with_capacity(root_names.len());
    for root_name in root_names {
        let selected = catalog
            .get(root_name.as_str())
            .and_then(|candidates| candidates.first())
            .expect("root package name should have a candidate");
        dependencies.push(StrictLockRootDependencyV1 {
            name: selected.name.clone(),
            requirement: format!("={}", selected.version),
        });
    }
    vec![StrictLockRootV1 {
        name: "app".to_string(),
        dependencies,
    }]
}

fn solve_deterministic_versions(
    catalog: &HashMap<String, Vec<ValidatedPackage>>,
    roots: &[StrictLockRootV1],
    advisories: &[AdvisoryEntry],
) -> Result<HashMap<String, ValidatedPackage>, PkgLockError> {
    let mut advisories_by_package: HashMap<String, Vec<AdvisoryEntry>> = HashMap::new();
    for advisory in advisories {
        advisories_by_package
            .entry(advisory.package.clone())
            .or_default()
            .push(advisory.clone());
    }
    for entries in advisories_by_package.values_mut() {
        entries.sort_by(|a, b| a.id.cmp(&b.id));
    }

    let mut constraints: HashMap<String, Vec<ParsedRequirement>> = HashMap::new();
    for root in roots {
        for dep in &root.dependencies {
            let parsed = ParsedRequirement::parse(dep.requirement.as_str()).map_err(|_| {
                PkgLockError::new(
                    "C111",
                    format!(
                        "root `{}` dependency `{}` has invalid requirement `{}`",
                        root.name, dep.name, dep.requirement
                    ),
                )
            })?;
            constraints
                .entry(dep.name.clone())
                .or_default()
                .push(parsed);
        }
    }
    solve_step(catalog, constraints, HashMap::new(), &advisories_by_package)
}

fn solve_step(
    catalog: &HashMap<String, Vec<ValidatedPackage>>,
    constraints: HashMap<String, Vec<ParsedRequirement>>,
    selected: HashMap<String, ValidatedPackage>,
    advisories_by_package: &HashMap<String, Vec<AdvisoryEntry>>,
) -> Result<HashMap<String, ValidatedPackage>, PkgLockError> {
    let mut unresolved: Vec<&str> = constraints
        .keys()
        .filter(|name| !selected.contains_key(name.as_str()))
        .map(|name| name.as_str())
        .collect();
    unresolved.sort();
    let Some(pkg_name) = unresolved.first().copied() else {
        return Ok(selected);
    };
    let reqs = constraints
        .get(pkg_name)
        .expect("unresolved package should have constraints");
    let candidates = catalog.get(pkg_name).ok_or_else(|| {
        PkgLockError::new(
            "C113",
            format!(
                "deterministic semver solver found no satisfiable version set for `{}`",
                pkg_name
            ),
        )
    })?;

    let matching: Vec<&ValidatedPackage> = candidates
        .iter()
        .filter(|candidate| reqs.iter().all(|req| req.matches(&candidate.semver)))
        .collect();
    if matching.is_empty() {
        return Err(PkgLockError::new(
            "C113",
            format!(
                "deterministic semver solver found no satisfiable version set for `{}` with constraints [{}]",
                pkg_name,
                reqs.iter()
                    .map(|req| req.raw.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        ));
    }

    let mut allowed_candidates = Vec::new();
    let mut deny_hits: Vec<(String, String, String)> = Vec::new();
    let mut force_hits: Vec<(String, String, String, String)> = Vec::new();
    for candidate in matching {
        if let Some(pkg_advisories) = advisories_by_package.get(pkg_name) {
            let mut denied_by: Option<(&str, &str)> = None;
            let mut forced_by: Option<(&str, &str, SemVer)> = None;
            for advisory in pkg_advisories {
                if !advisory.affected.matches(&candidate.semver) {
                    continue;
                }
                match advisory.action {
                    AdvisoryAction::Deny => {
                        denied_by = Some((advisory.id.as_str(), advisory.severity.as_str()));
                        break;
                    }
                    AdvisoryAction::ForceUpgrade => {
                        let min = advisory
                            .minimum_safe_version
                            .expect("force-upgrade advisories require minimum safe version");
                        if candidate.semver < min {
                            forced_by =
                                Some((advisory.id.as_str(), advisory.severity.as_str(), min));
                            break;
                        }
                    }
                    AdvisoryAction::Warn => {}
                }
            }
            if let Some((advisory_id, severity)) = denied_by {
                deny_hits.push((
                    advisory_id.to_string(),
                    severity.to_string(),
                    candidate.version.clone(),
                ));
                continue;
            }
            if let Some((advisory_id, severity, min_safe)) = forced_by {
                force_hits.push((
                    advisory_id.to_string(),
                    severity.to_string(),
                    candidate.version.clone(),
                    format!("{}.{}.{}", min_safe.major, min_safe.minor, min_safe.patch),
                ));
                continue;
            }
        }
        allowed_candidates.push(candidate);
    }

    if allowed_candidates.is_empty() {
        deny_hits.sort();
        force_hits.sort();
        if let Some((advisory_id, severity, version)) = deny_hits.first() {
            return Err(PkgLockError::new(
                "C115",
                format!(
                    "advisory deny policy rejected package `{}` version `{}` (advisory `{}`, severity `{}`)",
                    pkg_name, version, advisory_id, severity
                ),
            ));
        }
        if let Some((advisory_id, severity, version, min_safe)) = force_hits.first() {
            return Err(PkgLockError::new(
                "C116",
                format!(
                    "advisory forced-upgrade policy could not find compliant version for `{}` (selected `{}` requires >= `{}` via advisory `{}` severity `{}`)",
                    pkg_name, version, min_safe, advisory_id, severity
                ),
            ));
        }
        return Err(PkgLockError::new(
            "C113",
            format!(
                "deterministic semver solver found no satisfiable version set for `{}` with constraints [{}]",
                pkg_name,
                reqs.iter()
                    .map(|req| req.raw.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        ));
    }

    let mut best_err: Option<PkgLockError> = None;
    for candidate in allowed_candidates {
        let mut next_selected = selected.clone();
        next_selected.insert(pkg_name.to_string(), candidate.clone());
        let mut next_constraints = constraints.clone();
        let mut branch_valid = true;
        for dep in &candidate.dependencies {
            let parsed = ParsedRequirement::parse(dep.requirement.as_str())?;
            next_constraints
                .entry(dep.name.clone())
                .or_default()
                .push(parsed);
            if let Some(existing_dep) = next_selected.get(dep.name.as_str()) {
                let dep_constraints = next_constraints
                    .get(dep.name.as_str())
                    .expect("constraint list should exist for selected dependency");
                if !dep_constraints
                    .iter()
                    .all(|constraint| constraint.matches(&existing_dep.semver))
                {
                    branch_valid = false;
                    break;
                }
            }
        }
        if !branch_valid {
            continue;
        }
        match solve_step(
            catalog,
            next_constraints,
            next_selected,
            advisories_by_package,
        ) {
            Ok(result) => return Ok(result),
            Err(err) => {
                best_err = match best_err {
                    None => Some(err),
                    Some(prev) => Some(prefer_solver_error(prev, err)),
                };
            }
        }
    }

    if let Some(err) = best_err {
        Err(err)
    } else {
        Err(PkgLockError::new(
            "C113",
            format!(
                "deterministic semver solver found no satisfiable version set for `{}` with constraints [{}]",
                pkg_name,
                reqs.iter()
                    .map(|req| req.raw.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        ))
    }
}

fn prefer_solver_error(left: PkgLockError, right: PkgLockError) -> PkgLockError {
    let left_rank = solver_error_rank(left.code());
    let right_rank = solver_error_rank(right.code());
    if left_rank < right_rank {
        left
    } else if right_rank < left_rank {
        right
    } else if right.message <= left.message {
        right
    } else {
        left
    }
}

fn solver_error_rank(code: &str) -> u8 {
    match code {
        "C115" => 0,
        "C116" => 1,
        "C113" => 2,
        _ => 3,
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum DfsState {
    Visiting,
    Done,
}

fn detect_resolved_cycle(selected: &HashMap<String, ValidatedPackage>) -> Option<String> {
    let mut ids = Vec::with_capacity(selected.len());
    for pkg in selected.values() {
        ids.push(format!("{}@{}", pkg.name, pkg.version));
    }
    ids.sort();

    let mut edges: HashMap<String, Vec<String>> = HashMap::new();
    for pkg in selected.values() {
        let from = format!("{}@{}", pkg.name, pkg.version);
        let mut deps = Vec::new();
        for dep in &pkg.dependencies {
            if let Some(dep_pkg) = selected.get(dep.name.as_str()) {
                deps.push(format!("{}@{}", dep_pkg.name, dep_pkg.version));
            }
        }
        deps.sort();
        edges.insert(from, deps);
    }
    let mut state: HashMap<String, DfsState> = HashMap::new();
    let mut stack = Vec::new();
    let mut stack_set: HashSet<String> = HashSet::new();
    for id in ids {
        if matches!(state.get(id.as_str()), Some(DfsState::Done)) {
            continue;
        }
        if let Some(cycle_ids) =
            dfs_cycle(id.as_str(), &edges, &mut state, &mut stack, &mut stack_set)
        {
            return Some(canonical_cycle_path(cycle_ids));
        }
    }
    None
}

fn dfs_cycle(
    node: &str,
    edges: &HashMap<String, Vec<String>>,
    state: &mut HashMap<String, DfsState>,
    stack: &mut Vec<String>,
    stack_set: &mut HashSet<String>,
) -> Option<Vec<String>> {
    state.insert(node.to_string(), DfsState::Visiting);
    stack.push(node.to_string());
    stack_set.insert(node.to_string());

    if let Some(neighbors) = edges.get(node) {
        for neighbor in neighbors {
            if matches!(state.get(neighbor.as_str()), Some(DfsState::Done)) {
                continue;
            }
            if stack_set.contains(neighbor.as_str()) {
                let start = stack.iter().position(|id| id == neighbor).unwrap_or(0);
                let mut cycle = stack[start..].to_vec();
                cycle.push(neighbor.clone());
                return Some(cycle);
            }
            if let Some(cycle) = dfs_cycle(neighbor, edges, state, stack, stack_set) {
                return Some(cycle);
            }
        }
    }

    stack.pop();
    stack_set.remove(node);
    state.insert(node.to_string(), DfsState::Done);
    None
}

fn canonical_cycle_path(mut cycle_ids: Vec<String>) -> String {
    if cycle_ids.len() <= 2 {
        return cycle_ids.join(" -> ");
    }
    cycle_ids.pop();
    let mut smallest_idx = 0usize;
    for (idx, id) in cycle_ids.iter().enumerate().skip(1) {
        if id < &cycle_ids[smallest_idx] {
            smallest_idx = idx;
        }
    }
    let mut ordered = Vec::with_capacity(cycle_ids.len() + 1);
    ordered.extend(cycle_ids[smallest_idx..].iter().cloned());
    ordered.extend(cycle_ids[..smallest_idx].iter().cloned());
    if let Some(first) = ordered.first().cloned() {
        ordered.push(first);
    }
    ordered.join(" -> ")
}

fn write_resolved_graph_artifact(root: &Path, lockfile: &StrictLockfileV1) -> Result<String> {
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

fn canonical_resolved_graph_bytes(lockfile: &StrictLockfileV1) -> Result<Vec<u8>> {
    let mut packages = Vec::with_capacity(lockfile.packages.len());
    for pkg in &lockfile.packages {
        let mut deps = pkg.dependencies.clone();
        deps.sort();
        packages.push(ResolvedGraphPackageV1 {
            id: pkg.id.clone(),
            dependencies: deps,
        });
    }
    packages.sort_by(|a, b| a.id.cmp(&b.id));
    let artifact = ResolvedGraphArtifactV1 {
        schema_version: lockfile.schema_version,
        resolver_version: lockfile.resolver_version,
        roots: lockfile.roots.clone(),
        packages,
    };
    let value = serde_json::to_value(artifact).context("serializing resolved graph value")?;
    Ok(canonical_json_bytes(&value))
}

fn write_lockfile(path: &Path, lockfile: &StrictLockfileV1) -> Result<String> {
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

fn canonical_lockfile_bytes(lockfile: &StrictLockfileV1) -> Result<Vec<u8>> {
    let value = serde_json::to_value(lockfile).context("serializing strict lockfile value")?;
    Ok(canonical_json_bytes(&value))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lockfile_from_metadata_sorts_and_pins() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let metadata_path = tmp.path().join(CANONICAL_PACKAGE_METADATA_FILE);
        fs::write(
            &metadata_path,
            r#"{
  "schema_version": 1,
  "packages": [
    {
      "name": "z::pkg",
      "version": "2.0.0",
      "digest": "sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
      "artifact": { "format": "wasm", "path": "store/z.wasm" },
      "abi_id": "abi:z::pkg:2.0.0"
    },
    {
      "name": "a::pkg",
      "version": "1.0.0",
      "digest": "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
      "artifact": { "format": "wasm", "path": "store/a.wasm" },
      "abi_id": "abi:a::pkg:1.0.0"
    }
  ]
}"#,
        )
        .expect("write metadata");
        let lockfile = load_lockfile_from_metadata(metadata_path.as_path(), None)
            .expect("load lockfile from metadata");
        assert_eq!(lockfile.schema_version, 1);
        assert_eq!(lockfile.resolver_version, 1);
        assert_eq!(lockfile.packages.len(), 2);
        assert_eq!(lockfile.packages[0].id, "a::pkg@1.0.0");
        assert_eq!(lockfile.packages[1].id, "z::pkg@2.0.0");
        assert_eq!(lockfile.roots.len(), 1);
        assert_eq!(lockfile.roots[0].name, "app");
        assert_eq!(lockfile.roots[0].dependencies[0].name, "a::pkg");
    }

    #[test]
    fn lockfile_from_metadata_derives_roots_from_unreferenced_packages() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let metadata_path = tmp.path().join(CANONICAL_PACKAGE_METADATA_FILE);
        fs::write(
            &metadata_path,
            r#"{
  "schema_version": 1,
  "packages": [
    {
      "name": "app::entry",
      "version": "1.0.0",
      "digest": "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
      "abi_id": "abi:app::entry:1.0.0",
      "dependencies": [
        { "name": "lib::core", "requirement": "^1.0.0" }
      ]
    },
    {
      "name": "lib::core",
      "version": "1.0.0",
      "digest": "sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
      "abi_id": "abi:lib::core:1.0.0"
    }
  ]
}"#,
        )
        .expect("write metadata");

        let lockfile = load_lockfile_from_metadata(metadata_path.as_path(), None)
            .expect("load lockfile from metadata");
        assert_eq!(lockfile.roots.len(), 1);
        assert_eq!(lockfile.roots[0].dependencies.len(), 1);
        assert_eq!(lockfile.roots[0].dependencies[0].name, "app::entry");
    }

    #[test]
    fn lockfile_from_metadata_rejects_duplicate_package_ids() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let metadata_path = tmp.path().join(CANONICAL_PACKAGE_METADATA_FILE);
        fs::write(
            &metadata_path,
            r#"{
  "schema_version": 1,
  "packages": [
    {
      "name": "dup",
      "version": "1.0.0",
      "digest": "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
      "abi_id": "abi:dup:1.0.0"
    },
    {
      "name": "dup",
      "version": "1.0.0",
      "digest": "sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
      "abi_id": "abi:dup-alt:1.0.0"
    }
  ]
}"#,
        )
        .expect("write metadata");
        let err = load_lockfile_from_metadata(metadata_path.as_path(), None)
            .expect_err("expected duplicate error");
        assert!(err.to_string().contains("duplicate package id `dup@1.0.0`"));
    }

    #[test]
    fn lockfile_from_metadata_rejects_unknown_dependency() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let metadata_path = tmp.path().join(CANONICAL_PACKAGE_METADATA_FILE);
        fs::write(
            &metadata_path,
            r#"{
  "schema_version": 1,
  "packages": [
    {
      "name": "app::pkg",
      "version": "1.0.0",
      "digest": "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
      "abi_id": "abi:app::pkg:1.0.0",
      "dependencies": [
        { "name": "missing::pkg", "requirement": "^1.0.0" }
      ]
    }
  ]
}"#,
        )
        .expect("write metadata");
        let err = load_lockfile_from_metadata(metadata_path.as_path(), None)
            .expect_err("expected unknown dependency");
        assert!(err
            .to_string()
            .contains("not present in canonical package metadata"));
    }

    #[test]
    fn lockfile_from_metadata_solver_picks_highest_satisfying_transitive_version() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let metadata_path = tmp.path().join(CANONICAL_PACKAGE_METADATA_FILE);
        fs::write(
            &metadata_path,
            r#"{
  "schema_version": 1,
  "packages": [
    {
      "name": "app::entry",
      "version": "1.0.0",
      "digest": "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
      "abi_id": "abi:app::entry:1.0.0",
      "dependencies": [{ "name": "lib::core", "requirement": "^1.0.0" }]
    },
    {
      "name": "lib::core",
      "version": "1.0.0",
      "digest": "sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
      "abi_id": "abi:lib::core:1.0.0"
    },
    {
      "name": "lib::core",
      "version": "1.2.0",
      "digest": "sha256:cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc",
      "abi_id": "abi:lib::core:1.2.0"
    },
    {
      "name": "lib::core",
      "version": "2.0.0",
      "digest": "sha256:dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd",
      "abi_id": "abi:lib::core:2.0.0"
    }
  ]
}"#,
        )
        .expect("write metadata");

        let lockfile = load_lockfile_from_metadata(metadata_path.as_path(), None)
            .expect("solver should resolve");
        let mut ids: Vec<String> = lockfile.packages.iter().map(|p| p.id.clone()).collect();
        ids.sort();
        assert_eq!(ids, vec!["app::entry@1.0.0", "lib::core@1.2.0"]);
    }

    #[test]
    fn lockfile_from_metadata_reports_c113_for_unsatisfiable_semver_constraints() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let metadata_path = tmp.path().join(CANONICAL_PACKAGE_METADATA_FILE);
        fs::write(
            &metadata_path,
            r#"{
  "schema_version": 1,
  "packages": [
    {
      "name": "app::entry",
      "version": "1.0.0",
      "digest": "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
      "abi_id": "abi:app::entry:1.0.0",
      "dependencies": [{ "name": "lib::core", "requirement": "^2.0.0" }]
    },
    {
      "name": "lib::core",
      "version": "1.2.0",
      "digest": "sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
      "abi_id": "abi:lib::core:1.2.0"
    }
  ]
}"#,
        )
        .expect("write metadata");

        let err = load_lockfile_from_metadata(metadata_path.as_path(), None)
            .expect_err("unsatisfiable constraints should fail");
        assert_eq!(err.code(), "C113");
        assert!(err.to_string().contains("no satisfiable version set"));
    }

    #[test]
    fn lockfile_from_metadata_reports_c115_for_deny_advisory() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let metadata_path = tmp.path().join(CANONICAL_PACKAGE_METADATA_FILE);
        fs::write(
            &metadata_path,
            r#"{
  "schema_version": 1,
  "packages": [
    {
      "name": "app::entry",
      "version": "1.0.0",
      "digest": "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
      "abi_id": "abi:app::entry:1.0.0",
      "dependencies": [{ "name": "lib::core", "requirement": "^1.0.0" }]
    },
    {
      "name": "lib::core",
      "version": "1.1.0",
      "digest": "sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
      "abi_id": "abi:lib::core:1.1.0"
    }
  ]
}"#,
        )
        .expect("write metadata");
        fs::write(
            tmp.path().join(ADVISORY_FILE),
            r#"{
  "schema_version": 1,
  "advisories": [
    {
      "id": "ADV-001",
      "package": "lib::core",
      "affected": "^1.0.0",
      "severity": "high",
      "action": "deny",
      "issued_at": "2026-01-01T00:00:00Z",
      "expires_at": "2027-01-01T00:00:00Z"
    }
  ]
}"#,
        )
        .expect("write advisories");

        let err = load_lockfile_from_metadata(metadata_path.as_path(), None)
            .expect_err("deny advisory should fail");
        assert_eq!(err.code(), "C115");
        assert!(err.to_string().contains("ADV-001"));
    }

    #[test]
    fn lockfile_from_metadata_reports_c116_for_force_upgrade_without_safe_candidate() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let metadata_path = tmp.path().join(CANONICAL_PACKAGE_METADATA_FILE);
        fs::write(
            &metadata_path,
            r#"{
  "schema_version": 1,
  "packages": [
    {
      "name": "app::entry",
      "version": "1.0.0",
      "digest": "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
      "abi_id": "abi:app::entry:1.0.0",
      "dependencies": [{ "name": "lib::core", "requirement": "^1.0.0" }]
    },
    {
      "name": "lib::core",
      "version": "1.0.0",
      "digest": "sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
      "abi_id": "abi:lib::core:1.0.0"
    }
  ]
}"#,
        )
        .expect("write metadata");
        fs::write(
            tmp.path().join(ADVISORY_FILE),
            r#"{
  "schema_version": 1,
  "advisories": [
    {
      "id": "ADV-002",
      "package": "lib::core",
      "affected": "^1.0.0",
      "severity": "critical",
      "action": "force_upgrade",
      "minimum_safe_version": "2.0.0",
      "issued_at": "2026-01-01T00:00:00Z",
      "expires_at": "2027-01-01T00:00:00Z"
    }
  ]
}"#,
        )
        .expect("write advisories");

        let err = load_lockfile_from_metadata(metadata_path.as_path(), None)
            .expect_err("force-upgrade advisory should fail");
        assert_eq!(err.code(), "C116");
        assert!(err.to_string().contains("ADV-002"));
    }

    #[test]
    fn lockfile_from_metadata_force_upgrade_selects_safe_candidate_when_available() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let metadata_path = tmp.path().join(CANONICAL_PACKAGE_METADATA_FILE);
        fs::write(
            &metadata_path,
            r#"{
  "schema_version": 1,
  "packages": [
    {
      "name": "app::entry",
      "version": "1.0.0",
      "digest": "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
      "abi_id": "abi:app::entry:1.0.0",
      "dependencies": [{ "name": "lib::core", "requirement": "~1.0.0" }]
    },
    {
      "name": "lib::core",
      "version": "1.0.0",
      "digest": "sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
      "abi_id": "abi:lib::core:1.0.0"
    },
    {
      "name": "lib::core",
      "version": "1.0.5",
      "digest": "sha256:cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc",
      "abi_id": "abi:lib::core:1.0.5"
    }
  ]
}"#,
        )
        .expect("write metadata");
        fs::write(
            tmp.path().join(ADVISORY_FILE),
            r#"{
  "schema_version": 1,
  "advisories": [
    {
      "id": "ADV-003",
      "package": "lib::core",
      "affected": "^1.0.0",
      "severity": "high",
      "action": "force_upgrade",
      "minimum_safe_version": "1.0.5",
      "issued_at": "2026-01-01T00:00:00Z",
      "expires_at": "2027-01-01T00:00:00Z"
    }
  ]
}"#,
        )
        .expect("write advisories");

        let lockfile = load_lockfile_from_metadata(metadata_path.as_path(), None)
            .expect("force-upgrade should resolve to safe candidate");
        let mut ids: Vec<String> = lockfile.packages.iter().map(|p| p.id.clone()).collect();
        ids.sort();
        assert_eq!(ids, vec!["app::entry@1.0.0", "lib::core@1.0.5"]);
    }

    #[test]
    fn lockfile_from_metadata_reports_c117_for_invalid_advisory_input() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let metadata_path = tmp.path().join(CANONICAL_PACKAGE_METADATA_FILE);
        fs::write(
            &metadata_path,
            r#"{
  "schema_version": 1,
  "packages": [
    {
      "name": "app::entry",
      "version": "1.0.0",
      "digest": "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
      "abi_id": "abi:app::entry:1.0.0"
    }
  ]
}"#,
        )
        .expect("write metadata");
        fs::write(
            tmp.path().join(ADVISORY_FILE),
            r#"{"schema_version":1,"advisories":[{"id":"ADV-BAD"}]}"#,
        )
        .expect("write advisories");

        let err = load_lockfile_from_metadata(metadata_path.as_path(), None)
            .expect_err("invalid advisory schema should fail");
        assert_eq!(err.code(), "C117");
    }

    #[test]
    fn canonical_lockfile_bytes_are_stable_and_hashed() {
        let lockfile = StrictLockfileV1 {
            schema_version: 1,
            resolver_version: 1,
            roots: vec![StrictLockRootV1 {
                name: "app".to_string(),
                dependencies: vec![StrictLockRootDependencyV1 {
                    name: "std::core".to_string(),
                    requirement: "=1.0.0".to_string(),
                }],
            }],
            packages: vec![StrictLockedPackageV1 {
                id: "std::core@1.0.0".to_string(),
                name: "std::core".to_string(),
                version: "1.0.0".to_string(),
                digest: "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
                    .to_string(),
                abi_id: "abi:std::core:1.0.0".to_string(),
                dependencies: Vec::new(),
            }],
        };
        let bytes_a = canonical_lockfile_bytes(&lockfile).expect("canonical bytes");
        let bytes_b = canonical_lockfile_bytes(&lockfile).expect("canonical bytes");
        assert_eq!(bytes_a, bytes_b);
        assert_eq!(
            String::from_utf8(bytes_a.clone()).expect("utf8"),
            "{\"packages\":[{\"abi_id\":\"abi:std::core:1.0.0\",\"dependencies\":[],\"digest\":\"sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\",\"id\":\"std::core@1.0.0\",\"name\":\"std::core\",\"version\":\"1.0.0\"}],\"resolver_version\":1,\"roots\":[{\"dependencies\":[{\"name\":\"std::core\",\"requirement\":\"=1.0.0\"}],\"name\":\"app\"}],\"schema_version\":1}"
        );
        let hash = sha256_hex(bytes_a.as_slice());
        assert_eq!(hash.len(), 64);
        assert!(hash
            .chars()
            .all(|ch| ch.is_ascii_hexdigit() && !ch.is_ascii_uppercase()));
    }

    #[test]
    fn write_lockfile_appends_newline_and_returns_canonical_hash() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let path = tmp.path().join(STRICT_LOCKFILE_FILE);
        let lockfile = StrictLockfileV1 {
            schema_version: 1,
            resolver_version: 1,
            roots: vec![StrictLockRootV1 {
                name: "app".to_string(),
                dependencies: vec![StrictLockRootDependencyV1 {
                    name: "std::host".to_string(),
                    requirement: "=1.0.0".to_string(),
                }],
            }],
            packages: vec![StrictLockedPackageV1 {
                id: "std::host@1.0.0".to_string(),
                name: "std::host".to_string(),
                version: "1.0.0".to_string(),
                digest: "sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"
                    .to_string(),
                abi_id: "abi:std::host:1.0.0".to_string(),
                dependencies: Vec::new(),
            }],
        };
        let hash = write_lockfile(path.as_path(), &lockfile).expect("write lockfile");
        let written = fs::read(path).expect("read lockfile");
        assert_eq!(written.last().copied(), Some(b'\n'));
        let canonical = &written[..written.len() - 1];
        assert_eq!(sha256_hex(canonical), hash);
    }

    #[test]
    fn canonical_resolved_graph_bytes_are_stable_and_hashed() {
        let lockfile = StrictLockfileV1 {
            schema_version: 1,
            resolver_version: 1,
            roots: vec![StrictLockRootV1 {
                name: "app".to_string(),
                dependencies: vec![StrictLockRootDependencyV1 {
                    name: "std::core".to_string(),
                    requirement: "^1.0.0".to_string(),
                }],
            }],
            packages: vec![
                StrictLockedPackageV1 {
                    id: "std::host@1.0.0".to_string(),
                    name: "std::host".to_string(),
                    version: "1.0.0".to_string(),
                    digest:
                        "sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"
                            .to_string(),
                    abi_id: "abi:std::host:1.0.0".to_string(),
                    dependencies: Vec::new(),
                },
                StrictLockedPackageV1 {
                    id: "std::core@1.0.0".to_string(),
                    name: "std::core".to_string(),
                    version: "1.0.0".to_string(),
                    digest:
                        "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
                            .to_string(),
                    abi_id: "abi:std::core:1.0.0".to_string(),
                    dependencies: vec!["std::host@1.0.0".to_string()],
                },
            ],
        };
        let bytes_a = canonical_resolved_graph_bytes(&lockfile).expect("resolved graph bytes");
        let bytes_b = canonical_resolved_graph_bytes(&lockfile).expect("resolved graph bytes");
        assert_eq!(bytes_a, bytes_b);
        let hash = sha256_hex(bytes_a.as_slice());
        assert_eq!(hash.len(), 64);
    }

    #[test]
    fn write_resolved_graph_artifact_writes_bytes_and_hash_sidecar() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let lockfile = StrictLockfileV1 {
            schema_version: 1,
            resolver_version: 1,
            roots: vec![StrictLockRootV1 {
                name: "app".to_string(),
                dependencies: vec![StrictLockRootDependencyV1 {
                    name: "pkg::a".to_string(),
                    requirement: "=1.0.0".to_string(),
                }],
            }],
            packages: vec![StrictLockedPackageV1 {
                id: "pkg::a@1.0.0".to_string(),
                name: "pkg::a".to_string(),
                version: "1.0.0".to_string(),
                digest: "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
                    .to_string(),
                abi_id: "abi:pkg::a:1.0.0".to_string(),
                dependencies: Vec::new(),
            }],
        };
        let hash = write_resolved_graph_artifact(tmp.path(), &lockfile)
            .expect("write resolved graph artifact");
        let graph_bytes = fs::read(tmp.path().join(RESOLVED_GRAPH_FILE)).expect("read graph bytes");
        let graph_canonical = &graph_bytes[..graph_bytes.len() - 1];
        assert_eq!(sha256_hex(graph_canonical), hash);
        let hash_text = fs::read_to_string(tmp.path().join(RESOLVED_GRAPH_HASH_FILE))
            .expect("read graph hash sidecar");
        assert_eq!(hash_text.trim(), hash);
    }

    #[test]
    fn lockfile_from_metadata_reports_c112_for_transitive_cycle() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let metadata_path = tmp.path().join(CANONICAL_PACKAGE_METADATA_FILE);
        fs::write(
            &metadata_path,
            r#"{
  "schema_version": 1,
  "packages": [
    {
      "name": "z::pkg",
      "version": "1.0.0",
      "digest": "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
      "abi_id": "abi:z::pkg:1.0.0",
      "dependencies": [{ "name": "y::pkg", "requirement": "^1.0.0" }]
    },
    {
      "name": "y::pkg",
      "version": "1.0.0",
      "digest": "sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
      "abi_id": "abi:y::pkg:1.0.0",
      "dependencies": [{ "name": "x::pkg", "requirement": "^1.0.0" }]
    },
    {
      "name": "x::pkg",
      "version": "1.0.0",
      "digest": "sha256:cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc",
      "abi_id": "abi:x::pkg:1.0.0",
      "dependencies": [{ "name": "z::pkg", "requirement": "^1.0.0" }]
    }
  ]
}"#,
        )
        .expect("write metadata");

        let err = load_lockfile_from_metadata(metadata_path.as_path(), None)
            .expect_err("expected transitive cycle diagnostics");
        assert_eq!(err.code(), "C112");
        assert!(
            err.to_string()
                .contains("x::pkg@1.0.0 -> z::pkg@1.0.0 -> y::pkg@1.0.0 -> x::pkg@1.0.0"),
            "message: {}",
            err
        );
    }
}
