pub(super) fn apply_solver_outcomes_if_configured(vcs: &mut [VerificationCondition]) -> Result<()> {
    if vcs.is_empty() {
        return Ok(());
    }
    let selected_backend = resolve_solver_backend_kind()?;
    let effective_backend = resolve_effective_solver_backend_kind(selected_backend)?;
    match effective_backend {
        SolverBackendKind::ExternalZ3Cli => apply_external_z3_cli_solver_outcomes(vcs),
        SolverBackendKind::RustZ3Lib => apply_rust_z3_lib_solver_outcomes(vcs),
    }
}

fn resolve_solver_backend_kind() -> Result<SolverBackendKind> {
    let raw = std::env::var(CLG_SOLVER_BACKEND_ENV).ok();
    select_solver_backend_kind(raw.as_deref(), cfg!(feature = "rust-z3-lib"))
}

fn select_solver_backend_kind(
    raw: Option<&str>,
    rust_z3_lib_enabled: bool,
) -> Result<SolverBackendKind> {
    let requested = raw.map(str::trim).filter(|value| !value.is_empty());
    match requested {
        None => Ok(SolverBackendKind::ExternalZ3Cli),
        Some(SOLVER_BACKEND_EXTERNAL_Z3_CLI) => Ok(SolverBackendKind::ExternalZ3Cli),
        Some(SOLVER_BACKEND_RUST_Z3_LIB) => {
            if rust_z3_lib_enabled {
                Ok(SolverBackendKind::RustZ3Lib)
            } else {
                anyhow::bail!(
                    "solver backend `{SOLVER_BACKEND_RUST_Z3_LIB}` requested via `{CLG_SOLVER_BACKEND_ENV}`, but this build does not enable feature `rust-z3-lib`"
                );
            }
        }
        Some(other) => anyhow::bail!(
            "unsupported solver backend `{other}` in `{CLG_SOLVER_BACKEND_ENV}` (supported: `{SOLVER_BACKEND_EXTERNAL_Z3_CLI}`, `{SOLVER_BACKEND_RUST_Z3_LIB}`)"
        ),
    }
}

fn effective_solver_backend_kind(
    selected: SolverBackendKind,
    rust_z3_cutover_enabled: bool,
) -> SolverBackendKind {
    match selected {
        SolverBackendKind::ExternalZ3Cli => SolverBackendKind::ExternalZ3Cli,
        SolverBackendKind::RustZ3Lib => {
            if rust_z3_cutover_enabled {
                SolverBackendKind::RustZ3Lib
            } else {
                SolverBackendKind::ExternalZ3Cli
            }
        }
    }
}

fn resolve_effective_solver_backend_kind(selected: SolverBackendKind) -> Result<SolverBackendKind> {
    match selected {
        SolverBackendKind::ExternalZ3Cli => Ok(SolverBackendKind::ExternalZ3Cli),
        SolverBackendKind::RustZ3Lib => Ok(effective_solver_backend_kind(
            SolverBackendKind::RustZ3Lib,
            rust_z3_lib_cutover_enabled()?,
        )),
    }
}

fn rust_z3_lib_cutover_enabled() -> Result<bool> {
    let raw = std::env::var(CLG_SOLVER_RUST_Z3_CUTOVER_ENV).ok();
    parse_bool_cutover_flag(raw.as_deref())
}

fn parse_bool_cutover_flag(raw: Option<&str>) -> Result<bool> {
    let Some(raw) = raw.map(str::trim).filter(|value| !value.is_empty()) else {
        return Ok(false);
    };
    let normalized = raw.to_ascii_lowercase();
    match normalized.as_str() {
        "1" | "true" | "on" | "yes" => Ok(true),
        "0" | "false" | "off" | "no" => Ok(false),
        other => anyhow::bail!(
            "invalid boolean value `{other}` for `{CLG_SOLVER_RUST_Z3_CUTOVER_ENV}` (supported: 1|0|true|false|on|off|yes|no)"
        ),
    }
}

fn apply_external_z3_cli_solver_outcomes(vcs: &mut [VerificationCondition]) -> Result<()> {
    let Some(resolved_solver) = resolve_solver_bin() else {
        return Ok(());
    };
    let solver_bin = resolved_solver.path;
    let profile = load_solver_runtime_profile()?;
    match solver_reports_pinned_version(solver_bin.as_path(), &profile)? {
        Some(true) => {}
        Some(false) => {
            anyhow::bail!(
                "configured solver `{}` does not match pinned {} solver_version `{}` from `docs/design/phase-25.1.4-solver-profile.lock.json`",
                solver_bin.display(),
                profile.solver_family,
                profile.solver_version
            );
        }
        None => {
            // Keep existing "generated" status when solver cannot be launched.
            return Ok(());
        }
    }
    let started = Instant::now();
    for vc in vcs {
        let elapsed_ms = started.elapsed().as_millis() as u64;
        if elapsed_ms >= profile.total_timeout_ms {
            vc.status = "timeout";
            continue;
        }
        let remaining_ms = profile.total_timeout_ms.saturating_sub(elapsed_ms);
        let per_vc_timeout_ms = profile.per_vc_timeout_ms.min(remaining_ms);
        match solver_outcome_for_vc(&solver_bin, &profile.options, per_vc_timeout_ms, vc) {
            Ok(outcome) => {
                vc.status = outcome.status;
                vc.counterexample = outcome.counterexample;
            }
            Err(SolverExecError::Unavailable) => {
                // Keep existing "generated" status when solver cannot be launched.
                return Ok(());
            }
            Err(SolverExecError::TimedOut) => {
                vc.status = "timeout";
            }
            Err(SolverExecError::TerminationFailed) => {
                anyhow::bail!("solver timeout termination failed");
            }
            Err(SolverExecError::InvocationFailed) => {
                vc.status = "unknown";
            }
        }
    }
    Ok(())
}

#[cfg(feature = "rust-z3-lib")]
fn apply_rust_z3_lib_solver_outcomes(vcs: &mut [VerificationCondition]) -> Result<()> {
    let profile = load_solver_runtime_profile()?;
    let version = rust_z3_full_version();
    if !version
        .to_ascii_lowercase()
        .contains(profile.solver_version.to_ascii_lowercase().as_str())
    {
        anyhow::bail!(
            "configured backend `{SOLVER_BACKEND_RUST_Z3_LIB}` reports version `{version}`, which does not match pinned solver_version `{}` from `docs/design/phase-25.1.4-solver-profile.lock.json`",
            profile.solver_version
        );
    }
    let started = Instant::now();
    for vc in vcs {
        let elapsed_ms = started.elapsed().as_millis() as u64;
        if elapsed_ms >= profile.total_timeout_ms {
            vc.status = "timeout";
            continue;
        }
        let remaining_ms = profile.total_timeout_ms.saturating_sub(elapsed_ms);
        let per_vc_timeout_ms = profile.per_vc_timeout_ms.min(remaining_ms);
        match rust_z3_lib_outcome_for_vc(&profile.options, per_vc_timeout_ms, vc) {
            Ok(outcome) => {
                vc.status = outcome.status;
                vc.counterexample = outcome.counterexample;
            }
            Err(RustZ3ExecError::TimedOut) => vc.status = "timeout",
            Err(RustZ3ExecError::InvocationFailed) => vc.status = "unknown",
        }
    }
    Ok(())
}

#[cfg(not(feature = "rust-z3-lib"))]
fn apply_rust_z3_lib_solver_outcomes(_vcs: &mut [VerificationCondition]) -> Result<()> {
    anyhow::bail!(
        "solver backend `{SOLVER_BACKEND_RUST_Z3_LIB}` is not enabled in this `clg` build"
    );
}
