#[cfg(feature = "rust-z3-lib")]
fn rust_z3_lib_outcome_for_vc(
    options: &[SolverOption],
    per_vc_timeout_ms: u64,
    vc: &VerificationCondition,
) -> std::result::Result<SolverOutcome, RustZ3ExecError> {
    let (prelude, body) = split_vc_formula(&vc.vc_smt2);
    let mut script = String::new();
    if !prelude.is_empty() {
        script.push_str(prelude.as_str());
        script.push('\n');
    }
    script.push_str(format!("(assert (not {}))\n", body).as_str());

    let mut cfg = Z3Config::new();
    let timeout_ms_u64 = per_vc_timeout_ms.min(u64::from(u32::MAX));
    cfg.set_timeout_msec(timeout_ms_u64);
    with_z3_config(&cfg, || {
        let timeout_ms = timeout_ms_u64 as u32;
        let solver = Z3Solver::new();
        let mut params = Z3Params::new();
        params.set_u32("timeout", timeout_ms);
        apply_rust_z3_params(options, &mut params);
        solver.set_params(&params);
        solver.from_string(script.as_str());

        match solver.check() {
            Z3SatResult::Unsat => Ok(SolverOutcome {
                status: "proved",
                counterexample: None,
            }),
            Z3SatResult::Sat => Ok(SolverOutcome {
                status: "failed",
                counterexample: solver.get_model().map(|model| model.to_string()),
            }),
            Z3SatResult::Unknown => {
                let reason = solver.get_reason_unknown().unwrap_or_default();
                if reason.to_ascii_lowercase().contains("timeout") {
                    Err(RustZ3ExecError::TimedOut)
                } else {
                    Ok(SolverOutcome {
                        status: "unknown",
                        counterexample: None,
                    })
                }
            }
        }
    })
}

#[cfg(feature = "rust-z3-lib")]
fn apply_rust_z3_params(options: &[SolverOption], params: &mut Z3Params) {
    for option in options {
        if option.value.eq_ignore_ascii_case("true") {
            params.set_bool(option.key.as_str(), true);
            continue;
        }
        if option.value.eq_ignore_ascii_case("false") {
            params.set_bool(option.key.as_str(), false);
            continue;
        }
        if let Ok(value) = option.value.parse::<u32>() {
            params.set_u32(option.key.as_str(), value);
            continue;
        }
        if let Ok(value) = option.value.parse::<f64>() {
            params.set_f64(option.key.as_str(), value);
            continue;
        }
        params.set_symbol(option.key.as_str(), option.value.as_str());
    }
}

#[cfg(feature = "rust-z3-lib")]
fn rust_z3_full_version() -> String {
    z3::full_version().to_string()
}
