fn emit_std_binding_map_artifact(
    out_dir: &Path,
    binding_map: &StdBindingMapLockFile,
) -> Result<(), String> {
    fs::create_dir_all(out_dir).map_err(|e| format!("create `{}`: {e}", out_dir.display()))?;
    let json_path = out_dir.join("std-binding-map-v1.json");
    let sha_path = out_dir.join("std-binding-map-v1.sha256");
    let bytes = pretty_json_bytes(binding_map)?;
    fs::write(&json_path, &bytes).map_err(|e| format!("write `{}`: {e}", json_path.display()))?;
    let digest = format!("sha256:{}", hex::encode(Sha256::digest(&bytes)));
    fs::write(&sha_path, format!("{digest}\n"))
        .map_err(|e| format!("write `{}`: {e}", sha_path.display()))?;
    Ok(())
}

fn pretty_json_bytes<T: Serialize>(value: &T) -> Result<Vec<u8>, String> {
    let mut bytes = serde_json::to_vec_pretty(value).map_err(|e| format!("serialize json: {e}"))?;
    bytes.push(b'\n');
    Ok(bytes)
}

fn minimal_wasm_module_bytes() -> Vec<u8> {
    vec![0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00]
}

fn select_core_modules(mut modules: Vec<StdMetadataModule>) -> Vec<StdMetadataModule> {
    const CORE_MODULES: [&str; 9] = [
        "std::str",
        "std::bytes",
        "std::u64",
        "std::u128",
        "std::u256",
        "std::array",
        "std::slice",
        "std::list",
        "std::set",
        // `std::map` included through filter below.
    ];
    modules
        .retain(|module| module.path == "std::map" || CORE_MODULES.contains(&module.path.as_str()));
    modules.sort_by(|a, b| a.path.cmp(&b.path));
    for module in &mut modules {
        module
            .exports
            .sort_by(|a, b| a.name.cmp(&b.name).then_with(|| a.kind.cmp(&b.kind)));
    }
    modules
}

fn core_modules_to_abi_imports(modules: &[StdMetadataModule]) -> Vec<StrictAbiImport> {
    let mut imports = Vec::new();
    for module in modules {
        for export in &module.exports {
            if export.kind != "value" {
                continue;
            }
            imports.push(StrictAbiImport {
                symbol: format!("{}::{}", module.path, export.name),
                effect: if export.name.ends_with("_mut") {
                    "mut".to_string()
                } else {
                    "pure".to_string()
                },
                params: Vec::new(),
                ret: "Unknown".to_string(),
                capability: None,
            });
        }
    }
    imports.sort_by(|a, b| a.symbol.cmp(&b.symbol));
    imports
}

fn write_json_pretty<T: Serialize>(path: &Path, value: &T) -> Result<(), String> {
    let bytes = serde_json::to_vec_pretty(value)
        .map_err(|e| format!("serialize `{}`: {e}", path.display()))?;
    fs::write(path, bytes).map_err(|e| format!("write `{}`: {e}", path.display()))
}

fn run(cmd: &mut Command) -> Result<(), String> {
    let program = cmd.get_program().to_os_string();
    let args = cmd
        .get_args()
        .map(|arg| arg.to_os_string())
        .collect::<Vec<_>>();
    let display = display_cmd(&program, &args);
    let status = cmd
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()
        .map_err(|e| format!("failed to run {display}: {e}"))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!(
            "command {display} failed with status {:?}",
            status.code()
        ))
    }
}

fn display_cmd(program: &OsStr, args: &[std::ffi::OsString]) -> String {
    let mut line = program.to_string_lossy().to_string();
    for arg in args {
        line.push(' ');
        line.push_str(&arg.to_string_lossy());
    }
    line
}

fn print_help() {
    println!("xtask commands:");
    println!("  fmt        - cargo fmt --all");
    println!("  clippy     - cargo clippy --workspace --all-targets -- -D warnings");
    println!("  test       - cargo test --workspace");
    println!(
        "  release-precheck - cargo fmt --check + clippy -D warnings + test + clg test schema gate"
    );
    println!("  validate   - build samples and run wasm-tools validate");
    println!("  emit-vcs   - build a small contract sample with --emit-vcs");
    println!("  std-core-artifact [--version X.Y.Z] [--out-dir DIR]");
    println!(
        "  shared-std-publish --package-id std::text|std::int|std::sequence|std::codec|std::contract [--version X.Y.Z] --signed-at RFC3339Z [--out-dir DIR] [--registry-dir DIR] [--key-id ID] [--trusted-anchor-id ID] [--statement-format FORMAT] [--abi-major N] [--abi-minor-min N] [--abi-minor-max N]"
    );
    println!("  solver-vendor-stage --from PATH [--platform windows|linux|macos] [--key-id ID]");
    println!("  binary-repro-witness [--out FILE] (phase 25.6 binary reproducibility witness)");
    println!(
        "  milestone3-binary-bundle [--platform windows|linux|macos] [--binary FILE] [--out-dir DIR] [--key-id ID]"
    );
    println!("  milestone3-binary-bundle-verify --bundle-dir DIR");
    println!(
        "  manifest-lock-drift-check [--path DIR] (phase 25.4 manifest/lock consistency gate)"
    );
    println!(
        "  std-surface-drift-check [--emit-artifact DIR] [--refresh-lock] (phase 21 drift gate)"
    );
    println!(
        "  std-arch-sync [--write] [--refresh-lock] [--emit-artifact DIR] (phase 26.6/27.2 canonical std catalog, classification, verified ABI, and external-package plan sync)"
    );
    println!(
        "  std-arch-conformance-check (phase 26.6 doc/signature/lowering/runtime parity gate)"
    );
    println!(
        "  std-first-production-readiness-check (phase 26 must-have std closure audit)"
    );
    println!(
        "  shared-std-distribution-check (phase 28 shared std release/provenance/verify-bundle activation gate)"
    );
    println!(
        "  host-capability-policy-artifact [--emit-artifact DIR] [--refresh-lock] (phase 21 host policy gate)"
    );
    println!("  milestone2-perf-gate [--portability-smoke|--self-test]");
    println!("  milestone2-supply-chain-gate [--self-test] [--out-dir DIR]");
    println!("  ci         - release-precheck + build --release + validate");
}

#[derive(Clone, Debug)]
struct StdCoreArtifactOpts {
    version: String,
    out_dir: Option<PathBuf>,
}

#[derive(Clone, Debug)]
struct SharedStdPublishOpts {
    package_id: String,
    version: String,
    out_dir: Option<PathBuf>,
    registry_dir: Option<PathBuf>,
    key_id: String,
    trusted_anchor_id: String,
    signed_at: String,
    statement_format: String,
    abi_major: u32,
    abi_minor_min: u32,
    abi_minor_max: u32,
}

#[derive(Clone, Debug)]
struct SolverVendorStageOpts {
    from: PathBuf,
    platform: String,
    key_id: String,
}

#[derive(Clone, Debug)]
struct BinaryReproWitnessOpts {
    out: Option<PathBuf>,
}

#[derive(Clone, Debug)]
struct Milestone3BinaryBundleOpts {
    platform: String,
    binary: Option<PathBuf>,
    out_dir: Option<PathBuf>,
    key_id: String,
}

#[derive(Clone, Debug)]
struct Milestone3BinaryBundleVerifyOpts {
    bundle_dir: PathBuf,
}

#[derive(Clone, Debug)]
struct ManifestLockDriftOpts {
    paths: Vec<PathBuf>,
}

#[derive(Clone, Debug)]
struct StdSurfaceDriftOpts {
    emit_artifact: Option<PathBuf>,
    refresh_lock: bool,
}

#[derive(Clone, Debug)]
struct HostPolicyOpts {
    emit_artifact: Option<PathBuf>,
    refresh_lock: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct StdMetadataRoot {
    schema_version: u32,
    modules: Vec<StdMetadataModule>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct StdMetadataModule {
    path: String,
    exports: Vec<StdMetadataExport>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct StdMetadataExport {
    name: String,
    kind: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    layout: Option<serde_json::Value>,
}

#[derive(Clone, Debug, Serialize)]
struct StdCoreSurfaceMetadata {
    schema_version: u32,
    package: String,
    version: String,
    source_std_metadata_schema_version: u32,
    modules: Vec<StdMetadataModule>,
}

#[derive(Clone, Debug, Serialize)]
struct StrictPackageMetadataFile {
    schema_version: u32,
    packages: Vec<StrictPackageMetadataEntry>,
}

#[derive(Clone, Debug, Serialize)]
struct StrictPackageMetadataEntry {
    name: String,
    version: String,
    digest: String,
    artifact: StrictPackageArtifact,
    abi_id: String,
}

#[derive(Clone, Debug, Serialize)]
struct StrictPackageArtifact {
    format: String,
    path: String,
}

#[derive(Clone, Debug, Serialize)]
struct StrictAbiFile {
    schema_version: u32,
    contracts: Vec<StrictAbiContract>,
}

#[derive(Clone, Debug, Serialize)]
struct StrictAbiContract {
    abi_id: String,
    package: String,
    version: String,
    imports: Vec<StrictAbiImport>,
}

#[derive(Clone, Debug, Serialize)]
struct StrictAbiImport {
    symbol: String,
    effect: String,
    params: Vec<String>,
    ret: String,
    capability: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
struct StdCoreArtifactManifest {
    schema_version: u32,
    package: String,
    version: String,
    artifact: ArtifactEntry,
    metadata: MetadataEntry,
}

#[derive(Clone, Debug, Serialize)]
struct ArtifactEntry {
    file: String,
    digest: String,
}

#[derive(Clone, Debug, Serialize)]
struct MetadataEntry {
    surface: String,
    strict_package_metadata: String,
    strict_package_abi: String,
}

#[derive(Clone, Debug, Serialize)]
struct SharedStdArtifactDescriptor {
    format: String,
    path: String,
    digest: String,
    size_bytes: u64,
}

#[derive(Clone, Debug, Serialize)]
struct SharedStdVerifiedAbiDescriptor {
    major: u32,
    minor_min: u32,
    minor_max: u32,
}

#[derive(Clone, Debug, Serialize)]
struct SharedStdPackageSignatureEntry {
    name: String,
    version: String,
    digest: String,
    key_id: String,
    signed_at: String,
    signature_format: String,
    signature: String,
}

#[derive(Clone, Debug, Serialize)]
struct SharedStdPackageSignaturesFile {
    schema_version: u32,
    signatures: Vec<SharedStdPackageSignatureEntry>,
}

#[derive(Clone, Debug, Serialize)]
struct SharedStdPublishedPackageSignature {
    key_id: String,
    algorithm: String,
    signed_at: String,
    signature: String,
}

#[derive(Clone, Debug, Serialize)]
struct SharedStdPublishedProvenance {
    statement_digest: String,
    statement_format: String,
}

#[derive(Clone, Debug, Serialize)]
struct SharedStdPublishedPackage {
    schema_version: u32,
    package_id: String,
    version: String,
    verified_std_abi: SharedStdVerifiedAbiDescriptor,
    artifact: SharedStdArtifactDescriptor,
    signature: SharedStdPublishedPackageSignature,
    provenance: SharedStdPublishedProvenance,
    symbols: Vec<String>,
    dependencies: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
struct SharedStdProvenanceStatement {
    schema_version: u32,
    statement_type: String,
    package_id: String,
    version: String,
    verified_std_abi: SharedStdVerifiedAbiDescriptor,
    artifact: SharedStdArtifactDescriptor,
    metadata: SharedStdProvenanceMetadata,
    signature: SharedStdProvenanceSignatureRef,
    registry: SharedStdProvenanceRegistry,
}

#[derive(Clone, Debug, Serialize)]
struct SharedStdProvenanceMetadata {
    manifest_path: String,
    manifest_digest: String,
    package_metadata_path: String,
    package_metadata_digest: String,
    package_abi_path: String,
    package_abi_digest: String,
    package_signatures_path: String,
}

#[derive(Clone, Debug, Serialize)]
struct SharedStdProvenanceSignatureRef {
    key_id: String,
    algorithm: String,
    signed_at: String,
}

#[derive(Clone, Debug, Serialize)]
struct SharedStdProvenanceRegistry {
    mode: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    registry_dir: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
struct SharedStdPublishFileEntry {
    path: String,
    digest: String,
    size_bytes: u64,
}

#[derive(Clone, Debug, Serialize)]
struct SharedStdPublishManifest {
    schema_version: u32,
    package_id: String,
    version: String,
    verified_std_abi: SharedStdVerifiedAbiDescriptor,
    key_id: String,
    signed_at: String,
    statement_format: String,
    package_manifest: String,
    provenance_statement: String,
    public_key: String,
    registry_mode: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    registry_dir: Option<String>,
    files: Vec<SharedStdPublishFileEntry>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct StdBindingMapLockFile {
    schema_version: u32,
    symbols: Vec<StdBindingRouteEntry>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct StdBindingRouteEntry {
    symbol: String,
    route: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, Eq, PartialEq)]
struct HostCapabilityPolicyFile {
    schema_version: u32,
    profiles: Vec<HostCapabilityProfile>,
}

#[derive(Clone, Debug, Deserialize, Serialize, Eq, PartialEq)]
struct HostCapabilityProfile {
    profile: String,
    capabilities: Vec<HostCapabilityRule>,
}

#[derive(Clone, Debug, Deserialize, Serialize, Eq, PartialEq)]
struct HostCapabilityRule {
    capability: String,
    strict_mode: String,
    reason: String,
}

#[derive(Clone, Debug, Deserialize)]
struct DriftManifestFile {
    schema_version: u32,
    project: DriftManifestProject,
    #[serde(default)]
    dependencies: Vec<DriftManifestDependency>,
}

#[derive(Clone, Debug, Deserialize)]
struct DriftManifestProject {
    name: String,
}

#[derive(Clone, Debug, Deserialize)]
struct DriftManifestDependency {
    name: String,
    requirement: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, Eq, PartialEq)]
struct DriftLockFile {
    schema_version: u32,
    resolver_version: u32,
    roots: Vec<DriftLockRoot>,
    #[serde(default)]
    packages: Vec<DriftLockPackage>,
}

#[derive(Clone, Debug, Deserialize, Serialize, Eq, PartialEq)]
struct DriftLockRoot {
    name: String,
    dependencies: Vec<DriftLockRootDependency>,
}

#[derive(Clone, Debug, Deserialize, Serialize, Eq, PartialEq)]
struct DriftLockRootDependency {
    name: String,
    requirement: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, Eq, PartialEq)]
struct DriftLockPackage {
    id: String,
    name: String,
    version: String,
    digest: String,
    abi_id: String,
    #[serde(default)]
    dependencies: Vec<String>,
}
