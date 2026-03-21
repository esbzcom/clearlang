use std::fs;
use std::path::Path;

#[test]
fn host_profile_production_policy_doc_covers_profiles_and_diagnostics() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let path = root
        .join("docs")
        .join("runtime")
        .join("host-profiles-production-policy.md");
    let content = fs::read_to_string(&path).expect("read host profile production policy doc");

    assert!(
        content.contains("contract_static"),
        "policy doc should cover contract_static profile"
    );
    assert!(
        content.contains("shared_app"),
        "policy doc should cover shared_app profile"
    );
    assert!(
        content.contains("C106"),
        "policy doc should include strict host-capability diagnostic contract"
    );
    assert!(
        content.contains("R012"),
        "policy doc should include runtime fail-closed contract"
    );
    assert!(
        content.contains("R016"),
        "policy doc should include runtime host-profile input diagnostics"
    );
    assert!(
        content.contains("std::env::time") && content.contains("std::env::random"),
        "policy doc should lock deterministic env capability policy"
    );
}
