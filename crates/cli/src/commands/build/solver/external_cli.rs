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
    // Close stdin so solver can finish on bounded scripts and avoid deadlocks.
    child.stdin.take();

    let deadline = Instant::now()
        .checked_add(Duration::from_millis(per_vc_timeout_ms.max(1)))
        .unwrap_or_else(Instant::now);
    loop {
        if let Some(_status) = child
            .try_wait()
            .map_err(|_| SolverExecError::InvocationFailed)?
        {
            break;
        }
        if Instant::now() >= deadline {
            child.kill().map_err(|_| SolverExecError::TerminationFailed)?;
            child.wait().map_err(|_| SolverExecError::TerminationFailed)?;
            return Err(SolverExecError::TimedOut);
        }
        sleep(Duration::from_millis(5));
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
