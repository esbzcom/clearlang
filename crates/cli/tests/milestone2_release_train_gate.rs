use serde_json::Value;
use std::env;
use std::fs;
use std::path::Path;

fn enforce_gate() -> bool {
    env::var("CLG_ENFORCE_MILESTONE2_RELEASE_GATE")
        .ok()
        .as_deref()
        == Some("1")
}

fn todo_has_checked_item(todo: &str, item: &str) -> bool {
    todo.lines().any(|line| {
        let line = line.trim_start();
        let Some(rest) = line.strip_prefix("- [x] ") else {
            return false;
        };
        let Some(after) = rest.strip_prefix(item) else {
            return false;
        };
        after
            .chars()
            .next()
            .map(|ch| ch.is_ascii_whitespace())
            .unwrap_or(true)
    })
}

fn validate_release_train_lock_artifacts(root: &Path) {
    let runtime_rollout_lock_path = root
        .join("docs")
        .join("runtime")
        .join("runtime-loader-rollout-gate.lock.json");
    let runtime_rollout_lock: Value = serde_json::from_slice(
        &fs::read(&runtime_rollout_lock_path).expect("read runtime loader rollout lock"),
    )
    .expect("parse runtime loader rollout lock");
    assert_eq!(
        runtime_rollout_lock
            .get("schema_version")
            .and_then(|v| v.as_u64()),
        Some(1),
        "release-train gate requires runtime rollout lock schema_version=1"
    );
    let runtime_required = runtime_rollout_lock
        .get("required_diagnostics")
        .and_then(|v| v.as_array())
        .expect("runtime rollout lock required_diagnostics[]");
    for code in ["R012", "R013", "R014", "R015", "R016", "R017"] {
        assert!(
            runtime_required.iter().any(|v| v.as_str() == Some(code)),
            "release-train gate requires runtime rollout lock diagnostic `{code}`"
        );
    }
    assert!(
        runtime_rollout_lock
            .get("release_gate_evidence")
            .and_then(|v| v.as_array())
            .map(|items| !items.is_empty())
            .unwrap_or(false),
        "release-train gate requires non-empty runtime rollout release evidence lock"
    );

    let host_profile_lock_path = root
        .join("docs")
        .join("runtime")
        .join("host-profiles-production-policy.lock.json");
    let host_profile_lock: Value =
        serde_json::from_slice(&fs::read(&host_profile_lock_path).expect("read host profile lock"))
            .expect("parse host profile lock");
    assert_eq!(
        host_profile_lock
            .get("schema_version")
            .and_then(|v| v.as_u64()),
        Some(1),
        "release-train gate requires host profile lock schema_version=1"
    );
    let host_diagnostics = host_profile_lock
        .get("diagnostics")
        .and_then(|v| v.as_array())
        .expect("host profile lock diagnostics[]");
    for code in ["C106", "R012", "R016"] {
        assert!(
            host_diagnostics.iter().any(|v| v.as_str() == Some(code)),
            "release-train gate requires host profile lock diagnostic `{code}`"
        );
    }
    let profile_ids = host_profile_lock
        .get("profiles")
        .and_then(|v| v.as_array())
        .expect("host profile lock profiles[]");
    for profile in ["contract_static", "shared_app"] {
        assert!(
            profile_ids
                .iter()
                .any(|entry| entry.get("id").and_then(|v| v.as_str()) == Some(profile)),
            "release-train gate requires host profile lock profile `{profile}`"
        );
    }
}

#[test]
fn milestone2_release_train_gate_validates_lock_artifacts_in_all_runs() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    validate_release_train_lock_artifacts(&root);
}

#[test]
fn milestone2_release_train_gate_requires_24_2_checklist_and_release_notes_when_enforced() {
    if !enforce_gate() {
        return;
    }

    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let todo_path = root.join("docs").join("TODO.md");
    let todo = fs::read_to_string(&todo_path).expect("read docs/TODO.md");
    for idx in 1..=10 {
        let item = format!("24.2.{idx}");
        assert!(
            todo_has_checked_item(&todo, item.as_str()),
            "release-train gate requires checklist item `- [x] {item}` to be complete before milestone_2 tag"
        );
    }

    let release_notes_path = root.join("release_notes").join("milestone_2.md");
    assert!(
        release_notes_path.exists(),
        "release-train gate requires `release_notes/milestone_2.md`"
    );
    validate_release_train_lock_artifacts(&root);
}
