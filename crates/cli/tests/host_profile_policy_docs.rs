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
        content.contains("host-profiles-production-policy.lock.json"),
        "policy doc should reference the machine-readable host profile policy lock artifact"
    );

    let lock_path = root
        .join("docs")
        .join("runtime")
        .join("host-profiles-production-policy.lock.json");
    let lock: serde_json::Value =
        serde_json::from_slice(&fs::read(&lock_path).expect("read host profile lock artifact"))
            .expect("parse host profile lock artifact");
    assert!(
        lock.get("schema_version").and_then(|v| v.as_u64()) == Some(1),
        "host profile lock artifact must use schema_version=1"
    );
    let profiles = lock
        .get("profiles")
        .and_then(|v| v.as_array())
        .expect("profiles array in host profile lock artifact");
    assert!(
        profiles
            .iter()
            .filter_map(|p| p.get("id").and_then(|v| v.as_str()))
            .collect::<Vec<_>>()
            == vec!["contract_static", "shared_app"],
        "host profile lock artifact should pin both production profiles in deterministic order"
    );

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
    assert!(
        lock.get("diagnostics")
            .and_then(|v| v.as_array())
            .map(|codes| {
                let values = codes.iter().filter_map(|v| v.as_str()).collect::<Vec<_>>();
                values.contains(&"C106") && values.contains(&"R012") && values.contains(&"R016")
            })
            .unwrap_or(false),
        "host profile lock artifact should pin strict/runtime diagnostic codes for this policy"
    );
}
