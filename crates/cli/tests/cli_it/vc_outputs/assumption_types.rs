#[derive(Deserialize)]
struct IntrinsicLevelEntry {
    intrinsic: String,
    tier: String,
    label: String,
}

#[derive(Deserialize)]
struct ProofAssumptionEntry {
    id: String,
    category: String,
    #[serde(default)]
    symbols: Vec<String>,
    #[serde(default)]
    intrinsic_levels: Vec<IntrinsicLevelEntry>,
}

#[derive(Deserialize)]
struct AssuranceLevelsEntry {
    #[serde(rename = "L0")]
    l0: String,
    #[serde(rename = "L1")]
    l1: String,
    #[serde(rename = "L2")]
    l2: String,
    #[serde(rename = "L3")]
    l3: String,
}

#[derive(Deserialize)]
struct AssuranceEntry {
    tier: String,
    label: String,
    levels: AssuranceLevelsEntry,
}

#[derive(Deserialize)]
struct AssuranceClaimEntry {
    compiler_mode: String,
    non_strict_evidence_only: bool,
    release_grade_trust: bool,
}

#[derive(Deserialize)]
struct ProofAssumptionsEntry {
    items: Vec<ProofAssumptionEntry>,
}

#[derive(Deserialize)]
struct ProofVcAssumptionsEntry {
    vc_id: String,
    #[serde(default)]
    assumptions: Option<ProofAssumptionsEntry>,
    #[serde(default)]
    assurance: Option<AssuranceEntry>,
}

#[derive(Deserialize)]
struct ProofFunctionAssumptionsEntry {
    name: String,
    vcs: Vec<ProofVcAssumptionsEntry>,
}

#[derive(Deserialize)]
struct ProofAssumptionsSection {
    functions: Vec<ProofFunctionAssumptionsEntry>,
    #[serde(default)]
    assurance: Option<AssuranceEntry>,
    #[serde(default)]
    assurance_claim: Option<AssuranceClaimEntry>,
}

