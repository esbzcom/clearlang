use std::process::{Command, Stdio};
use std::time::Instant;

use clg_typer::VerificationCondition;

const CLG_SOLVER_BIN_ENV: &str = "CLG_SOLVER_BIN";

#[derive(Debug, Clone)]
struct SolverOption {
    key: String,
    value: String,
}

#[derive(Debug, Clone)]
struct SolverRuntimeProfile {
    solver_family: String,
    solver_version: String,
    options: Vec<SolverOption>,
    per_vc_timeout_ms: u64,
    total_timeout_ms: u64,
}

pub(super) fn apply_solver_outcomes_if_configured(vcs: &mut [VerificationCondition]) -> Result<()> {
    let Some(solver_bin) = std::env::var_os(CLG_SOLVER_BIN_ENV) else {
        return Ok(());
    };
    if vcs.is_empty() {
        return Ok(());
    }
    let solver_bin = std::path::PathBuf::from(solver_bin);
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
            Ok(status) => vc.status = status,
            Err(SolverExecError::Unavailable) => {
                // Keep existing "generated" status when solver cannot be launched.
                return Ok(());
            }
            Err(SolverExecError::InvocationFailed) => {
                vc.status = "unknown";
            }
        }
    }
    Ok(())
}

fn solver_reports_pinned_version(
    solver_bin: &std::path::Path,
    profile: &SolverRuntimeProfile,
) -> Result<Option<bool>> {
    for flag in ["--version", "-version"] {
        let output = match Command::new(solver_bin).arg(flag).output() {
            Ok(output) => output,
            Err(err) => {
                if err.kind() == std::io::ErrorKind::NotFound {
                    return Ok(None);
                }
                continue;
            }
        };
        let mut text = String::new();
        text.push_str(String::from_utf8_lossy(output.stdout.as_slice()).as_ref());
        text.push_str(String::from_utf8_lossy(output.stderr.as_slice()).as_ref());
        if text.trim().is_empty() {
            continue;
        }
        let lower = text.to_ascii_lowercase();
        return Ok(Some(lower.contains(profile.solver_version.to_ascii_lowercase().as_str())));
    }
    anyhow::bail!(
        "failed to read configured solver `{}` version output",
        solver_bin.display()
    );
}

fn solver_outcome_for_vc(
    solver_bin: &std::path::Path,
    options: &[SolverOption],
    per_vc_timeout_ms: u64,
    vc: &VerificationCondition,
) -> std::result::Result<&'static str, SolverExecError> {
    let (prelude, body) = split_vc_formula(&vc.vc_smt2);
    let mut check_script = String::new();
    append_solver_script_prelude(&mut check_script, options, per_vc_timeout_ms);
    if !prelude.is_empty() {
        check_script.push_str(prelude.as_str());
        check_script.push('\n');
    }
    check_script.push_str(format!("(assert (not {}))\n(check-sat)\n", body).as_str());
    let check_output = run_solver(solver_bin, check_script.as_str(), per_vc_timeout_ms)?;
    let status = parse_check_sat_status(check_output.as_str());
    match status {
        Some("unsat") => Ok("proved"),
        Some("sat") => Ok("failed"),
        Some("unknown") => {
            let mut reason_script = String::new();
            append_solver_script_prelude(&mut reason_script, options, per_vc_timeout_ms);
            if !prelude.is_empty() {
                reason_script.push_str(prelude.as_str());
                reason_script.push('\n');
            }
            reason_script.push_str(
                format!("(assert (not {}))\n(check-sat)\n(get-info :reason-unknown)\n", body)
                    .as_str(),
            );
            let reason_output = run_solver(solver_bin, reason_script.as_str(), per_vc_timeout_ms)?;
            if parse_reason_is_timeout(reason_output.as_str()) {
                Ok("timeout")
            } else {
                Ok("unknown")
            }
        }
        _ => Ok("unknown"),
    }
}

fn split_vc_formula(vc_smt2: &str) -> (String, String) {
    let mut lines: Vec<&str> = vc_smt2
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect();
    if lines.is_empty() {
        return (String::new(), "true".to_string());
    }
    let body = lines.pop().unwrap_or("true").to_string();
    (lines.join("\n"), body)
}

fn append_solver_script_prelude(
    out: &mut String,
    options: &[SolverOption],
    per_vc_timeout_ms: u64,
) {
    out.push_str("(set-option :print-success false)\n");
    out.push_str(format!("(set-option :timeout {})\n", per_vc_timeout_ms).as_str());
    for option in options {
        out.push_str(format!("(set-option :{} {})\n", option.key, option.value).as_str());
    }
}

fn parse_check_sat_status(output: &str) -> Option<&'static str> {
    for line in output.lines() {
        match line.trim() {
            "unsat" => return Some("unsat"),
            "sat" => return Some("sat"),
            "unknown" => return Some("unknown"),
            _ => {}
        }
    }
    None
}

fn parse_reason_is_timeout(output: &str) -> bool {
    output.to_ascii_lowercase().contains("timeout")
}

fn run_solver(
    solver_bin: &std::path::Path,
    script: &str,
    per_vc_timeout_ms: u64,
) -> std::result::Result<String, SolverExecError> {
    let timeout_seconds = std::cmp::max(1, per_vc_timeout_ms.div_ceil(1000));
    let mut child = Command::new(solver_bin)
        .arg("-in")
        .arg("-smt2")
        .arg(format!("-T:{timeout_seconds}"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|err| match err.kind() {
            std::io::ErrorKind::NotFound => SolverExecError::Unavailable,
            _ => SolverExecError::InvocationFailed,
        })?;
    {
        let stdin = child
            .stdin
            .as_mut()
            .ok_or(SolverExecError::InvocationFailed)?;
        use std::io::Write;
        if let Err(err) = stdin.write_all(script.as_bytes()) {
            if err.kind() != std::io::ErrorKind::BrokenPipe {
                return Err(SolverExecError::InvocationFailed);
            }
        }
    }
    let output = child
        .wait_with_output()
        .map_err(|_| SolverExecError::InvocationFailed)?;
    if output.status.success() {
        Ok(String::from_utf8_lossy(output.stdout.as_slice()).to_string())
    } else {
        let stderr = String::from_utf8_lossy(output.stderr.as_slice());
        let _ = stderr;
        Err(SolverExecError::InvocationFailed)
    }
}

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
    InvocationFailed,
}
