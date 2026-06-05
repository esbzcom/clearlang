use std::collections::HashSet;
use std::fs;
use std::path::{Component, Path};

use serde::{Deserialize, Serialize};

use crate::commands::validation::{
    validate_exact_semver, validate_package_id, validate_sha256_digest,
};

const STRICT_LOCKFILE_FILE: &str = "clg.lock.json";

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq, Ord, PartialOrd)]
pub(crate) struct SharedStdAbiClaim {
    pub(crate) major: u32,
    pub(crate) minor_min: u32,
    pub(crate) minor_max: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq, Ord, PartialOrd)]
pub(crate) struct SharedStdEvidence {
    pub(crate) package_id: String,
    pub(crate) version: String,
    pub(crate) verified_std_abi: SharedStdAbiClaim,
    pub(crate) artifact_digest: String,
    pub(crate) signature_key_id: String,
    pub(crate) provenance_digest: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct ValidatedSharedStdLockSectionV2 {
    pub(crate) delivery: String,
    pub(crate) packages: Vec<ValidatedSharedStdLockPackageV2>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct ValidatedSharedStdLockPackageV2 {
    pub(crate) package_id: String,
    pub(crate) version: String,
    pub(crate) verified_std_abi: SharedStdAbiClaim,
    pub(crate) artifact: ValidatedSharedStdArtifactV2,
    pub(crate) signature: ValidatedSharedStdSignatureV2,
    pub(crate) provenance: ValidatedSharedStdProvenanceV2,
    pub(crate) symbols: Vec<String>,
    pub(crate) dependencies: Vec<String>,
}

impl ValidatedSharedStdLockPackageV2 {
    pub(crate) fn exact_id(&self) -> String {
        format!("{}@{}", self.package_id, self.version)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct ValidatedSharedStdArtifactV2 {
    pub(crate) format: String,
    pub(crate) path: String,
    pub(crate) digest: String,
    pub(crate) size_bytes: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct ValidatedSharedStdSignatureV2 {
    pub(crate) key_id: String,
    pub(crate) algorithm: String,
    pub(crate) signed_at: String,
    pub(crate) signature: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct ValidatedSharedStdProvenanceV2 {
    pub(crate) statement_digest: String,
    pub(crate) statement_format: String,
}

#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawSharedStdLockfileEnvelopeV2 {
    schema_version: u32,
    resolver_version: u32,
    roots: Vec<serde_json::Value>,
    packages: Vec<RawNormalPackageIdV1>,
    std: RawStrictStdSectionV2,
}

#[derive(Clone, Deserialize)]
struct RawNormalPackageIdV1 {
    id: String,
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

pub(crate) fn load_validated_shared_std_lock_section(
    root: &Path,
) -> Result<Option<ValidatedSharedStdLockSectionV2>, String> {
    let path = root.join(STRICT_LOCKFILE_FILE);
    if !path.exists() {
        return Ok(None);
    }
    let bytes = fs::read(&path)
        .map_err(|err| format!("reading shared std lockfile `{}`: {err}", path.display()))?;
    let value: serde_json::Value = serde_json::from_slice(bytes.as_slice())
        .map_err(|err| format!("parsing shared std lockfile `{}`: {err}", path.display()))?;
    parse_validated_shared_std_lock_section(&value, &path, "shared std lockfile")
}

pub(crate) fn parse_validated_shared_std_lock_section(
    value: &serde_json::Value,
    path: &Path,
    subject: &str,
) -> Result<Option<ValidatedSharedStdLockSectionV2>, String> {
    let schema_version = value
        .get("schema_version")
        .and_then(serde_json::Value::as_u64)
        .ok_or_else(|| {
            format!(
                "{subject} `{}` is missing required `schema_version`",
                path.display()
            )
        })?;
    if schema_version != 2 {
        return Ok(None);
    }
    let raw: RawSharedStdLockfileEnvelopeV2 = serde_json::from_value(value.clone()).map_err(|_| {
        format!(
            "{subject} `{}` does not match schema v2 (`schema_version`, `resolver_version`, `roots[]`, `packages[]`, `std`)",
            path.display()
        )
    })?;
    debug_assert_eq!(raw.schema_version, 2);
    let _ = raw.roots.len();
    if raw.resolver_version != 1 {
        return Err(format!(
            "{subject} `{}` has unsupported resolver_version {}; expected 1",
            path.display(),
            raw.resolver_version
        ));
    }
    let normal_package_ids = raw
        .packages
        .into_iter()
        .map(|pkg| pkg.id)
        .collect::<HashSet<_>>();
    validate_shared_std_section_v2(path, subject, raw.std, &normal_package_ids).map(Some)
}

pub(crate) fn project_shared_std_evidence(
    shared_std: &ValidatedSharedStdLockSectionV2,
) -> Vec<SharedStdEvidence> {
    if shared_std.delivery != "shared" {
        return Vec::new();
    }
    let mut out = shared_std
        .packages
        .iter()
        .map(|package| SharedStdEvidence {
            package_id: package.package_id.clone(),
            version: package.version.clone(),
            verified_std_abi: package.verified_std_abi.clone(),
            artifact_digest: package.artifact.digest.clone(),
            signature_key_id: package.signature.key_id.clone(),
            provenance_digest: package.provenance.statement_digest.clone(),
        })
        .collect::<Vec<_>>();
    out.sort();
    out
}

pub(crate) fn canonicalize_shared_std_lock_section(
    mut shared_std: ValidatedSharedStdLockSectionV2,
) -> ValidatedSharedStdLockSectionV2 {
    for package in &mut shared_std.packages {
        package.symbols.sort();
        package.dependencies.sort();
    }
    shared_std
        .packages
        .sort_by_key(|package| package.exact_id());
    shared_std
}

pub(crate) fn normalize_and_validate_shared_std_lock_section_for_lockfile<R, P>(
    roots: &[R],
    packages: &[P],
    shared_std: ValidatedSharedStdLockSectionV2,
    path: &Path,
    subject: &str,
) -> Result<ValidatedSharedStdLockSectionV2, String>
where
    R: Serialize,
    P: Serialize,
{
    let normalized = canonicalize_shared_std_lock_section(shared_std);
    let envelope = serde_json::json!({
        "schema_version": 2,
        "resolver_version": 1,
        "roots": roots,
        "packages": packages,
        "std": normalized,
    });
    parse_validated_shared_std_lock_section(&envelope, path, subject)?.ok_or_else(|| {
        format!(
            "{subject} `{}` did not validate as schema v2",
            path.display()
        )
    })
}

fn validate_shared_std_section_v2(
    path: &Path,
    subject: &str,
    raw: RawStrictStdSectionV2,
    normal_package_ids: &HashSet<String>,
) -> Result<ValidatedSharedStdLockSectionV2, String> {
    let delivery = raw.delivery.as_str();
    if delivery != "embedded" && delivery != "shared" {
        return Err(format!(
            "{subject} `{}` has invalid `std.delivery` `{}`; expected `embedded` or `shared`",
            path.display(),
            raw.delivery
        ));
    }
    if delivery == "embedded" && !raw.packages.is_empty() {
        return Err(format!(
            "{subject} `{}` declares `std.delivery = embedded` but also contains `std.packages[]`",
            path.display()
        ));
    }
    if delivery == "shared" && raw.packages.is_empty() {
        return Err(format!(
            "{subject} `{}` declares `std.delivery = shared` but `std.packages[]` is empty",
            path.display()
        ));
    }

    let mut validated = Vec::with_capacity(raw.packages.len());
    let mut previous_id: Option<String> = None;
    let mut seen_exact_ids = HashSet::with_capacity(raw.packages.len());
    for shared_pkg in raw.packages {
        validate_package_id(shared_pkg.package_id.as_str()).map_err(|msg| {
            format!(
                "{subject} `{}` has invalid shared std package_id `{}`: {msg}",
                path.display(),
                shared_pkg.package_id
            )
        })?;
        validate_exact_semver(shared_pkg.version.as_str()).map_err(|msg| {
            format!(
                "{subject} `{}` has invalid shared std version `{}` for `{}`: {msg}",
                path.display(),
                shared_pkg.version,
                shared_pkg.package_id
            )
        })?;
        if shared_pkg.verified_std_abi.minor_max < shared_pkg.verified_std_abi.minor_min {
            return Err(format!(
                "{subject} `{}` shared std package `{}` has invalid verified_std_abi range {}.{}..{}",
                path.display(),
                shared_pkg.package_id,
                shared_pkg.verified_std_abi.major,
                shared_pkg.verified_std_abi.minor_min,
                shared_pkg.verified_std_abi.minor_max
            ));
        }
        if shared_pkg.artifact.format.trim().is_empty() {
            return Err(format!(
                "{subject} `{}` shared std package `{}` has empty artifact format",
                path.display(),
                shared_pkg.package_id
            ));
        }
        validate_relative_artifact_path(shared_pkg.artifact.path.as_str()).map_err(|msg| {
            format!(
                "{subject} `{}` shared std package `{}` has invalid artifact path `{}`: {msg}",
                path.display(),
                shared_pkg.package_id,
                shared_pkg.artifact.path
            )
        })?;
        if shared_pkg.artifact.size_bytes == 0 {
            return Err(format!(
                "{subject} `{}` shared std package `{}` has zero artifact size_bytes",
                path.display(),
                shared_pkg.package_id
            ));
        }
        validate_sha256_digest(shared_pkg.artifact.digest.as_str()).map_err(|msg| {
            format!(
                "{subject} `{}` shared std package `{}` has invalid artifact digest `{}`: {msg}",
                path.display(),
                shared_pkg.package_id,
                shared_pkg.artifact.digest
            )
        })?;
        if shared_pkg.signature.key_id.trim().is_empty()
            || shared_pkg.signature.algorithm.trim().is_empty()
            || shared_pkg.signature.signed_at.trim().is_empty()
            || shared_pkg.signature.signature.trim().is_empty()
        {
            return Err(format!(
                "{subject} `{}` shared std package `{}` has incomplete signature fields",
                path.display(),
                shared_pkg.package_id
            ));
        }
        validate_sha256_digest(shared_pkg.provenance.statement_digest.as_str()).map_err(|msg| {
            format!(
                "{subject} `{}` shared std package `{}` has invalid provenance statement_digest `{}`: {msg}",
                path.display(),
                shared_pkg.package_id,
                shared_pkg.provenance.statement_digest
            )
        })?;
        if shared_pkg.provenance.statement_format.trim().is_empty() {
            return Err(format!(
                "{subject} `{}` shared std package `{}` has empty provenance statement_format",
                path.display(),
                shared_pkg.package_id
            ));
        }
        let mut previous_symbol: Option<&str> = None;
        let mut seen_symbols = HashSet::with_capacity(shared_pkg.symbols.len());
        for symbol in &shared_pkg.symbols {
            if symbol.trim().is_empty() {
                return Err(format!(
                    "{subject} `{}` shared std package `{}` has empty symbol entry",
                    path.display(),
                    shared_pkg.package_id
                ));
            }
            if let Some(prev) = previous_symbol {
                if symbol.as_str() < prev {
                    return Err(format!(
                        "{subject} `{}` shared std package `{}` symbols must be sorted",
                        path.display(),
                        shared_pkg.package_id
                    ));
                }
            }
            if !seen_symbols.insert(symbol.as_str()) {
                return Err(format!(
                    "{subject} `{}` shared std package `{}` has duplicate symbol `{}`",
                    path.display(),
                    shared_pkg.package_id,
                    symbol
                ));
            }
            previous_symbol = Some(symbol.as_str());
        }
        let mut previous_dependency: Option<&str> = None;
        let mut seen_dependencies = HashSet::with_capacity(shared_pkg.dependencies.len());
        for dep in &shared_pkg.dependencies {
            validate_exact_package_id(dep.as_str()).map_err(|msg| {
                format!(
                    "{subject} `{}` shared std package `{}` has invalid dependency id `{}`: {msg}",
                    path.display(),
                    shared_pkg.package_id,
                    dep
                )
            })?;
            if let Some(prev) = previous_dependency {
                if dep.as_str() < prev {
                    return Err(format!(
                        "{subject} `{}` shared std package `{}` dependencies must be sorted by exact id",
                        path.display(),
                        shared_pkg.package_id
                    ));
                }
            }
            if !seen_dependencies.insert(dep.as_str()) {
                return Err(format!(
                    "{subject} `{}` shared std package `{}` has duplicate dependency id `{}`",
                    path.display(),
                    shared_pkg.package_id,
                    dep
                ));
            }
            previous_dependency = Some(dep.as_str());
        }

        let exact_id = format!("{}@{}", shared_pkg.package_id, shared_pkg.version);
        if let Some(prev) = previous_id.as_ref() {
            if exact_id < *prev {
                return Err(format!(
                    "{subject} `{}` shared std packages must be sorted by `package_id@version` (found `{}` before `{}`)",
                    path.display(),
                    prev,
                    exact_id
                ));
            }
        }
        previous_id = Some(exact_id.clone());
        if normal_package_ids.contains(exact_id.as_str()) {
            return Err(format!(
                "{subject} `{}` shared std package `{}` collides with normal package id space",
                path.display(),
                exact_id
            ));
        }
        if !seen_exact_ids.insert(exact_id) {
            return Err(format!(
                "{subject} `{}` has duplicate shared std package id `{}`",
                path.display(),
                format!("{}@{}", shared_pkg.package_id, shared_pkg.version)
            ));
        }
        validated.push(ValidatedSharedStdLockPackageV2 {
            package_id: shared_pkg.package_id,
            version: shared_pkg.version,
            verified_std_abi: SharedStdAbiClaim {
                major: shared_pkg.verified_std_abi.major,
                minor_min: shared_pkg.verified_std_abi.minor_min,
                minor_max: shared_pkg.verified_std_abi.minor_max,
            },
            artifact: ValidatedSharedStdArtifactV2 {
                format: shared_pkg.artifact.format,
                path: shared_pkg.artifact.path,
                digest: shared_pkg.artifact.digest,
                size_bytes: shared_pkg.artifact.size_bytes,
            },
            signature: ValidatedSharedStdSignatureV2 {
                key_id: shared_pkg.signature.key_id,
                algorithm: shared_pkg.signature.algorithm,
                signed_at: shared_pkg.signature.signed_at,
                signature: shared_pkg.signature.signature,
            },
            provenance: ValidatedSharedStdProvenanceV2 {
                statement_digest: shared_pkg.provenance.statement_digest,
                statement_format: shared_pkg.provenance.statement_format,
            },
            symbols: shared_pkg.symbols,
            dependencies: shared_pkg.dependencies,
        });
    }

    Ok(ValidatedSharedStdLockSectionV2 {
        delivery: raw.delivery,
        packages: validated,
    })
}

fn validate_exact_package_id(value: &str) -> Result<(), String> {
    let Some((name, version)) = value.rsplit_once('@') else {
        return Err("expected exact `package_id@version`".to_string());
    };
    validate_package_id(name)?;
    validate_exact_semver(version)
}

fn validate_relative_artifact_path(path: &str) -> Result<(), String> {
    if path.trim().is_empty() {
        return Err("path is empty".to_string());
    }
    let candidate = Path::new(path);
    if candidate.is_absolute() {
        return Err("path must be relative".to_string());
    }
    for component in candidate.components() {
        match component {
            Component::ParentDir => {
                return Err("path must not contain parent-directory traversal (`..`)".to_string())
            }
            Component::Prefix(_) | Component::RootDir => {
                return Err("path must be relative".to_string());
            }
            Component::CurDir | Component::Normal(_) => {}
        }
    }
    Ok(())
}

#[cfg(test)]
pub(crate) mod tests {
    use super::{parse_validated_shared_std_lock_section, project_shared_std_evidence};
    use serde_json::json;
    use std::path::Path;

    pub(crate) struct SharedStdMalformedCase {
        pub(crate) label: &'static str,
        pub(crate) value: serde_json::Value,
        pub(crate) expected_substring: &'static str,
    }

    pub(crate) fn valid_shared_std_package_value(
        package_id: &str,
        version: &str,
    ) -> serde_json::Value {
        json!({
            "package_id": package_id,
            "version": version,
            "verified_std_abi": {
                "major": 1,
                "minor_min": 0,
                "minor_max": 0
            },
            "artifact": {
                "format": "wasm",
                "path": format!("std-packages/{}-{}.wasm", package_id.replace("::", "-"), version),
                "digest": "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
                "size_bytes": 4
            },
            "signature": {
                "key_id": "std-publisher-ed25519-2026q2",
                "algorithm": "ed25519",
                "signed_at": "2026-06-01T00:00:00Z",
                "signature": "sig"
            },
            "provenance": {
                "statement_digest": "sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
                "statement_format": "in-toto-v1"
            },
            "symbols": [
                "std::bytes::eq_ct",
                "std::str::len"
            ],
            "dependencies": []
        })
    }

    pub(crate) fn valid_shared_std_lockfile_value() -> serde_json::Value {
        json!({
            "schema_version": 2,
            "resolver_version": 1,
            "roots": [],
            "packages": [],
            "std": {
                "delivery": "shared",
                "packages": [valid_shared_std_package_value("std::text", "1.2.0")]
            }
        })
    }

    pub(crate) fn malformed_shared_std_lockfile_cases() -> Vec<SharedStdMalformedCase> {
        let mut cases = Vec::new();

        let mut invalid_delivery = valid_shared_std_lockfile_value();
        invalid_delivery["std"]["delivery"] = json!("broken");
        cases.push(SharedStdMalformedCase {
            label: "invalid delivery",
            value: invalid_delivery,
            expected_substring: "invalid `std.delivery`",
        });

        let mut empty_shared_set = valid_shared_std_lockfile_value();
        empty_shared_set["std"]["packages"] = json!([]);
        cases.push(SharedStdMalformedCase {
            label: "empty shared package set",
            value: empty_shared_set,
            expected_substring: "`std.packages[]` is empty",
        });

        let mut duplicate_package_ids = valid_shared_std_lockfile_value();
        duplicate_package_ids["std"]["packages"] = json!([
            valid_shared_std_package_value("std::text", "1.2.0"),
            valid_shared_std_package_value("std::text", "1.2.0")
        ]);
        cases.push(SharedStdMalformedCase {
            label: "duplicate package ids",
            value: duplicate_package_ids,
            expected_substring: "duplicate shared std package id",
        });

        let mut unsorted_package_ids = valid_shared_std_lockfile_value();
        unsorted_package_ids["std"]["packages"] = json!([
            valid_shared_std_package_value("std::text", "1.3.0"),
            valid_shared_std_package_value("std::text", "1.2.0")
        ]);
        cases.push(SharedStdMalformedCase {
            label: "unsorted package ids",
            value: unsorted_package_ids,
            expected_substring: "shared std packages must be sorted",
        });

        let mut duplicate_symbols = valid_shared_std_lockfile_value();
        duplicate_symbols["std"]["packages"][0]["symbols"] =
            json!(["std::bytes::eq_ct", "std::bytes::eq_ct"]);
        cases.push(SharedStdMalformedCase {
            label: "duplicate symbols",
            value: duplicate_symbols,
            expected_substring: "duplicate symbol",
        });

        let mut unsorted_symbols = valid_shared_std_lockfile_value();
        unsorted_symbols["std"]["packages"][0]["symbols"] =
            json!(["std::str::len", "std::bytes::eq_ct"]);
        cases.push(SharedStdMalformedCase {
            label: "unsorted symbols",
            value: unsorted_symbols,
            expected_substring: "symbols must be sorted",
        });

        let mut duplicate_dependencies = valid_shared_std_lockfile_value();
        duplicate_dependencies["std"]["packages"][0]["dependencies"] =
            json!(["std::bytes@1.0.0", "std::bytes@1.0.0"]);
        cases.push(SharedStdMalformedCase {
            label: "duplicate dependencies",
            value: duplicate_dependencies,
            expected_substring: "duplicate dependency id",
        });

        let mut unsorted_dependencies = valid_shared_std_lockfile_value();
        unsorted_dependencies["std"]["packages"][0]["dependencies"] =
            json!(["std::zeta@1.0.0", "std::alpha@1.0.0"]);
        cases.push(SharedStdMalformedCase {
            label: "unsorted dependencies",
            value: unsorted_dependencies,
            expected_substring: "dependencies must be sorted by exact id",
        });

        let mut invalid_artifact_digest = valid_shared_std_lockfile_value();
        invalid_artifact_digest["std"]["packages"][0]["artifact"]["digest"] = json!("sha256:bad");
        cases.push(SharedStdMalformedCase {
            label: "invalid artifact digest",
            value: invalid_artifact_digest,
            expected_substring: "invalid artifact digest",
        });

        let mut invalid_provenance_digest = valid_shared_std_lockfile_value();
        invalid_provenance_digest["std"]["packages"][0]["provenance"]["statement_digest"] =
            json!("sha256:bad");
        cases.push(SharedStdMalformedCase {
            label: "invalid provenance digest",
            value: invalid_provenance_digest,
            expected_substring: "invalid provenance statement_digest",
        });

        let mut zero_artifact_size = valid_shared_std_lockfile_value();
        zero_artifact_size["std"]["packages"][0]["artifact"]["size_bytes"] = json!(0);
        cases.push(SharedStdMalformedCase {
            label: "zero artifact size",
            value: zero_artifact_size,
            expected_substring: "zero artifact size_bytes",
        });

        let mut incomplete_signature = valid_shared_std_lockfile_value();
        incomplete_signature["std"]["packages"][0]["signature"]["key_id"] = json!("");
        cases.push(SharedStdMalformedCase {
            label: "incomplete signature fields",
            value: incomplete_signature,
            expected_substring: "incomplete signature fields",
        });

        let mut empty_provenance_format = valid_shared_std_lockfile_value();
        empty_provenance_format["std"]["packages"][0]["provenance"]["statement_format"] = json!("");
        cases.push(SharedStdMalformedCase {
            label: "empty provenance statement format",
            value: empty_provenance_format,
            expected_substring: "empty provenance statement_format",
        });

        let mut normal_shared_collision = valid_shared_std_lockfile_value();
        normal_shared_collision["packages"] = json!([{ "id": "std::text@1.2.0" }]);
        cases.push(SharedStdMalformedCase {
            label: "normal shared collision",
            value: normal_shared_collision,
            expected_substring: "collides with normal package id space",
        });

        cases
    }

    #[test]
    fn parse_shared_std_lock_section_rejects_unsorted_symbols() {
        let mut value = valid_shared_std_lockfile_value();
        value["std"]["packages"][0]["symbols"] = json!(["std::str::len", "std::bytes::eq_ct"]);
        let err = parse_validated_shared_std_lock_section(
            &value,
            Path::new("clg.lock.json"),
            "shared std lockfile",
        )
        .expect_err("unsorted symbols should fail");
        assert!(err.contains("symbols must be sorted"));
    }

    #[test]
    fn project_shared_std_evidence_sorts_records() {
        let value = valid_shared_std_lockfile_value();
        let parsed = parse_validated_shared_std_lock_section(
            &value,
            Path::new("clg.lock.json"),
            "shared std lockfile",
        )
        .expect("valid section")
        .expect("schema v2");
        let projected = project_shared_std_evidence(&parsed);
        assert_eq!(projected.len(), 1);
        assert_eq!(projected[0].package_id, "std::text");
    }

    #[test]
    fn parse_shared_std_lock_section_rejects_shared_malformed_case_matrix() {
        for case in malformed_shared_std_lockfile_cases() {
            let err = parse_validated_shared_std_lock_section(
                &case.value,
                Path::new("clg.lock.json"),
                "shared std lockfile",
            )
            .expect_err("malformed shared std lockfile should fail");
            assert!(
                err.contains(case.expected_substring),
                "{}: expected `{}`, got `{}`",
                case.label,
                case.expected_substring,
                err
            );
        }
    }
}
