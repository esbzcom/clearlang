fn load_solver_runtime_profile() -> Result<SolverRuntimeProfile> {
    let (profile, _hash) = load_solver_profile_claim()?;
    let solver_family = profile
        .get("solver_family")
        .and_then(|value| value.as_str())
        .ok_or_else(|| anyhow!("solver profile missing solver_family"))?
        .trim()
        .to_string();
    if solver_family != "z3" {
        anyhow::bail!("unsupported solver_family `{solver_family}` (expected `z3`)");
    }
    let solver_version = profile
        .get("solver_version")
        .and_then(|value| value.as_str())
        .ok_or_else(|| anyhow!("solver profile missing solver_version"))?
        .trim()
        .to_string();
    if solver_version.is_empty() {
        anyhow::bail!("solver profile solver_version cannot be empty");
    }
    let options = profile
        .get("options")
        .and_then(|value| value.as_array())
        .ok_or_else(|| anyhow!("solver profile missing options[]"))?
        .iter()
        .map(|entry| {
            let key = entry
                .get("key")
                .and_then(|value| value.as_str())
                .ok_or_else(|| anyhow!("solver option missing key"))?
                .trim()
                .to_string();
            let value = entry
                .get("value")
                .and_then(|value| value.as_str())
                .ok_or_else(|| anyhow!("solver option missing value"))?
                .trim()
                .to_string();
            if key.is_empty() || value.is_empty() {
                return Err(anyhow!("solver option key/value cannot be empty"));
            }
            Ok(SolverOption { key, value })
        })
        .collect::<Result<Vec<_>>>()?;
    let timeouts = profile
        .get("timeouts_ms")
        .and_then(|value| value.as_object())
        .ok_or_else(|| anyhow!("solver profile missing timeouts_ms object"))?;
    let per_vc_timeout_ms = timeouts
        .get("per_vc")
        .and_then(|value| value.as_u64())
        .ok_or_else(|| anyhow!("solver profile missing timeouts_ms.per_vc"))?;
    let total_timeout_ms = timeouts
        .get("total")
        .and_then(|value| value.as_u64())
        .ok_or_else(|| anyhow!("solver profile missing timeouts_ms.total"))?;
    if per_vc_timeout_ms == 0 || total_timeout_ms == 0 {
        return Err(anyhow!("solver profile timeouts must be greater than zero"));
    }
    Ok(SolverRuntimeProfile {
        solver_family,
        solver_version,
        options,
        per_vc_timeout_ms,
        total_timeout_ms,
    })
}

enum SolverExecError {
    Unavailable,
    TimedOut,
    TerminationFailed,
    InvocationFailed,
}

#[cfg(feature = "rust-z3-lib")]
enum RustZ3ExecError {
    TimedOut,
    InvocationFailed,
}
