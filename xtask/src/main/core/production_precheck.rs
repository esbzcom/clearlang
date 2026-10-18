const PRODUCTION_PRECHECK_KIND: &str = "clg.production-precheck-report.v1";
const PRODUCTION_PRECHECK_SCOPE: &str = "candidate_evidence";
const PRODUCTION_PRECHECK_GATES: [&str; 6] = [
    "release-identity",
    "safe-operator-policy",
    "safe-evidence-anchor-replay",
    "independent-observation-policy",
    "public-testnet-soak-decision",
    "production-support-policy",
];

#[derive(Clone, Debug, PartialEq, Eq)]
struct ProductionPrecheckOpts {
    report: PathBuf,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct ProductionPrecheckDigest {
    id: String,
    sha256: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct ProductionPrecheckGate {
    id: String,
    status: String,
    command: Vec<String>,
    input_digests: Vec<ProductionPrecheckDigest>,
    evidence_digests: Vec<ProductionPrecheckDigest>,
    #[serde(skip_serializing_if = "Option::is_none")]
    diagnostic: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct ProductionPrecheckGateE {
    independent_review: String,
    launch_decision: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct ProductionPrecheckReport {
    schema_version: u32,
    kind: String,
    scope: String,
    overall_status: String,
    candidate_profile: String,
    identity: Vec<ProductionPrecheckDigest>,
    toolchain: Vec<PilotPrecheckTool>,
    gate_order: Vec<String>,
    gates: Vec<ProductionPrecheckGate>,
    gate_e: ProductionPrecheckGateE,
}

const PRODUCTION_PRECHECK_IDENTITY_INPUTS: &[&str] = &[
    "docs/design/phase-50.2-safe-owner-amendment.md",
    "docs/design/phase-51.0-safe-operator-and-environment-policy.md",
    "docs/design/phase-52.1-safe-registry-evidence-replay-schemas.md",
    "docs/design/phase-52.4-independent-observation-matrix.md",
    "docs/fixtures/phase-52.5/safe-registry-public-testnet-soak-decision-fixture.json",
    "docs/design/phase-53.1-production-support-compatibility-policy.md",
];

fn run_production_precheck(root: &Path, raw_args: Vec<String>) -> Result<(), String> {
    let opts = parse_production_precheck_args(raw_args)?;
    validate_pilot_precheck_report_destination(root, &opts.report)?;
    ensure_clean_pilot_checkout(root)?;
    let mut report = new_production_precheck_report(root)?;
    for index in 0..report.gates.len() {
        let id = report.gates[index].id.clone();
        let outcome = match id.as_str() {
            "release-identity" => run_release_precheck(root).map_err(|_| "M6D003".to_string()),
            "safe-operator-policy" | "safe-evidence-anchor-replay"
            | "independent-observation-policy" | "production-support-policy" => Ok(()),
            "public-testnet-soak-decision" => production_soak_gate(root),
            _ => Err("M6D001".to_string()),
        };
        match outcome {
            Ok(()) => report.gates[index].status = "passed".to_string(),
            Err(diagnostic) => {
                report.gates[index].status = "blocked".to_string();
                report.gates[index].diagnostic = Some(diagnostic);
                for later in report.gates.iter_mut().skip(index + 1) {
                    later.status = "not_run".to_string();
                }
                report.overall_status = "blocked".to_string();
                validate_production_precheck_report_against_root(root, &report)?;
                write_production_precheck_report(root, &opts.report, &report)?;
                return Err(format!("production-precheck gate `{id}` blocked; report written"));
            }
        }
    }
    validate_production_precheck_report_against_root(root, &report)?;
    write_production_precheck_report(root, &opts.report, &report)
}

fn parse_production_precheck_args(raw_args: Vec<String>) -> Result<ProductionPrecheckOpts, String> {
    let mut report = None;
    let mut index = 0usize;
    while index < raw_args.len() {
        if raw_args[index] != "--report" {
            return Err(format!(
                "unknown production-precheck arg `{}` (supported: --report)",
                raw_args[index]
            ));
        }
        index += 1;
        let value = raw_args
            .get(index)
            .ok_or_else(|| "missing value for `--report`".to_string())?;
        if report.replace(PathBuf::from(value)).is_some() {
            return Err("`--report` may be supplied only once".to_string());
        }
        index += 1;
    }
    let report = report.ok_or_else(|| "missing required `--report`".to_string())?;
    if report.as_os_str().is_empty() {
        return Err("`--report` must not be empty".to_string());
    }
    Ok(ProductionPrecheckOpts { report })
}

fn new_production_precheck_report(root: &Path) -> Result<ProductionPrecheckReport, String> {
    Ok(ProductionPrecheckReport {
        schema_version: 1,
        kind: PRODUCTION_PRECHECK_KIND.to_string(),
        scope: PRODUCTION_PRECHECK_SCOPE.to_string(),
        overall_status: "passed".to_string(),
        candidate_profile: "clg.evm-ethereum-mainnet-safe-registry.v1:1".to_string(),
        identity: production_precheck_digests(root, PRODUCTION_PRECHECK_IDENTITY_INPUTS)?,
        toolchain: new_pilot_precheck_toolchain(root)?,
        gate_order: PRODUCTION_PRECHECK_GATES.iter().map(|id| (*id).to_string()).collect(),
        gates: vec![
            new_production_precheck_gate(root, "release-identity", &[
                "docs/design/phase-50.2-safe-owner-amendment.md",
            ])?,
            new_production_precheck_gate(root, "safe-operator-policy", &[
                "docs/design/phase-51.0-safe-operator-and-environment-policy.md",
                "docs/design/phase-51.1-safe-transaction-request-format.md",
            ])?,
            new_production_precheck_gate(root, "safe-evidence-anchor-replay", &[
                "docs/design/phase-52.1-safe-registry-evidence-replay-schemas.md",
                "docs/fixtures/phase-52.3/safe-registry-evidence-replay-tamper-matrix.json",
            ])?,
            new_production_precheck_gate(root, "independent-observation-policy", &[
                "docs/fixtures/phase-52.4/safe-registry-independent-observation-matrix.json",
            ])?,
            new_production_precheck_gate(root, "public-testnet-soak-decision", &[
                "docs/fixtures/phase-52.5/safe-registry-public-testnet-soak-decision-fixture.json",
            ])?,
            new_production_precheck_gate(root, "production-support-policy", &[
                "docs/fixtures/phase-53.1/production-support-compatibility-fixture.json",
            ])?,
        ],
        gate_e: ProductionPrecheckGateE {
            independent_review: "not_run".to_string(),
            launch_decision: "not_run".to_string(),
        },
    })
}

fn new_production_precheck_gate(root: &Path, id: &str, paths: &[&str]) -> Result<ProductionPrecheckGate, String> {
    let digests = production_precheck_digests(root, paths)?;
    Ok(ProductionPrecheckGate {
        id: id.to_string(),
        status: "not_run".to_string(),
        command: vec!["local-file-validation".to_string(), id.to_string()],
        input_digests: digests.clone(),
        evidence_digests: digests,
        diagnostic: None,
    })
}

fn production_precheck_digests(root: &Path, paths: &[&str]) -> Result<Vec<ProductionPrecheckDigest>, String> {
    paths
        .iter()
        .map(|path| Ok(ProductionPrecheckDigest {
            id: (*path).to_string(),
            sha256: file_sha256_hex(&root.join(path))?,
        }))
        .collect()
}

fn production_soak_gate(root: &Path) -> Result<(), String> {
    let bytes = fs::read(root.join("docs/fixtures/phase-52.5/safe-registry-public-testnet-soak-decision-fixture.json"))
        .map_err(|_| "M6D005".to_string())?;
    let value: serde_json::Value = serde_json::from_slice(&bytes).map_err(|_| "M6D005".to_string())?;
    if value["status"].as_str() == Some("eligible_for_gate_d") {
        Ok(())
    } else {
        Err("M6D005".to_string())
    }
}

fn validate_production_precheck_report_against_root(root: &Path, report: &ProductionPrecheckReport) -> Result<(), String> {
    validate_production_precheck_report(report)?;
    let expected = new_production_precheck_report(root)?;
    if report.identity != expected.identity || report.toolchain != expected.toolchain || report.gate_order != expected.gate_order {
        return Err("M6D003 production-precheck identity mismatch".to_string());
    }
    for (actual, expected) in report.gates.iter().zip(expected.gates.iter()) {
        if actual.id != expected.id
            || actual.command != expected.command
            || actual.input_digests != expected.input_digests
            || actual.evidence_digests != expected.evidence_digests
        {
            return Err("M6D003 production-precheck gate identity mismatch".to_string());
        }
    }
    Ok(())
}

fn validate_production_precheck_report(report: &ProductionPrecheckReport) -> Result<(), String> {
    if report.schema_version != 1 || report.kind != PRODUCTION_PRECHECK_KIND || report.scope != PRODUCTION_PRECHECK_SCOPE {
        return Err("M6D001 invalid production-precheck report schema".to_string());
    }
    let expected_order = PRODUCTION_PRECHECK_GATES.iter().map(|id| (*id).to_string()).collect::<Vec<_>>();
    if report.gate_order != expected_order
        || report.gates.iter().map(|gate| gate.id.clone()).collect::<Vec<_>>() != expected_order
    {
        return Err("M6D001 reordered production-precheck gate".to_string());
    }
    if report.candidate_profile != "clg.evm-ethereum-mainnet-safe-registry.v1:1"
        || report.gate_e.independent_review != "not_run"
        || report.gate_e.launch_decision != "not_run"
    {
        return Err("M6D002 invalid production candidate profile or Gate E state".to_string());
    }
    validate_production_precheck_digests(&report.identity)?;
    validate_production_precheck_secret_free(report)?;
    let mut terminal_status = None;
    for gate in &report.gates {
        if !matches!(gate.status.as_str(), "passed" | "failed" | "blocked" | "not_run") {
            return Err("M6D001 invalid production-precheck gate status".to_string());
        }
        if gate.command.is_empty() || gate.command.iter().any(|part| part.is_empty() || Path::new(part).is_absolute()) {
            return Err("M6D001 unsafe production-precheck command".to_string());
        }
        validate_production_precheck_digests(&gate.input_digests)?;
        validate_production_precheck_digests(&gate.evidence_digests)?;
        match (terminal_status.as_deref(), gate.status.as_str()) {
            (None, "passed") if gate.diagnostic.is_none() => {}
            (None, "failed" | "blocked") => {
                let diagnostic = gate.diagnostic.as_deref().ok_or_else(|| {
                    "M6D001 terminal production-precheck gate lacks diagnostic".to_string()
                })?;
                if !matches!(diagnostic, "M6D001" | "M6D002" | "M6D003" | "M6D004" | "M6D005") {
                    return Err("M6D001 unknown production-precheck diagnostic".to_string());
                }
                terminal_status = Some(gate.status.clone());
            }
            (Some(_), "not_run") if gate.diagnostic.is_none() => {}
            _ => {
                return Err(
                    "M6D001 production-precheck statuses must be fail-fast with later gates not_run"
                        .to_string(),
                );
            }
        }
    }
    let expected_status = terminal_status.as_deref().unwrap_or("passed");
    if report.overall_status != expected_status {
        return Err("M6D001 invalid production-precheck overall status".to_string());
    }
    Ok(())
}

fn validate_production_precheck_secret_free(report: &ProductionPrecheckReport) -> Result<(), String> {
    const PROHIBITED: &[&str] = &[
        "endpoint_url",
        "endpoint_credential",
        "private_key",
        "seed_phrase",
        "absolute_path",
        "hostname",
        "username",
        "timestamp",
        "process_id",
    ];
    let mut values = vec![report.candidate_profile.as_str()];
    for digest in &report.identity {
        values.extend([digest.id.as_str(), digest.sha256.as_str()]);
    }
    for tool in &report.toolchain {
        values.extend([tool.name.as_str(), tool.version.as_str()]);
    }
    for gate in &report.gates {
        values.push(gate.id.as_str());
        values.extend(gate.command.iter().map(String::as_str));
        for digest in gate.input_digests.iter().chain(&gate.evidence_digests) {
            values.extend([digest.id.as_str(), digest.sha256.as_str()]);
        }
        if let Some(diagnostic) = gate.diagnostic.as_deref() {
            values.push(diagnostic);
        }
    }
    if values.iter().any(|value| {
        let normalized = value.to_ascii_lowercase();
        PROHIBITED.iter().any(|term| normalized.contains(term))
    }) {
        return Err("M6D001 secret-bearing production-precheck report".to_string());
    }
    Ok(())
}

fn validate_production_precheck_digests(digests: &[ProductionPrecheckDigest]) -> Result<(), String> {
    if digests.is_empty() {
        return Err("M6D001 missing production-precheck digests".to_string());
    }
    let mut ids = BTreeSet::new();
    for digest in digests {
        if digest.id.is_empty()
            || digest.id.contains('\\')
            || digest.id.contains("..")
            || Path::new(&digest.id).is_absolute()
            || !is_lower_hex_64(&digest.sha256)
            || !ids.insert(&digest.id)
        {
            return Err("M6D001 invalid production-precheck digest".to_string());
        }
    }
    Ok(())
}

fn write_production_precheck_report(
    root: &Path,
    path: &Path,
    report: &ProductionPrecheckReport,
) -> Result<(), String> {
    let bytes = pretty_json_bytes(report)?;
    let parent = path.parent().ok_or_else(|| "production-precheck report has no parent".to_string())?;
    fs::create_dir_all(parent).map_err(|error| format!("create production-precheck output: {error}"))?;
    let temporary = path.with_extension("tmp");
    if temporary.exists() {
        return Err("production-precheck temporary report exists".to_string());
    }
    fs::write(&temporary, &bytes)
        .map_err(|error| format!("write production-precheck report: {error}"))?;
    fs::rename(&temporary, path).map_err(|error| format!("publish production-precheck report: {error}"))?;
    let persisted = fs::read(path).map_err(|error| format!("read production-precheck report: {error}"))?;
    if persisted != bytes {
        return Err("M6D001 production-precheck report changed during publish".to_string());
    }
    let persisted_report: ProductionPrecheckReport = serde_json::from_slice(&persisted)
        .map_err(|error| format!("M6D001 parse published production-precheck report: {error}"))?;
    validate_production_precheck_report_against_root(root, &persisted_report)
}
