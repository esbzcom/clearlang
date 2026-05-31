#[cfg(test)]
mod tests {
    use super::{
        classify_runtime_failure, escape_xml, expected_outcome_label, merge_mock_sets,
        normalize_relpath, runtime_failure_id,
        panic_payload_message, render_replay_command, replay_contract, timeout_failure_outcome,
        TestExpectedOutcomeKind, TestExpectedOutcomeSpec, TestPlan, TestPlanCase,
        DEFAULT_TEST_TIMEOUT_MS, TEST_EXPECTATION_FAILURE_CODE,
        DEFAULT_TEST_WORKER_FUEL_LIMIT, DEFAULT_TEST_WORKER_MEMORY_LIMIT_BYTES,
        TEST_ASSERTION_FAILURE_CODE, TEST_RUNTIME_FAILURE_CODE, TEST_TIMEOUT_FAILURE_CODE,
    };
    use std::path::Path;

    #[test]
    fn normalize_relpath_uses_forward_slashes() {
        let root = Path::new("C:/repo");
        let path = Path::new("C:/repo/tests/unit/a.clear");
        assert_eq!(normalize_relpath(root, path), "tests/unit/a.clear");
    }

    #[test]
    fn xml_escape_covers_reserved_characters() {
        assert_eq!(escape_xml("<a&b>\"'"), "&lt;a&amp;b&gt;&quot;&apos;");
    }

    #[test]
    fn test_plan_case_order_can_be_checked_deterministically() {
        let plan = TestPlan {
            schema_version: 1,
            default_mock_sets: vec!["common".to_string()],
            cases: vec![
                TestPlanCase {
                    test_id: "tests/unit/b.clear::test_b".to_string(),
                    mock_sets: vec!["common".to_string()],
                    timeout_ms: None,
                    expected_outcome: None,
                },
                TestPlanCase {
                    test_id: "tests/unit/a.clear::test_a".to_string(),
                    mock_sets: vec!["common".to_string()],
                    timeout_ms: Some(120_000),
                    expected_outcome: None,
                },
            ],
        };

        let mut ids: Vec<&str> = plan.cases.iter().map(|c| c.test_id.as_str()).collect();
        let original = ids.clone();
        ids.sort_unstable();
        assert_ne!(
            original, ids,
            "unsorted test plan fixtures should differ from sorted order"
        );
    }

    #[test]
    fn merge_mock_sets_preserves_last_override_deterministically() {
        let merged = merge_mock_sets(
            &["common".to_string(), "base".to_string()],
            &["promo".to_string(), "base".to_string()],
        );
        assert_eq!(merged, vec!["common", "promo", "base"]);
    }

    #[test]
    fn default_timeout_contract_is_two_minutes() {
        assert_eq!(DEFAULT_TEST_TIMEOUT_MS, 120_000);
    }

    #[test]
    fn replay_contract_contains_single_test_filter_flow() {
        let replay = replay_contract("tests/unit/a.clear::test_a");
        assert_eq!(
            replay.argv,
            vec![
                "clg".to_string(),
                "test".to_string(),
                "<project-root>".to_string(),
                "--filter".to_string(),
                "tests/unit/a.clear::test_a".to_string(),
                "--report".to_string(),
                "json".to_string()
            ]
        );
    }

    #[test]
    fn render_replay_command_quotes_non_identifier_arguments() {
        let rendered = render_replay_command(&[
            "clg".to_string(),
            "test".to_string(),
            "<project-root>".to_string(),
            "--filter".to_string(),
            "tests/unit/a.clear::test a".to_string(),
        ]);
        assert!(rendered.contains("\"<project-root>\""));
        assert!(rendered.contains("\"tests/unit/a.clear::test a\""));
    }

    #[test]
    fn test_failure_codes_are_stable() {
        assert_eq!(TEST_TIMEOUT_FAILURE_CODE, "C137");
        assert_eq!(TEST_RUNTIME_FAILURE_CODE, "C138");
        assert_eq!(TEST_ASSERTION_FAILURE_CODE, "C139");
        assert_eq!(TEST_EXPECTATION_FAILURE_CODE, "C141");
    }

    #[test]
    fn timeout_failure_outcome_maps_to_c137_contract() {
        let outcome = timeout_failure_outcome(1);
        assert_eq!(outcome.status, "failed");
        assert_eq!(outcome.failure_kind, Some("timeout"));
        assert_eq!(outcome.failure_code, Some(TEST_TIMEOUT_FAILURE_CODE));
        assert!(
            outcome
                .reason
                .as_deref()
                .unwrap_or_default()
                .contains("timeout after 1ms"),
            "expected deterministic timeout reason"
        );
    }

    #[test]
    fn default_runtime_safety_limits_are_stable() {
        assert_eq!(DEFAULT_TEST_WORKER_FUEL_LIMIT, 50_000_000);
        assert_eq!(DEFAULT_TEST_WORKER_MEMORY_LIMIT_BYTES, 64 * 1024 * 1024);
    }

    #[test]
    fn classify_runtime_failure_maps_fuel_and_memory_reasons() {
        assert_eq!(
            classify_runtime_failure("all fuel consumed by WebAssembly"),
            "fuel_exhausted: all fuel consumed by WebAssembly"
        );
        assert_eq!(
            classify_runtime_failure("forcing trap when growing memory to 123 bytes"),
            "memory_limit: forcing trap when growing memory to 123 bytes"
        );
        assert_eq!(
            classify_runtime_failure("unexpected host trap"),
            "runtime_trap: unexpected host trap"
        );
    }

    #[test]
    fn runtime_failure_id_is_deterministic_by_reason_prefix() {
        assert_eq!(
            runtime_failure_id("fuel_exhausted: out of fuel"),
            "runtime.fuel_exhausted"
        );
        assert_eq!(
            runtime_failure_id("memory_limit: cannot grow memory"),
            "runtime.memory_limit"
        );
        assert_eq!(
            runtime_failure_id("worker_crash: panic payload"),
            "runtime.worker_crash"
        );
        assert_eq!(runtime_failure_id("runtime_trap: trap"), "runtime.trap");
    }

    #[test]
    fn expected_outcome_label_defaults_to_pass() {
        assert_eq!(expected_outcome_label(None), "pass");
        assert_eq!(
            expected_outcome_label(Some(&TestExpectedOutcomeSpec {
                kind: TestExpectedOutcomeKind::Runtime,
                failure_code: None,
                reason_contains: None,
            })),
            "runtime"
        );
    }

    #[test]
    fn panic_payload_message_extracts_string_payloads() {
        let static_payload: Box<dyn std::any::Any + Send> = Box::new("boom");
        assert_eq!(panic_payload_message(static_payload.as_ref()), "boom");

        let owned_payload: Box<dyn std::any::Any + Send> = Box::new("owned".to_string());
        assert_eq!(panic_payload_message(owned_payload.as_ref()), "owned");
    }
}
