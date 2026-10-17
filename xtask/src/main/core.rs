// Gate-contract wiring markers kept in this top-level file for policy tests:
// "release-precheck" -> run_release_precheck
include!("core/dispatch_and_drift.rs");
include!("core/artifacts_and_vendor.rs");
include!("core/safe_transaction_policy.rs");
