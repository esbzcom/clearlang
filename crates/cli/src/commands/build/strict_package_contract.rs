use std::collections::{BTreeSet, HashMap, HashSet};
use std::fs;
use std::path::Path;

use serde::Deserialize;

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
struct RawPackageMetadataRoot {
    schema_version: u32,
    packages: Vec<RawPackageMetadataEntry>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawPackageMetadataEntry {
    name: String,
    version: String,
    digest: String,
    artifact: RawPackageArtifact,
    abi_id: String,
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

pub(super) fn load_required_package_metadata_abi_v0(
    root: &Path,
) -> Result<StrictPackageContractV0, StrictPackageContractError> {
    let metadata_path = root.join(STRICT_PACKAGE_METADATA_FILE);
    let abi_path = root.join(STRICT_PACKAGE_ABI_FILE);

    if !metadata_path.exists() {
        return Err(StrictPackageContractError::new(
            "C104",
            format!(
                "strict mode requires `{}` at `{}`",
                STRICT_PACKAGE_METADATA_FILE,
                metadata_path.display()
            ),
        ));
    }
    if !metadata_path.is_file() {
        return Err(StrictPackageContractError::new(
            "C104",
            format!(
                "strict package metadata path `{}` exists but is not a file",
                metadata_path.display()
            ),
        ));
    }
    if !abi_path.exists() {
        return Err(StrictPackageContractError::new(
            "C104",
            format!(
                "strict mode requires `{}` at `{}`",
                STRICT_PACKAGE_ABI_FILE,
                abi_path.display()
            ),
        ));
    }
    if !abi_path.is_file() {
        return Err(StrictPackageContractError::new(
            "C104",
            format!(
                "strict package ABI path `{}` exists but is not a file",
                abi_path.display()
            ),
        ));
    }

    let metadata_content = fs::read_to_string(&metadata_path).map_err(|err| {
        StrictPackageContractError::new(
            "C104",
            format!(
                "failed to read strict package metadata `{}`: {err}",
                metadata_path.display()
            ),
        )
    })?;
    let abi_content = fs::read_to_string(&abi_path).map_err(|err| {
        StrictPackageContractError::new(
            "C104",
            format!(
                "failed to read strict package ABI `{}`: {err}",
                abi_path.display()
            ),
        )
    })?;

    parse_package_metadata_abi_v0(&metadata_content, &abi_content, &metadata_path, &abi_path)
}

fn parse_package_metadata_abi_v0(
    metadata_content: &str,
    abi_content: &str,
    metadata_path: &Path,
    abi_path: &Path,
) -> Result<StrictPackageContractV0, StrictPackageContractError> {
    let metadata_value: serde_json::Value =
        serde_json::from_str(metadata_content).map_err(|_| {
            StrictPackageContractError::new(
                "C104",
                format!(
                    "strict package metadata `{}` is not valid JSON (expected schema v0 object)",
                    metadata_path.display()
                ),
            )
        })?;
    let raw_metadata: RawPackageMetadataRoot =
        serde_json::from_value(metadata_value).map_err(|_| {
            StrictPackageContractError::new(
                "C104",
                format!(
                    "strict package metadata `{}` does not match schema v0 (`schema_version`, `packages[]`)",
                    metadata_path.display()
                ),
            )
        })?;
    if raw_metadata.schema_version != 0 {
        return Err(StrictPackageContractError::new(
            "C104",
            format!(
                "strict package metadata `{}` has unsupported schema_version {}; expected 0",
                metadata_path.display(),
                raw_metadata.schema_version
            ),
        ));
    }

    let abi_value: serde_json::Value = serde_json::from_str(abi_content).map_err(|_| {
        StrictPackageContractError::new(
            "C104",
            format!(
                "strict package ABI `{}` is not valid JSON (expected schema v0 object)",
                abi_path.display()
            ),
        )
    })?;
    let raw_abi: RawAbiRoot = serde_json::from_value(abi_value).map_err(|_| {
        StrictPackageContractError::new(
            "C104",
            format!(
                "strict package ABI `{}` does not match schema v0 (`schema_version`, `contracts[]`)",
                abi_path.display()
            ),
        )
    })?;
    if raw_abi.schema_version != 0 {
        return Err(StrictPackageContractError::new(
            "C104",
            format!(
                "strict package ABI `{}` has unsupported schema_version {}; expected 0",
                abi_path.display(),
                raw_abi.schema_version
            ),
        ));
    }

    let mut packages: Vec<StrictPackageMetadataEntry> = raw_metadata
        .packages
        .into_iter()
        .map(|pkg| StrictPackageMetadataEntry {
            name: pkg.name,
            version: pkg.version,
            digest: pkg.digest,
            artifact_format: pkg.artifact.format,
            artifact_path: pkg.artifact.path,
            abi_id: pkg.abi_id,
        })
        .collect();
    packages.sort_by(|a, b| a.name.cmp(&b.name));

    let mut package_names = HashSet::with_capacity(packages.len());
    let mut package_dupes = BTreeSet::new();
    for pkg in &packages {
        if !package_names.insert(pkg.name.clone()) {
            package_dupes.insert(pkg.name.clone());
        }
    }
    if let Some(dupe) = package_dupes.iter().next() {
        return Err(StrictPackageContractError::new(
            "C104",
            format!(
                "strict package metadata `{}` has duplicate package name `{}`",
                metadata_path.display(),
                dupe
            ),
        ));
    }

    for pkg in &packages {
        validate_package_name(&pkg.name).map_err(|msg| {
            StrictPackageContractError::new(
                "C104",
                format!(
                    "strict package metadata `{}` package `{}` is invalid: {msg}",
                    metadata_path.display(),
                    pkg.name
                ),
            )
        })?;
        validate_exact_semver(&pkg.version).map_err(|msg| {
            StrictPackageContractError::new(
                "C104",
                format!(
                    "strict package metadata `{}` package `{}` has invalid version `{}`: {msg}",
                    metadata_path.display(),
                    pkg.name,
                    pkg.version
                ),
            )
        })?;
        validate_sha256_digest(&pkg.digest).map_err(|msg| {
            StrictPackageContractError::new(
                "C104",
                format!(
                    "strict package metadata `{}` package `{}` has invalid digest `{}`: {msg}",
                    metadata_path.display(),
                    pkg.name,
                    pkg.digest
                ),
            )
        })?;
        if pkg.artifact_path.trim().is_empty() {
            return Err(StrictPackageContractError::new(
                "C104",
                format!(
                    "strict package metadata `{}` package `{}` has empty artifact path",
                    metadata_path.display(),
                    pkg.name
                ),
            ));
        }
        if pkg.artifact_format != "wasm" {
            return Err(StrictPackageContractError::new(
                "C104",
                format!(
                    "strict package metadata `{}` package `{}` has unsupported artifact format `{}`; expected `wasm`",
                    metadata_path.display(),
                    pkg.name,
                    pkg.artifact_format
                ),
            ));
        }
        let artifact_path = Path::new(&pkg.artifact_path);
        if artifact_path.is_absolute() {
            return Err(StrictPackageContractError::new(
                "C104",
                format!(
                    "strict package metadata `{}` package `{}` artifact path must be relative",
                    metadata_path.display(),
                    pkg.name
                ),
            ));
        }
        if pkg.abi_id.trim().is_empty() {
            return Err(StrictPackageContractError::new(
                "C104",
                format!(
                    "strict package metadata `{}` package `{}` has empty abi_id",
                    metadata_path.display(),
                    pkg.name
                ),
            ));
        }
    }
    let mut package_abi_ids = HashSet::with_capacity(packages.len());
    let mut duplicate_package_abi_ids = BTreeSet::new();
    for pkg in &packages {
        if !package_abi_ids.insert(pkg.abi_id.clone()) {
            duplicate_package_abi_ids.insert(pkg.abi_id.clone());
        }
    }
    if let Some(dupe) = duplicate_package_abi_ids.iter().next() {
        return Err(StrictPackageContractError::new(
            "C104",
            format!(
                "strict package metadata `{}` has duplicate abi_id `{}` across packages",
                metadata_path.display(),
                dupe
            ),
        ));
    }

    let mut contracts: Vec<StrictAbiContractEntry> = raw_abi
        .contracts
        .into_iter()
        .map(|contract| StrictAbiContractEntry {
            abi_id: contract.abi_id,
            package: contract.package,
            version: contract.version,
            imports: contract
                .imports
                .into_iter()
                .map(|entry| StrictAbiImportEntry {
                    symbol: entry.symbol,
                    effect: entry.effect,
                    params: entry.params,
                    ret: entry.ret,
                    capability: entry.capability,
                })
                .collect(),
        })
        .collect();
    contracts.sort_by(|a, b| a.abi_id.cmp(&b.abi_id));

    let mut contract_ids = HashSet::with_capacity(contracts.len());
    let mut contract_dupes = BTreeSet::new();
    for contract in &contracts {
        if !contract_ids.insert(contract.abi_id.clone()) {
            contract_dupes.insert(contract.abi_id.clone());
        }
    }
    if let Some(dupe) = contract_dupes.iter().next() {
        return Err(StrictPackageContractError::new(
            "C104",
            format!(
                "strict package ABI `{}` has duplicate abi_id `{}`",
                abi_path.display(),
                dupe
            ),
        ));
    }

    for contract in &mut contracts {
        if contract.abi_id.trim().is_empty() {
            return Err(StrictPackageContractError::new(
                "C104",
                format!(
                    "strict package ABI `{}` contains contract with empty abi_id",
                    abi_path.display()
                ),
            ));
        }
        validate_package_name(&contract.package).map_err(|msg| {
            StrictPackageContractError::new(
                "C104",
                format!(
                    "strict package ABI `{}` contract `{}` has invalid package `{}`: {msg}",
                    abi_path.display(),
                    contract.abi_id,
                    contract.package
                ),
            )
        })?;
        validate_exact_semver(&contract.version).map_err(|msg| {
            StrictPackageContractError::new(
                "C104",
                format!(
                    "strict package ABI `{}` contract `{}` has invalid version `{}`: {msg}",
                    abi_path.display(),
                    contract.abi_id,
                    contract.version
                ),
            )
        })?;

        contract.imports.sort_by(|a, b| a.symbol.cmp(&b.symbol));
        let mut seen_symbols = HashSet::with_capacity(contract.imports.len());
        let mut symbol_dupes = BTreeSet::new();
        for import in &contract.imports {
            if !seen_symbols.insert(import.symbol.clone()) {
                symbol_dupes.insert(import.symbol.clone());
            }
        }
        if let Some(dupe) = symbol_dupes.iter().next() {
            return Err(StrictPackageContractError::new(
                "C104",
                format!(
                    "strict package ABI `{}` contract `{}` has duplicate symbol `{}`",
                    abi_path.display(),
                    contract.abi_id,
                    dupe
                ),
            ));
        }

        for import in &contract.imports {
            if import.symbol.trim().is_empty() {
                return Err(StrictPackageContractError::new(
                    "C104",
                    format!(
                        "strict package ABI `{}` contract `{}` contains empty symbol name",
                        abi_path.display(),
                        contract.abi_id
                    ),
                ));
            }
            match import.effect.as_str() {
                "pure" | "mut" | "io" => {}
                _ => {
                    return Err(StrictPackageContractError::new(
                        "C104",
                        format!(
                            "strict package ABI `{}` contract `{}` symbol `{}` has unsupported effect `{}`",
                            abi_path.display(),
                            contract.abi_id,
                            import.symbol,
                            import.effect
                        ),
                    ));
                }
            }
            if import.ret.trim().is_empty() {
                return Err(StrictPackageContractError::new(
                    "C104",
                    format!(
                        "strict package ABI `{}` contract `{}` symbol `{}` has empty return type",
                        abi_path.display(),
                        contract.abi_id,
                        import.symbol
                    ),
                ));
            }
            for ty in &import.params {
                if ty.trim().is_empty() {
                    return Err(StrictPackageContractError::new(
                        "C104",
                        format!(
                            "strict package ABI `{}` contract `{}` symbol `{}` has empty parameter type",
                            abi_path.display(),
                            contract.abi_id,
                            import.symbol
                        ),
                    ));
                }
            }
            if let Some(capability) = import.capability.as_ref() {
                if capability.trim().is_empty() {
                    return Err(StrictPackageContractError::new(
                        "C104",
                        format!(
                            "strict package ABI `{}` contract `{}` symbol `{}` has empty capability",
                            abi_path.display(),
                            contract.abi_id,
                            import.symbol
                        ),
                    ));
                }
            }
        }
    }

    let metadata_by_abi: HashMap<&str, (&str, &str)> = packages
        .iter()
        .map(|pkg| {
            (
                pkg.abi_id.as_str(),
                (pkg.name.as_str(), pkg.version.as_str()),
            )
        })
        .collect();
    let abi_by_id: HashMap<&str, (&str, &str)> = contracts
        .iter()
        .map(|abi| {
            (
                abi.abi_id.as_str(),
                (abi.package.as_str(), abi.version.as_str()),
            )
        })
        .collect();

    let mut missing_contract_ids = BTreeSet::new();
    for pkg in &packages {
        if !abi_by_id.contains_key(pkg.abi_id.as_str()) {
            missing_contract_ids.insert(pkg.abi_id.clone());
        }
    }
    if let Some(missing) = missing_contract_ids.iter().next() {
        return Err(StrictPackageContractError::new(
            "C105",
            format!(
                "strict package metadata/ABI mismatch: abi_id `{}` is referenced in metadata but missing from ABI contracts",
                missing
            ),
        ));
    }

    let mut orphan_contract_ids = BTreeSet::new();
    for contract in &contracts {
        if !metadata_by_abi.contains_key(contract.abi_id.as_str()) {
            orphan_contract_ids.insert(contract.abi_id.clone());
        }
    }
    if let Some(orphan) = orphan_contract_ids.iter().next() {
        return Err(StrictPackageContractError::new(
            "C105",
            format!(
                "strict package metadata/ABI mismatch: abi_id `{}` exists in ABI contracts but is not referenced by metadata",
                orphan
            ),
        ));
    }

    let mut mismatched = BTreeSet::new();
    for (abi_id, (meta_pkg, meta_ver)) in &metadata_by_abi {
        let (abi_pkg, abi_ver) = abi_by_id.get(abi_id).expect("checked presence above");
        if meta_pkg != abi_pkg || meta_ver != abi_ver {
            mismatched.insert((*abi_id).to_string());
        }
    }
    if let Some(abi_id) = mismatched.iter().next() {
        return Err(StrictPackageContractError::new(
            "C105",
            format!(
                "strict package metadata/ABI mismatch: abi_id `{}` has inconsistent package/version between metadata and ABI contract",
                abi_id
            ),
        ));
    }

    Ok(StrictPackageContractV0 {
        packages,
        contracts,
    })
}

fn validate_package_name(name: &str) -> Result<(), String> {
    if name.trim().is_empty() {
        return Err("name is empty".to_string());
    }
    for segment in name.split("::") {
        validate_identifier_segment(segment)?;
    }
    Ok(())
}

fn validate_identifier_segment(segment: &str) -> Result<(), String> {
    if segment.is_empty() {
        return Err("contains empty `::` segment".to_string());
    }
    let mut chars = segment.chars();
    let first = chars.next().expect("segment non-empty");
    if !(first == '_' || first.is_ascii_alphabetic()) {
        return Err(format!(
            "segment `{segment}` must start with ASCII letter or `_`"
        ));
    }
    for ch in chars {
        if !(ch == '_' || ch.is_ascii_alphanumeric()) {
            return Err(format!(
                "segment `{segment}` contains invalid character `{ch}`"
            ));
        }
    }
    Ok(())
}

fn validate_exact_semver(version: &str) -> Result<(), String> {
    if version.contains('-') || version.contains('+') {
        return Err(
            "must use exact MAJOR.MINOR.PATCH without pre-release/build metadata".to_string(),
        );
    }
    if !version.chars().all(|ch| ch.is_ascii_digit() || ch == '.') {
        return Err("must use exact MAJOR.MINOR.PATCH".to_string());
    }
    let parts: Vec<&str> = version.split('.').collect();
    if parts.len() != 3 {
        return Err("must use exact MAJOR.MINOR.PATCH".to_string());
    }
    for part in parts {
        if part.is_empty() || !part.chars().all(|ch| ch.is_ascii_digit()) {
            return Err(format!("version segment `{part}` is not numeric"));
        }
    }
    Ok(())
}

fn validate_sha256_digest(digest: &str) -> Result<(), String> {
    const PREFIX: &str = "sha256:";
    if !digest.starts_with(PREFIX) {
        return Err("digest must start with `sha256:`".to_string());
    }
    let hex = &digest[PREFIX.len()..];
    if hex.len() != 64 {
        return Err("digest must have exactly 64 lowercase hex characters".to_string());
    }
    if !hex
        .chars()
        .all(|ch| ch.is_ascii_digit() || ('a'..='f').contains(&ch))
    {
        return Err("digest must be lowercase hex (`0-9`, `a-f`)".to_string());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn valid_metadata_json() -> String {
        r#"{
  "schema_version": 0,
  "packages": [
    {
      "name": "std::core",
      "version": "1.0.0",
      "digest": "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
      "artifact": { "format": "wasm", "path": "store/std-core-1.0.0.wasm" },
      "abi_id": "abi:std::core:1.0.0"
    }
  ]
}"#
        .to_string()
    }

    fn valid_abi_json() -> String {
        r#"{
  "schema_version": 0,
  "contracts": [
    {
      "abi_id": "abi:std::core:1.0.0",
      "package": "std::core",
      "version": "1.0.0",
      "imports": [
        {
          "symbol": "std::core::math::add",
          "effect": "pure",
          "params": ["Int", "Int"],
          "ret": "Int",
          "capability": null
        }
      ]
    }
  ]
}"#
        .to_string()
    }

    #[test]
    fn valid_inputs_parse_and_validate() {
        let parsed = parse_package_metadata_abi_v0(
            &valid_metadata_json(),
            &valid_abi_json(),
            Path::new("clg.package-metadata.json"),
            Path::new("clg.package-abi.json"),
        )
        .expect("valid strict package metadata/abi");
        assert_eq!(parsed.packages.len(), 1);
        assert_eq!(parsed.contracts.len(), 1);
        assert_eq!(parsed.packages[0].name, "std::core");
        assert_eq!(parsed.contracts[0].abi_id, "abi:std::core:1.0.0");
    }

    #[test]
    fn missing_metadata_file_reports_c104() {
        let tmp = tempdir().expect("tempdir");
        fs::write(tmp.path().join(STRICT_PACKAGE_ABI_FILE), valid_abi_json())
            .expect("write abi file");
        let err =
            load_required_package_metadata_abi_v0(tmp.path()).expect_err("expected missing file");
        assert_eq!(err.code(), "C104");
        assert!(err.message().contains(STRICT_PACKAGE_METADATA_FILE));
    }

    #[test]
    fn malformed_metadata_schema_reports_c104() {
        let bad_metadata = r#"{
  "schema_version": 0,
  "packages": [],
  "extra": true
}"#;
        let err = parse_package_metadata_abi_v0(
            bad_metadata,
            &valid_abi_json(),
            Path::new("clg.package-metadata.json"),
            Path::new("clg.package-abi.json"),
        )
        .expect_err("expected malformed metadata schema error");
        assert_eq!(err.code(), "C104");
        assert!(err.message().contains("does not match schema v0"));
    }

    #[test]
    fn abi_id_missing_from_contracts_reports_c105() {
        let abi = r#"{
  "schema_version": 0,
  "contracts": []
}"#;
        let err = parse_package_metadata_abi_v0(
            &valid_metadata_json(),
            abi,
            Path::new("clg.package-metadata.json"),
            Path::new("clg.package-abi.json"),
        )
        .expect_err("expected ABI mismatch");
        assert_eq!(err.code(), "C105");
        assert!(err.message().contains("missing from ABI contracts"));
    }

    #[test]
    fn package_version_mismatch_reports_c105() {
        let abi = r#"{
  "schema_version": 0,
  "contracts": [
    {
      "abi_id": "abi:std::core:1.0.0",
      "package": "std::core",
      "version": "1.0.1",
      "imports": []
    }
  ]
}"#;
        let err = parse_package_metadata_abi_v0(
            &valid_metadata_json(),
            abi,
            Path::new("clg.package-metadata.json"),
            Path::new("clg.package-abi.json"),
        )
        .expect_err("expected package/version mismatch");
        assert_eq!(err.code(), "C105");
        assert!(err.message().contains("inconsistent package/version"));
    }

    #[test]
    fn duplicate_symbols_report_c104() {
        let abi = r#"{
  "schema_version": 0,
  "contracts": [
    {
      "abi_id": "abi:std::core:1.0.0",
      "package": "std::core",
      "version": "1.0.0",
      "imports": [
        {
          "symbol": "s",
          "effect": "pure",
          "params": ["Int"],
          "ret": "Int",
          "capability": null
        },
        {
          "symbol": "s",
          "effect": "pure",
          "params": ["Int"],
          "ret": "Int",
          "capability": null
        }
      ]
    }
  ]
}"#;
        let err = parse_package_metadata_abi_v0(
            &valid_metadata_json(),
            abi,
            Path::new("clg.package-metadata.json"),
            Path::new("clg.package-abi.json"),
        )
        .expect_err("expected duplicate symbol error");
        assert_eq!(err.code(), "C104");
        assert!(err.message().contains("duplicate symbol"));
    }

    #[test]
    fn duplicate_metadata_abi_id_reports_c104() {
        let metadata = r#"{
  "schema_version": 0,
  "packages": [
    {
      "name": "std::core",
      "version": "1.0.0",
      "digest": "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
      "artifact": { "format": "wasm", "path": "store/std-core-1.0.0.wasm" },
      "abi_id": "abi:shared:1"
    },
    {
      "name": "std::math",
      "version": "1.0.0",
      "digest": "sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
      "artifact": { "format": "wasm", "path": "store/std-math-1.0.0.wasm" },
      "abi_id": "abi:shared:1"
    }
  ]
}"#;
        let abi = r#"{
  "schema_version": 0,
  "contracts": [
    {
      "abi_id": "abi:shared:1",
      "package": "std::core",
      "version": "1.0.0",
      "imports": []
    }
  ]
}"#;
        let err = parse_package_metadata_abi_v0(
            metadata,
            abi,
            Path::new("clg.package-metadata.json"),
            Path::new("clg.package-abi.json"),
        )
        .expect_err("expected duplicate metadata abi_id");
        assert_eq!(err.code(), "C104");
        assert!(err.message().contains("duplicate abi_id"));
    }
}
