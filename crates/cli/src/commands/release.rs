use std::path::{Path, PathBuf};

use anyhow::Result;
use serde::Serialize;

use crate::commands::helpers::{make_single_json_error, CommandError};
use crate::logging::Logger;

#[derive(Debug, Clone, Serialize)]
struct ReleaseShape {
    schema_version: u32,
    policy_version: &'static str,
    primary_commands: [&'static str; 3],
    entry: String,
    root: String,
    advisory_as_of: String,
    key: String,
    key_id: String,
    pubkey: String,
    trust_policy: String,
    out_dir: String,
    artifacts: ReleaseArtifacts,
    orchestration: Vec<&'static str>,
}

#[derive(Debug, Clone, Serialize)]
struct ReleaseArtifacts {
    module: String,
    vcs: String,
    proof: String,
    signature: String,
    assurance_manifest: String,
    bundle_manifest: String,
}

#[allow(clippy::too_many_arguments)]
pub fn run(
    file: PathBuf,
    advisory_as_of: String,
    key: PathBuf,
    key_id: String,
    pubkey: PathBuf,
    root: Option<PathBuf>,
    out_dir: Option<PathBuf>,
    trust_policy: Option<PathBuf>,
    json_errors: bool,
    _logger: Logger,
) -> Result<()> {
    let root = root.unwrap_or_else(|| {
        file.parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| PathBuf::from("."))
    });
    let out_dir = out_dir.unwrap_or_else(|| root.join("out").join("release"));
    let trust_policy = trust_policy.unwrap_or_else(|| root.join("clg.trust-policy.json"));
    let stem = file_stem_or_error(file.as_path(), json_errors)?;

    let shape = ReleaseShape {
        schema_version: 1,
        policy_version: "25.2.2",
        primary_commands: ["check", "test", "release"],
        entry: file.display().to_string(),
        root: root.display().to_string(),
        advisory_as_of,
        key: key.display().to_string(),
        key_id,
        pubkey: pubkey.display().to_string(),
        trust_policy: trust_policy.display().to_string(),
        out_dir: out_dir.display().to_string(),
        artifacts: ReleaseArtifacts {
            module: out_dir.join(format!("{stem}.wasm")).display().to_string(),
            vcs: out_dir
                .join(format!("{stem}.vc.json"))
                .display()
                .to_string(),
            proof: out_dir
                .join(format!("{stem}.proof.json"))
                .display()
                .to_string(),
            signature: out_dir
                .join(format!("{stem}.sig.json"))
                .display()
                .to_string(),
            assurance_manifest: out_dir
                .join(format!("{stem}.assurance.json"))
                .display()
                .to_string(),
            bundle_manifest: out_dir
                .join(format!("{stem}.release-bundle.json"))
                .display()
                .to_string(),
        },
        orchestration: vec![
            "lock",
            "build/prove",
            "sign",
            "verify(require-assurance=proved_all)",
            "bundle",
        ],
    };

    println!(
        "{}",
        serde_json::to_string_pretty(&shape).expect("serialize release shape")
    );
    Ok(())
}

fn file_stem_or_error(file: &Path, json_errors: bool) -> Result<String> {
    let stem = file
        .file_stem()
        .and_then(|stem| stem.to_str())
        .map(str::trim)
        .filter(|stem| !stem.is_empty())
        .map(ToOwned::to_owned);
    match stem {
        Some(value) => Ok(value),
        None => {
            if json_errors {
                let json = make_single_json_error(
                    "C130",
                    "release",
                    format!(
                        "could not derive release artifact stem from entry path `{}`",
                        file.display()
                    ),
                    file,
                    0,
                    0,
                    None,
                );
                Err(CommandError::json(json).into())
            } else {
                anyhow::bail!(
                    "could not derive release artifact stem from entry path `{}`",
                    file.display()
                );
            }
        }
    }
}
