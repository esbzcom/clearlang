fn parse_metadata_json(
    metadata_content: &str,
    metadata_path: &Path,
) -> Result<(serde_json::Value, u64), StrictPackageContractError> {
    let metadata_value: serde_json::Value = serde_json::from_str(metadata_content).map_err(|_| {
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
    Ok((metadata_value, schema_version))
}

fn parse_abi_json(abi_content: &str, abi_path: &Path) -> Result<RawAbiRoot, StrictPackageContractError> {
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
    Ok(raw_abi)
}
