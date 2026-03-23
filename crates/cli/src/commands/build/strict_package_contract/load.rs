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

