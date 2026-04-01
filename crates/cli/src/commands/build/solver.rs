#[cfg(feature = "rust-z3-lib")]
use std::ffi::CStr;
use std::process::{Command, Stdio};
use std::thread::sleep;
use std::time::Duration;
use std::time::Instant;

use clg_typer::VerificationCondition;
#[cfg(feature = "rust-z3-lib")]
use z3::{
    Config as Z3Config, Context as Z3Context, Params as Z3Params, SatResult as Z3SatResult,
    Solver as Z3Solver,
};

const CLG_SOLVER_BIN_ENV: &str = "CLG_SOLVER_BIN";
const CLG_SOLVER_BUNDLE_ROOT_ENV: &str = "CLG_SOLVER_BUNDLE_ROOT";
const CLG_SOLVER_BACKEND_ENV: &str = "CLG_SOLVER_BACKEND";
const CLG_SOLVER_RUST_Z3_CUTOVER_ENV: &str = "CLG_SOLVER_RUST_Z3_CUTOVER";
const SOLVER_BACKEND_EXTERNAL_Z3_CLI: &str = "external-z3-cli";
const SOLVER_BACKEND_RUST_Z3_LIB: &str = "rust-z3-lib";
const SOLVER_SUPPLY_CHAIN_LOCK_JSON: &str =
    include_str!("../../../../../docs/design/phase-25.1.16-solver-supply-chain.lock.json");

#[derive(Debug, Clone)]
struct ResolvedSolverBin {
    path: PathBuf,
}

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SolverBackendKind {
    ExternalZ3Cli,
    RustZ3Lib,
}

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
            Ok(status) => vc.status = status,
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
            Ok(status) => vc.status = status,
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

fn resolve_solver_bin() -> Option<ResolvedSolverBin> {
    if let Some(configured) = configured_solver_bin_from_env() {
        return verify_solver_integrity(configured).map(|path| ResolvedSolverBin { path });
    }
    if let Some(from_bundle_env) = solver_bin_from_bundle_root_env() {
        return verify_solver_integrity(from_bundle_env).map(|path| ResolvedSolverBin { path });
    }
    resolver_default_bundle_candidates()
}

fn configured_solver_bin_from_env() -> Option<PathBuf> {
    let value = std::env::var_os(CLG_SOLVER_BIN_ENV)?;
    if value.is_empty() {
        return None;
    }
    Some(PathBuf::from(value))
}

fn solver_bin_from_bundle_root_env() -> Option<PathBuf> {
    let root = std::env::var_os(CLG_SOLVER_BUNDLE_ROOT_ENV)?;
    if root.is_empty() {
        return None;
    }
    let candidate = PathBuf::from(root).join(solver_bundle_relative_path());
    candidate.is_file().then_some(candidate)
}

fn resolver_default_bundle_candidates() -> Option<ResolvedSolverBin> {
    let relative = solver_bundle_relative_path();
    if let Ok(current_dir) = std::env::current_dir() {
        if let Some(path) = find_solver_in_ancestor_layouts(current_dir.as_path(), relative.as_path()) {
            if let Some(path) = verify_solver_integrity(path) {
                return Some(ResolvedSolverBin { path });
            }
        }
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(exe_dir) = exe.parent() {
            if let Some(path) = find_solver_in_ancestor_layouts(exe_dir, relative.as_path()) {
                if let Some(path) = verify_solver_integrity(path) {
                    return Some(ResolvedSolverBin { path });
                }
            }
            let packaged = exe_dir.join("solver").join(relative.as_path());
            if let Some(path) = verify_solver_integrity(packaged) {
                return Some(ResolvedSolverBin { path });
            }
        }
    }
    None
}

fn find_solver_in_ancestor_layouts(
    start: &std::path::Path,
    relative: &std::path::Path,
) -> Option<PathBuf> {
    let solver_name = relative.file_name()?.to_str()?;
    for ancestor in start.ancestors().take(8) {
        let candidate = ancestor.join("tools").join("proof").join("z3").join(relative);
        if candidate.is_file() {
            return Some(candidate);
        }
        if let Some(extracted) = find_solver_in_extracted_bundle_layout(ancestor, solver_name) {
            return Some(extracted);
        }
    }
    None
}

fn find_solver_in_extracted_bundle_layout(
    ancestor: &std::path::Path,
    solver_name: &str,
) -> Option<PathBuf> {
    let extract_root = ancestor
        .join("tools")
        .join("proof")
        .join("z3")
        .join("z3-extract");
    let entries = std::fs::read_dir(extract_root).ok()?;
    let mut dirs = entries
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| path.is_dir())
        .collect::<Vec<_>>();
    // Prefer highest semantic-like version sequence when multiple extracted bundles exist.
    dirs.sort_by(|lhs, rhs| {
        extracted_bundle_version_key(rhs)
            .cmp(&extracted_bundle_version_key(lhs))
            .then_with(|| rhs.to_string_lossy().cmp(&lhs.to_string_lossy()))
    });
    for dir in dirs {
        let candidate = dir.join("bin").join(solver_name);
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    None
}

fn extracted_bundle_version_key(path: &std::path::Path) -> Vec<u32> {
    let name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or_default();
    let mut nums = Vec::new();
    let mut cur = String::new();
    for ch in name.chars() {
        if ch.is_ascii_digit() {
            cur.push(ch);
        } else if !cur.is_empty() {
            nums.push(cur.parse::<u32>().unwrap_or(0));
            cur.clear();
        }
    }
    if !cur.is_empty() {
        nums.push(cur.parse::<u32>().unwrap_or(0));
    }
    nums
}

fn solver_bundle_relative_path() -> PathBuf {
    let platform = if cfg!(target_os = "windows") {
        "windows"
    } else if cfg!(target_os = "macos") {
        "macos"
    } else {
        "linux"
    };
    if cfg!(target_os = "windows") {
        PathBuf::from(platform).join("z3.exe")
    } else {
        PathBuf::from(platform).join("z3")
    }
}

fn verify_solver_integrity(solver_bin: PathBuf) -> Option<PathBuf> {
    if !solver_bin.is_file() {
        return None;
    }
    let checksum_path = sidecar_path(solver_bin.as_path(), "sha256");
    let signature_path = sidecar_path(solver_bin.as_path(), "sig");

    let checksum_entry = fs::read_to_string(&checksum_path).ok()?;
    let checksum_entry = checksum_entry
        .lines()
        .next()
        .map(str::trim)
        .filter(|line| !line.is_empty())?;
    if !is_sha256_prefixed_hex(checksum_entry) {
        return None;
    }

    let bytes = fs::read(&solver_bin).ok()?;
    let computed_digest = format!("sha256:{}", sha256_hex(bytes.as_slice()));
    if checksum_entry != computed_digest {
        return None;
    }

    let signature_entry: SolverBundleSignatureV1 =
        serde_json::from_slice(fs::read(&signature_path).ok()?.as_slice()).ok()?;
    if signature_entry.schema_version != 1 {
        return None;
    }
    if signature_entry.scheme != "ed25519" {
        return None;
    }
    if !is_sha256_prefixed_hex(signature_entry.signed_payload.as_str()) {
        return None;
    }
    if signature_entry.signed_payload != checksum_entry {
        return None;
    }
    let trusted_signer = load_solver_signature_policy()?
        .into_iter()
        .find(|signer| signer.key_id == signature_entry.key_id)?;
    let signature_bytes = decode_signature_hex(signature_entry.signature.as_str())?;
    let signature = ed25519_dalek::Signature::from_bytes(&signature_bytes);
    if ed25519_dalek::Verifier::verify(
        &trusted_signer.verifying_key,
        signature_entry.signed_payload.as_bytes(),
        &signature,
    )
    .is_err()
    {
        return None;
    }

    Some(solver_bin)
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct SolverBundleSignatureV1 {
    schema_version: u32,
    key_id: String,
    scheme: String,
    signed_payload: String,
    signature: String,
}

#[derive(Debug, Clone)]
struct TrustedSolverSigner {
    key_id: String,
    verifying_key: ed25519_dalek::VerifyingKey,
}

fn load_solver_signature_policy() -> Option<Vec<TrustedSolverSigner>> {
    let lock: serde_json::Value = serde_json::from_str(SOLVER_SUPPLY_CHAIN_LOCK_JSON).ok()?;
    if lock
        .get("schema_version")
        .and_then(|value| value.as_u64())
        .unwrap_or_default()
        != 1
    {
        return None;
    }
    let bundle_integrity = lock.get("bundle_integrity")?.as_object()?;
    if bundle_integrity
        .get("signature_required")
        .and_then(|value| value.as_bool())
        != Some(true)
    {
        return None;
    }
    if bundle_integrity
        .get("signature_mode")
        .and_then(|value| value.as_str())
        != Some("publisher-auth-ed25519-v1")
    {
        return None;
    }
    if bundle_integrity
        .get("signature_schema_version")
        .and_then(|value| value.as_u64())
        != Some(1)
    {
        return None;
    }
    let allowed_statuses = bundle_integrity
        .get("rotation_policy")?
        .get("allowed_statuses")?
        .as_array()?;
    let allowed_statuses = allowed_statuses
        .iter()
        .filter_map(|entry| entry.as_str().map(ToString::to_string))
        .collect::<std::collections::BTreeSet<_>>();
    if allowed_statuses.is_empty() {
        return None;
    }
    let min_signers = bundle_integrity
        .get("rotation_policy")?
        .get("min_trusted_signers")?
        .as_u64()
        .unwrap_or(1);

    let mut trusted = Vec::new();
    for entry in bundle_integrity.get("trusted_signers")?.as_array()? {
        let key_id = entry.get("key_id")?.as_str()?.trim().to_string();
        let scheme = entry.get("scheme")?.as_str()?.trim();
        let status = entry.get("status")?.as_str()?.trim().to_string();
        let public_key = entry.get("public_key")?.as_str()?.trim();
        if key_id.is_empty() || scheme != "ed25519" || !allowed_statuses.contains(status.as_str())
        {
            continue;
        }
        let public_key_bytes: [u8; 32] = decode_hex_prefixed(public_key, 32)?.try_into().ok()?;
        let verifying_key = ed25519_dalek::VerifyingKey::from_bytes(&public_key_bytes).ok()?;
        trusted.push(TrustedSolverSigner {
            key_id,
            verifying_key,
        });
    }
    if trusted.len() < min_signers as usize {
        return None;
    }
    Some(trusted)
}

fn decode_hex_prefixed(value: &str, expected_len: usize) -> Option<Vec<u8>> {
    let hex = value.strip_prefix("hex:")?;
    let bytes = hex::decode(hex).ok()?;
    (bytes.len() == expected_len).then_some(bytes)
}

fn decode_signature_hex(value: &str) -> Option<[u8; 64]> {
    let raw = hex::decode(value).ok()?;
    let bytes: [u8; 64] = raw.try_into().ok()?;
    Some(bytes)
}

fn sidecar_path(solver_bin: &std::path::Path, suffix: &str) -> PathBuf {
    let mut name = solver_bin.file_name().unwrap_or_default().to_os_string();
    name.push(format!(".{suffix}"));
    solver_bin
        .parent()
        .unwrap_or_else(|| std::path::Path::new("."))
        .join(name)
}

fn is_sha256_prefixed_hex(value: &str) -> bool {
    let Some(hex) = value.strip_prefix("sha256:") else {
        return false;
    };
    hex.len() == 64 && hex.bytes().all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
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

#[cfg(feature = "rust-z3-lib")]
fn rust_z3_lib_outcome_for_vc(
    options: &[SolverOption],
    per_vc_timeout_ms: u64,
    vc: &VerificationCondition,
) -> std::result::Result<&'static str, RustZ3ExecError> {
    let (prelude, body) = split_vc_formula(&vc.vc_smt2);
    let mut script = String::new();
    if !prelude.is_empty() {
        script.push_str(prelude.as_str());
        script.push('\n');
    }
    script.push_str(format!("(assert (not {}))\n", body).as_str());

    let mut cfg = Z3Config::new();
    let timeout_ms = per_vc_timeout_ms.min(u64::from(u32::MAX)) as u32;
    cfg.set_timeout_msec(timeout_ms);
    let ctx = Z3Context::new(&cfg);
    let solver = Z3Solver::new(&ctx);
    let mut params = Z3Params::new(&ctx);
    params.set_u32("timeout", timeout_ms);
    apply_rust_z3_params(options, &mut params);
    solver.set_params(&params);
    solver.from_string(script.as_str());

    match solver.check() {
        Z3SatResult::Unsat => Ok("proved"),
        Z3SatResult::Sat => Ok("failed"),
        Z3SatResult::Unknown => {
            let reason = solver.get_reason_unknown().unwrap_or_default();
            if reason.to_ascii_lowercase().contains("timeout") {
                Err(RustZ3ExecError::TimedOut)
            } else {
                Ok("unknown")
            }
        }
    }
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
    let version_ptr = unsafe { z3_sys::Z3_get_full_version() };
    if version_ptr.is_null() {
        return "<unknown>".to_string();
    }
    unsafe { CStr::from_ptr(version_ptr) }
        .to_string_lossy()
        .to_string()
}
