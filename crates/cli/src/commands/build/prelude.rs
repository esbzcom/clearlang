use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{anyhow, Context, Result};
use clg_ast::{Effect, Param, ParamKind, Program, Type};
use clg_codegen_wasm::{
    emit_from_ir_with_opts, CodegenOpts, ExportAlias, ExternalImport,
    StdCoreLinkMode as WasmStdCoreLinkMode,
};
use clg_ir::{IrType, Module as IrModule};
use clg_typer::{
    check_with_vcs_with_std_and_external, ExternalBuiltinSig, TypecheckOutput, TyperError,
};
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;

use crate::commands::helpers::{
    canonical_json_bytes, extract_function_name, make_single_json_error, sha256_hex, CommandError,
    JsonError, JsonErrorItem,
};
use crate::commands::modules::load_program;
use crate::logging::{LogLevel, Logger, StageTimings};
use crate::proofs::{
    build_assurance_manifest_payload, bundle_symbols_for_program, hash_module,
    load_proved_surface_allowlist, load_solver_profile_claim, module_bytes_with_zeroed_hash,
    proof_matrix_path_from_env, proof_status_for_vcs, write_proof_artifact_json,
    AssuranceManifestInput, ProofArtifactEmission, ProofPackage, PROOF_STATUS_PROVED_ALL,
};
use crate::signing::{self, SignScope};

mod strict;
mod strict_host_profile;
mod strict_lockfile;
mod strict_package_contract;
mod strict_package_signatures;
mod strict_preflight_input;
pub(crate) mod strict_trust_policy;
mod strict_validation;
mod contract_state_schema;
mod vcs_json;

use strict::{
    proof_strict_for_mode, release_crypto_boundary_violation, strict_l3_claim_violation,
    strict_language_profile_violation, strict_proof_violation,
};
use strict_host_profile::StrictHostProfileV0;
use strict_lockfile::StrictLockfileV0;
use strict_package_contract::StrictPackageContractV0;
use strict_package_signatures::enforce_trust_gate_v0;
use strict_preflight_input::load_required_strict_preflight_input_v0;
use vcs_json::write_vcs_json;
use contract_state_schema::{check_contract_state_schema_compatibility, write_contract_state_schema};

const LEGACY_PACKAGE_METADATA_FILE: &str = "clg-packages.json";

#[derive(Clone, Copy, Debug, Eq, PartialEq, clap::ValueEnum)]
pub enum CompilerMode {
    /// Development mode: VC strictness defaults to false unless explicitly enabled.
    Permissive,
    /// Current default behavior: VC strictness defaults to true.
    Standard,
    /// Production mode: requires --emit-vcs and enforces VC strictness.
    Strict,
}

impl CompilerMode {
    fn as_str(self) -> &'static str {
        match self {
            CompilerMode::Permissive => "permissive",
            CompilerMode::Standard => "standard",
            CompilerMode::Strict => "strict",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, clap::ValueEnum)]
pub enum ReleaseProfile {
    /// Development compile flow: theorem-grade release gate is not enforced.
    Dev,
    /// Production compile flow: fail-closed gate requires theorem-grade proof status.
    Production,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, clap::ValueEnum)]
pub enum StdCoreLinkMode {
    /// Keep using intrinsic lowering for std-core-capable symbols.
    Intrinsic,
    /// Activate strict precompiled std-core link contract path.
    Precompiled,
}

impl From<StdCoreLinkMode> for WasmStdCoreLinkMode {
    fn from(value: StdCoreLinkMode) -> Self {
        match value {
            StdCoreLinkMode::Intrinsic => WasmStdCoreLinkMode::Intrinsic,
            StdCoreLinkMode::Precompiled => WasmStdCoreLinkMode::Precompiled,
        }
    }
}

