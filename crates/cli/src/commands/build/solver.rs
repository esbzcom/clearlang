use std::process::{Command, Stdio};
use std::thread::sleep;
use std::time::Duration;
use std::time::Instant;

use clg_typer::VerificationCondition;
#[cfg(feature = "rust-z3-lib")]
use z3::{
    with_z3_config, Config as Z3Config, Params as Z3Params, SatResult as Z3SatResult,
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

include!("solver/backend.rs");
include!("solver/discovery.rs");
include!("solver/integrity.rs");
include!("solver/external_cli.rs");
include!("solver/profile.rs");
#[cfg(feature = "rust-z3-lib")]
include!("solver/rust_z3.rs");
