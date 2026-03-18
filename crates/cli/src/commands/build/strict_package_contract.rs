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
    pub(super) signature: Option<StrictPackageMetadataSignature>,
    pub(super) trusted_anchor_ids: Vec<String>,
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
    signature: RawPackageSignatureV1,
    trust: RawPackageTrustV1,
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
                    "strict package metadata `{}` is not valid JSON (expected schema v0/v1 object)",
                    metadata_path.display()
                ),
            )
        })?;
    let schema_version = metadata_value
        .get("schema_version")
        .and_then(|v| v.as_u64())
        .ok_or_else(|| {
            StrictPackageContractError::new(
                "C104",
                format!(
                    "strict package metadata `{}` does not match schema v0/v1 (`schema_version`, `packages[]`)",
                    metadata_path.display()
                ),
            )
        })?;

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

    let mut packages: Vec<StrictPackageMetadataEntry> = match schema_version {
        0 => {
            let raw_metadata: RawPackageMetadataRootV0 = serde_json::from_value(metadata_value)
                .map_err(|_| {
                    StrictPackageContractError::new(
                        "C104",
                        format!(
                            "strict package metadata `{}` does not match schema v0 (`schema_version`, `packages[]`)",
                            metadata_path.display()
                        ),
                    )
                })?;
            debug_assert_eq!(raw_metadata.schema_version, 0);
            raw_metadata
                .packages
                .into_iter()
                .map(|pkg| StrictPackageMetadataEntry {
                    name: pkg.name,
                    version: pkg.version,
                    digest: pkg.digest,
                    artifact_format: pkg.artifact.format,
                    artifact_path: pkg.artifact.path,
                    abi_id: pkg.abi_id,
                    signature: None,
                    trusted_anchor_ids: Vec::new(),
                })
                .collect()
        }
        1 => {
            let raw_metadata: RawPackageMetadataRootV1 = serde_json::from_value(metadata_value)
                .map_err(|_| {
                    StrictPackageContractError::new(
                        "C104",
                        format!(
                            "strict package metadata `{}` does not match schema v1 (`schema_version`, `packages[]` with `signature` + `trust`)",
                            metadata_path.display()
                        ),
                    )
                })?;
            debug_assert_eq!(raw_metadata.schema_version, 1);
            raw_metadata
                .packages
                .into_iter()
                .map(|pkg| StrictPackageMetadataEntry {
                    name: pkg.name,
                    version: pkg.version,
                    digest: pkg.digest,
                    artifact_format: pkg.artifact.format,
                    artifact_path: pkg.artifact.path,
                    abi_id: pkg.abi_id,
                    signature: Some(StrictPackageMetadataSignature {
                        format: pkg.signature.format,
                        key_id: pkg.signature.key_id,
                        signed_at: pkg.signature.signed_at,
                        signature: pkg.signature.signature,
                    }),
                    trusted_anchor_ids: pkg.trust.trusted_anchor_ids,
                })
                .collect()
        }
        other => {
            return Err(StrictPackageContractError::new(
                "C104",
                format!(
                    "strict package metadata `{}` has unsupported schema_version {}; expected 0 or 1",
                    metadata_path.display(),
                    other
                ),
            ));
        }
    };
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
        if let Some(signature) = pkg.signature.as_ref() {
            if pkg.trusted_anchor_ids.is_empty() {
                return Err(StrictPackageContractError::new(
                    "C104",
                    format!(
                        "strict package metadata `{}` package `{}` must declare at least one trusted anchor id when signature metadata is present",
                        metadata_path.display(),
                        pkg.name
                    ),
                ));
            }
            if signature.format != "ed25519" {
                return Err(StrictPackageContractError::new(
                    "C104",
                    format!(
                        "strict package metadata `{}` package `{}` has unsupported signature format `{}`; expected `ed25519`",
                        metadata_path.display(),
                        pkg.name,
                        signature.format
                    ),
                ));
            }
            if signature.key_id.trim().is_empty() {
                return Err(StrictPackageContractError::new(
                    "C104",
                    format!(
                        "strict package metadata `{}` package `{}` has empty signature key_id",
                        metadata_path.display(),
                        pkg.name
                    ),
                ));
            }
            validate_utc_rfc3339("signed_at", &signature.signed_at).map_err(|msg| {
                StrictPackageContractError::new(
                    "C104",
                    format!(
                        "strict package metadata `{}` package `{}` has invalid signature signed_at `{}`: {msg}",
                        metadata_path.display(),
                        pkg.name,
                        signature.signed_at
                    ),
                )
            })?;
            validate_signature_hex(&signature.signature).map_err(|msg| {
                StrictPackageContractError::new(
                    "C104",
                    format!(
                        "strict package metadata `{}` package `{}` has invalid signature `{}`: {msg}",
                        metadata_path.display(),
                        pkg.name,
                        signature.signature
                    ),
                )
            })?;
        }
        if !pkg.trusted_anchor_ids.is_empty() {
            let mut seen_anchors = HashSet::new();
            let mut anchor_dupes = BTreeSet::new();
            for anchor_id in &pkg.trusted_anchor_ids {
                if anchor_id.trim().is_empty() {
                    return Err(StrictPackageContractError::new(
                        "C104",
                        format!(
                            "strict package metadata `{}` package `{}` has empty trusted anchor id",
                            metadata_path.display(),
                            pkg.name
                        ),
                    ));
                }
                if !seen_anchors.insert(anchor_id.clone()) {
                    anchor_dupes.insert(anchor_id.clone());
                }
            }
            if let Some(dupe) = anchor_dupes.iter().next() {
                return Err(StrictPackageContractError::new(
                    "C104",
                    format!(
                        "strict package metadata `{}` package `{}` has duplicate trusted anchor id `{}`",
                        metadata_path.display(),
                        pkg.name,
                        dupe
                    ),
                ));
            }
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

fn validate_signature_hex(signature: &str) -> Result<(), String> {
    if signature.len() != 128 {
        return Err("signature must have exactly 128 lowercase hex characters".to_string());
    }
    if !signature
        .chars()
        .all(|ch| ch.is_ascii_digit() || ('a'..='f').contains(&ch))
    {
        return Err("signature must be lowercase hex (`0-9`, `a-f`)".to_string());
    }
    Ok(())
}

fn validate_utc_rfc3339(field: &str, value: &str) -> Result<(), String> {
    if value.len() != 20 {
        return Err(format!("{field} must use `YYYY-MM-DDTHH:MM:SSZ`"));
    }
    let bytes = value.as_bytes();
    if bytes[4] != b'-'
        || bytes[7] != b'-'
        || bytes[10] != b'T'
        || bytes[13] != b':'
        || bytes[16] != b':'
        || bytes[19] != b'Z'
    {
        return Err(format!("{field} must use UTC RFC3339 separators"));
    }

    fn parse_u16(s: &str) -> Result<u16, String> {
        if !s.chars().all(|ch| ch.is_ascii_digit()) {
            return Err(format!("`{s}` contains non-digit characters"));
        }
        s.parse::<u16>()
            .map_err(|_| format!("failed to parse `{s}`"))
    }
    fn parse_u8(s: &str) -> Result<u8, String> {
        if !s.chars().all(|ch| ch.is_ascii_digit()) {
            return Err(format!("`{s}` contains non-digit characters"));
        }
        s.parse::<u8>()
            .map_err(|_| format!("failed to parse `{s}`"))
    }

    let year = parse_u16(&value[0..4])?;
    let month = parse_u8(&value[5..7])?;
    let day = parse_u8(&value[8..10])?;
    let hour = parse_u8(&value[11..13])?;
    let minute = parse_u8(&value[14..16])?;
    let second = parse_u8(&value[17..19])?;

    if !(1..=12).contains(&month) {
        return Err(format!("month `{month}` is out of range 1..=12"));
    }
    if !(1..=31).contains(&day) {
        return Err(format!("day `{day}` is out of range 1..=31"));
    }
    let max_day = days_in_month(year, month);
    if day > max_day {
        return Err(format!(
            "day `{day}` is out of range 1..={max_day} for month `{month}`"
        ));
    }
    if hour > 23 {
        return Err(format!("hour `{hour}` is out of range 0..=23"));
    }
    if minute > 59 {
        return Err(format!("minute `{minute}` is out of range 0..=59"));
    }
    if second > 59 {
        return Err(format!("second `{second}` is out of range 0..=59"));
    }
    Ok(())
}

fn days_in_month(year: u16, month: u8) -> u8 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if is_leap_year(year) => 29,
        2 => 28,
        _ => 0,
    }
}

fn is_leap_year(year: u16) -> bool {
    (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
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

    fn valid_metadata_json_v1() -> String {
        r#"{
  "schema_version": 1,
  "packages": [
    {
      "name": "std::core",
      "version": "1.0.0",
      "digest": "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
      "artifact": { "format": "wasm", "path": "store/std-core-1.0.0.wasm" },
      "abi_id": "abi:std::core:1.0.0",
      "signature": {
        "format": "ed25519",
        "key_id": "k1",
        "signed_at": "2026-01-15T00:00:00Z",
        "signature": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
      },
      "trust": {
        "trusted_anchor_ids": ["k1"]
      }
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
    fn schema_v1_metadata_with_signature_and_trust_parses_and_validates() {
        let parsed = parse_package_metadata_abi_v0(
            &valid_metadata_json_v1(),
            &valid_abi_json(),
            Path::new("clg.package-metadata.json"),
            Path::new("clg.package-abi.json"),
        )
        .expect("valid strict package metadata/abi v1");
        assert_eq!(parsed.packages.len(), 1);
        let pkg = &parsed.packages[0];
        let sig = pkg.signature.as_ref().expect("signature metadata");
        assert_eq!(sig.key_id, "k1");
        assert_eq!(pkg.trusted_anchor_ids, vec!["k1"]);
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
    fn metadata_v1_requires_trusted_anchors_when_signature_present() {
        let metadata = r#"{
  "schema_version": 1,
  "packages": [
    {
      "name": "std::core",
      "version": "1.0.0",
      "digest": "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
      "artifact": { "format": "wasm", "path": "store/std-core-1.0.0.wasm" },
      "abi_id": "abi:std::core:1.0.0",
      "signature": {
        "format": "ed25519",
        "key_id": "k1",
        "signed_at": "2026-01-15T00:00:00Z",
        "signature": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
      },
      "trust": { "trusted_anchor_ids": [] }
    }
  ]
}"#;
        let err = parse_package_metadata_abi_v0(
            metadata,
            &valid_abi_json(),
            Path::new("clg.package-metadata.json"),
            Path::new("clg.package-abi.json"),
        )
        .expect_err("expected trusted-anchor validation error");
        assert_eq!(err.code(), "C104");
        assert!(err.message().contains("at least one trusted anchor id"));
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
