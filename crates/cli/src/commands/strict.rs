use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

use crate::commands::build;
use crate::commands::helpers::{make_single_json_error, CommandError};
use crate::commands::release_defaults::{
    load_required_release_defaults_v0, load_verify_trust_policy_v1,
    release_project_template_pretty_json, ReleaseDefaultsError, STRICT_PROJECT_FILE,
};
use crate::logging::{Logger, StageTimings};

const STRICT_LOCKFILE_FILE: &str = "clg.lock.json";
const STRICT_TRUST_POLICY_FILE: &str = "clg.trust-policy.json";
const STRICT_HOST_PROFILE_FILE: &str = "clg.host-profile.json";
const STRICT_PACKAGE_METADATA_FILE: &str = "clg.package-metadata.json";
const STRICT_PACKAGE_ABI_FILE: &str = "clg.package-abi.json";

const VERIFY_TRUST_POLICY_FILE: &str = "trust-policy.json";
const STRICT_INIT_DIAGNOSTIC_CODE: &str = "C131";

pub fn run_init(root: PathBuf, json_errors: bool, logger: Logger) -> Result<()> {
    let mut timings = StageTimings::new();
    let mut created_files = Vec::new();
    {
        let _stage = timings.start(logger, "strict_init_write_inputs");
        fs::create_dir_all(&root).with_context(|| format!("creating {}", root.display()))?;
        ensure_file(
            root.join(STRICT_PROJECT_FILE).as_path(),
            release_project_template_pretty_json().as_slice(),
            &mut created_files,
        )
        .map_err(|err| {
            strict_error(
                STRICT_INIT_DIAGNOSTIC_CODE,
                err.message().to_string(),
                root.as_path(),
                json_errors,
            )
        })?;
        ensure_file(
            root.join(STRICT_LOCKFILE_FILE).as_path(),
            minimal_strict_lockfile_bytes().as_slice(),
            &mut created_files,
        )
        .map_err(|err| {
            strict_error(
                STRICT_INIT_DIAGNOSTIC_CODE,
                err.message().to_string(),
                root.as_path(),
                json_errors,
            )
        })?;
        ensure_file(
            root.join(STRICT_TRUST_POLICY_FILE).as_path(),
            minimal_strict_trust_policy_bytes().as_slice(),
            &mut created_files,
        )
        .map_err(|err| {
            strict_error(
                STRICT_INIT_DIAGNOSTIC_CODE,
                err.message().to_string(),
                root.as_path(),
                json_errors,
            )
        })?;
        ensure_file(
            root.join(STRICT_HOST_PROFILE_FILE).as_path(),
            minimal_strict_host_profile_bytes().as_slice(),
            &mut created_files,
        )
        .map_err(|err| {
            strict_error(
                STRICT_INIT_DIAGNOSTIC_CODE,
                err.message().to_string(),
                root.as_path(),
                json_errors,
            )
        })?;
        ensure_file(
            root.join(STRICT_PACKAGE_METADATA_FILE).as_path(),
            minimal_package_metadata_bytes().as_slice(),
            &mut created_files,
        )
        .map_err(|err| {
            strict_error(
                STRICT_INIT_DIAGNOSTIC_CODE,
                err.message().to_string(),
                root.as_path(),
                json_errors,
            )
        })?;
        ensure_file(
            root.join(STRICT_PACKAGE_ABI_FILE).as_path(),
            minimal_package_abi_bytes().as_slice(),
            &mut created_files,
        )
        .map_err(|err| {
            strict_error(
                STRICT_INIT_DIAGNOSTIC_CODE,
                err.message().to_string(),
                root.as_path(),
                json_errors,
            )
        })?;
        ensure_file(
            root.join(VERIFY_TRUST_POLICY_FILE).as_path(),
            minimal_verify_trust_policy_bytes().as_slice(),
            &mut created_files,
        )
        .map_err(|err| {
            strict_error(
                STRICT_INIT_DIAGNOSTIC_CODE,
                err.message().to_string(),
                root.as_path(),
                json_errors,
            )
        })?;
    }

    {
        let _stage = timings.start(logger, "strict_init_validate_preflight");
        build::validate_required_strict_preflight_input_v0(root.as_path()).map_err(|err| {
            strict_error(
                err.code(),
                format!("{} (diagnostic {})", err.message(), err.code()),
                root.as_path(),
                json_errors,
            )
        })?;
    }

    let release_defaults = {
        let _stage = timings.start(logger, "strict_init_validate_release_defaults");
        load_required_release_defaults_v0(root.as_path()).map_err(|err| {
            strict_error(
                STRICT_INIT_DIAGNOSTIC_CODE,
                format!("{} (in {})", err.message(), STRICT_PROJECT_FILE),
                root.as_path(),
                json_errors,
            )
        })?
    };

    {
        let _stage = timings.start(logger, "strict_init_validate_verify_policy");
        let verify_path = root.join(release_defaults.trust_policy.as_str());
        load_verify_trust_policy_v1(verify_path.as_path()).map_err(|err| {
            strict_error(
                STRICT_INIT_DIAGNOSTIC_CODE,
                format!(
                    "{} (from release_defaults.trust_policy in {})",
                    err.message(),
                    STRICT_PROJECT_FILE
                ),
                root.as_path(),
                json_errors,
            )
        })?;
    }

    logger.summary(&timings);
    println!("strict init complete at {}", root.display());
    if created_files.is_empty() {
        println!("created: none");
    } else {
        println!("created: {}", created_files.join(", "));
    }
    println!(
        "validated: {}, {}, {}, {}, {}, {}, {}",
        STRICT_PROJECT_FILE,
        STRICT_LOCKFILE_FILE,
        STRICT_TRUST_POLICY_FILE,
        STRICT_HOST_PROFILE_FILE,
        STRICT_PACKAGE_METADATA_FILE,
        STRICT_PACKAGE_ABI_FILE,
        release_defaults.trust_policy
    );
    println!("release defaults loaded from {}", STRICT_PROJECT_FILE);
    println!("release command: clg release <FILE> --key <FILE> --pubkey <FILE>");
    Ok(())
}

fn ensure_file(
    path: &Path,
    bytes: &[u8],
    created_files: &mut Vec<String>,
) -> std::result::Result<(), ReleaseDefaultsError> {
    if path.exists() {
        if !path.is_file() {
            return Err(ReleaseDefaultsError::new(format!(
                "path `{}` exists but is not a file",
                path.display()
            )));
        }
        return Ok(());
    }
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent).map_err(|err| {
                ReleaseDefaultsError::new(format!(
                    "creating directory `{}`: {err}",
                    parent.display()
                ))
            })?;
        }
    }
    fs::write(path, bytes)
        .map_err(|err| ReleaseDefaultsError::new(format!("writing `{}`: {err}", path.display())))?;
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| path.display().to_string());
    created_files.push(name);
    Ok(())
}

fn minimal_strict_lockfile_bytes() -> Vec<u8> {
    serde_json::to_vec_pretty(&serde_json::json!({
        "schema_version": 1,
        "resolver_version": 1,
        "roots": [
            {
                "name": "app",
                "dependencies": []
            }
        ],
        "packages": []
    }))
    .expect("serialize strict lockfile template")
}

fn minimal_strict_trust_policy_bytes() -> Vec<u8> {
    serde_json::to_vec_pretty(&serde_json::json!({
        "schema_version": 0,
        "trusted_signers": [],
        "revoked_key_ids": []
    }))
    .expect("serialize strict trust policy template")
}

fn minimal_strict_host_profile_bytes() -> Vec<u8> {
    serde_json::to_vec_pretty(&serde_json::json!({
        "schema_version": 0,
        "profile": "contract_static",
        "capabilities": []
    }))
    .expect("serialize strict host profile template")
}

fn minimal_package_metadata_bytes() -> Vec<u8> {
    serde_json::to_vec_pretty(&serde_json::json!({
        "schema_version": 0,
        "packages": []
    }))
    .expect("serialize strict package metadata template")
}

fn minimal_package_abi_bytes() -> Vec<u8> {
    serde_json::to_vec_pretty(&serde_json::json!({
        "schema_version": 0,
        "contracts": []
    }))
    .expect("serialize strict package abi template")
}

fn minimal_verify_trust_policy_bytes() -> Vec<u8> {
    serde_json::to_vec_pretty(&serde_json::json!({
        "schema_version": 1,
        "trust_anchors": {
            "lean_checker": "4.14.0",
            "coq_checker": "8.19.2"
        }
    }))
    .expect("serialize verify trust policy template")
}

fn strict_error(
    code: &'static str,
    message: String,
    root: &Path,
    json_errors: bool,
) -> anyhow::Error {
    if json_errors {
        let json = make_single_json_error(code, "strict", message, root, 0, 0, None);
        return CommandError::json(json).into();
    }
    anyhow::anyhow!(message)
}
