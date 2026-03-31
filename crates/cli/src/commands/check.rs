use std::path::{Path, PathBuf};

use anyhow::Result;
use clg_typer::{check_with_vcs_with_std_and_external, ExternalBuiltinSig, TyperError};

use crate::commands::build;
use crate::commands::helpers::{extract_function_name, make_single_json_error, CommandError};
use crate::commands::modules::load_program;
use crate::commands::release_defaults::{
    load_required_release_defaults_v0, load_verify_trust_policy_v1, STRICT_PROJECT_FILE,
};
use crate::logging::{Logger, StageTimings};

const CHECK_DEFAULTS_DIAGNOSTIC_CODE: &str = "C131";

pub fn run(file: PathBuf, root: Option<PathBuf>, json_errors: bool, logger: Logger) -> Result<()> {
    let root = root.unwrap_or_else(|| {
        file.parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| PathBuf::from("."))
    });

    let mut timings = StageTimings::new();
    {
        let _stage = timings.start(logger, "check_preflight");
        build::validate_required_strict_preflight_input_v0(root.as_path()).map_err(|err| {
            check_error(
                err.code(),
                format!("{} (strict preflight at {})", err.message(), root.display()),
                &file,
                json_errors,
            )
        })?;
    }

    {
        let _stage = timings.start(logger, "check_release_defaults");
        let release_defaults =
            load_required_release_defaults_v0(root.as_path()).map_err(|err| {
                check_error(
                    CHECK_DEFAULTS_DIAGNOSTIC_CODE,
                    format!("loading `{}`: {}", STRICT_PROJECT_FILE, err.message()),
                    &file,
                    json_errors,
                )
            })?;
        let trust_policy_path = root.join(release_defaults.trust_policy.as_str());
        load_verify_trust_policy_v1(trust_policy_path.as_path()).map_err(|err| {
            check_error(
                CHECK_DEFAULTS_DIAGNOSTIC_CODE,
                format!("trust policy check failed: {}", err.message()),
                &file,
                json_errors,
            )
        })?;
    }

    let loaded = {
        let _stage = timings.start(logger, "parse");
        load_program(&file, json_errors)?
    };

    {
        let _stage = timings.start(logger, "typecheck");
        let external_sigs: Vec<ExternalBuiltinSig> = loaded
            .external_imports
            .iter()
            .map(|binding| ExternalBuiltinSig {
                name: binding.function.clone(),
                params: binding.params.clone(),
                ret: binding.ret.clone(),
                effect: binding.effect,
            })
            .collect();
        if let Err(err) = check_with_vcs_with_std_and_external(
            &loaded.program,
            &loaded.std_types,
            external_sigs.as_slice(),
        ) {
            if json_errors {
                let json = if let Some((typer, function)) = find_typer_error(&err) {
                    make_single_json_error(
                        typer.code,
                        "type",
                        typer.message.clone(),
                        &file,
                        typer.start,
                        typer.end,
                        function,
                    )
                } else {
                    make_single_json_error("T000", "type", format!("{err:#}"), &file, 0, 0, None)
                };
                return Err(CommandError::json(json).into());
            }
            return Err(anyhow::anyhow!("check type-check failed: {err}"));
        }
    }

    logger.summary(&timings);
    println!("check ok: {}", file.display());
    Ok(())
}

fn check_error(
    code: &'static str,
    message: String,
    file: &Path,
    json_errors: bool,
) -> anyhow::Error {
    if json_errors {
        let json = make_single_json_error(code, "check", message, file, 0, 0, None);
        return CommandError::json(json).into();
    }
    anyhow::anyhow!(message)
}

fn find_typer_error(err: &anyhow::Error) -> Option<(&TyperError, Option<String>)> {
    let mut current: Option<&(dyn std::error::Error + 'static)> = Some(err.as_ref());
    while let Some(cause) = current {
        if let Some(typer) = cause.downcast_ref::<TyperError>() {
            let function = extract_function_name(typer.message.as_str());
            return Some((typer, function));
        }
        current = cause.source();
    }
    None
}
