use std::path::Path;

use super::strict_host_profile::{load_required_host_profile_v0, StrictHostProfileV0};
use super::strict_lockfile::{load_required_strict_lockfile_v0, StrictLockfileV0};
use super::strict_package_contract::{
    load_required_package_metadata_abi_v0, StrictPackageContractV0,
};
use super::strict_trust_policy::{load_required_trust_policy_v0, StrictTrustPolicyV0};

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct StrictPreflightInputV0 {
    pub(super) lockfile: StrictLockfileV0,
    pub(super) trust_policy: StrictTrustPolicyV0,
    pub(super) package_contract: StrictPackageContractV0,
    pub(super) host_profile: StrictHostProfileV0,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct StrictPreflightInputError {
    code: &'static str,
    message: String,
}

impl StrictPreflightInputError {
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

pub(super) fn load_required_strict_preflight_input_v0(
    root: &Path,
) -> Result<StrictPreflightInputV0, StrictPreflightInputError> {
    let lockfile = load_required_strict_lockfile_v0(root)
        .map_err(|err| StrictPreflightInputError::new(err.code(), err.message()))?;
    let trust_policy = load_required_trust_policy_v0(root)
        .map_err(|err| StrictPreflightInputError::new(err.code(), err.message()))?;
    let package_contract = load_required_package_metadata_abi_v0(root)
        .map_err(|err| StrictPreflightInputError::new(err.code(), err.message()))?;
    let host_profile = load_required_host_profile_v0(root)
        .map_err(|err| StrictPreflightInputError::new(err.code(), err.message()))?;
    Ok(StrictPreflightInputV0 {
        lockfile,
        trust_policy,
        package_contract,
        host_profile,
    })
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::tempdir;

    use super::*;

    fn write_minimal_lockfile(root: &Path) {
        fs::write(
            root.join("clg.lock.json"),
            r#"{"schema_version":0,"dependencies":[]}"#,
        )
        .expect("write lockfile");
    }

    fn write_minimal_trust_policy(root: &Path) {
        fs::write(
            root.join("clg.trust-policy.json"),
            r#"{
  "schema_version": 0,
  "trusted_signers": [],
  "revoked_key_ids": []
}"#,
        )
        .expect("write trust policy");
    }

    fn write_minimal_package_contract(root: &Path) {
        fs::write(
            root.join("clg.package-metadata.json"),
            r#"{
  "schema_version": 0,
  "packages": []
}"#,
        )
        .expect("write package metadata");
        fs::write(
            root.join("clg.package-abi.json"),
            r#"{
  "schema_version": 0,
  "contracts": []
}"#,
        )
        .expect("write package abi");
    }

    fn write_minimal_host_profile(root: &Path) {
        fs::write(
            root.join("clg.host-profile.json"),
            r#"{
  "schema_version": 0,
  "profile": "contract_static",
  "capabilities": []
}"#,
        )
        .expect("write host profile");
    }

    #[test]
    fn loads_all_strict_preflight_inputs_v0() {
        let tmp = tempdir().expect("tempdir");
        write_minimal_lockfile(tmp.path());
        write_minimal_trust_policy(tmp.path());
        write_minimal_package_contract(tmp.path());
        write_minimal_host_profile(tmp.path());

        let loaded =
            load_required_strict_preflight_input_v0(tmp.path()).expect("load strict preflight");
        assert!(loaded.lockfile.dependencies.is_empty());
        assert!(loaded.trust_policy.trusted_signers.is_empty());
        assert!(loaded.package_contract.packages.is_empty());
        assert_eq!(loaded.host_profile.profile, "contract_static");
    }

    #[test]
    fn missing_lockfile_bubbles_c101() {
        let tmp = tempdir().expect("tempdir");
        let err = load_required_strict_preflight_input_v0(tmp.path()).expect_err("missing lock");
        assert_eq!(err.code(), "C101");
    }

    #[test]
    fn missing_host_profile_bubbles_c106() {
        let tmp = tempdir().expect("tempdir");
        write_minimal_lockfile(tmp.path());
        write_minimal_trust_policy(tmp.path());
        write_minimal_package_contract(tmp.path());
        let err =
            load_required_strict_preflight_input_v0(tmp.path()).expect_err("missing host profile");
        assert_eq!(err.code(), "C106");
    }
}
