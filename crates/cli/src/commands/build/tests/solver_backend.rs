    #[test]
    fn solver_backend_selection_defaults_to_external_cli() {
        assert_eq!(
            select_solver_backend_kind(None, false).expect("default backend"),
            SolverBackendKind::ExternalZ3Cli
        );
        assert_eq!(
            select_solver_backend_kind(Some(""), false).expect("empty backend uses default"),
            SolverBackendKind::ExternalZ3Cli
        );
        assert_eq!(
            select_solver_backend_kind(Some("   "), false)
                .expect("whitespace backend uses default"),
            SolverBackendKind::ExternalZ3Cli
        );
    }

    #[test]
    fn solver_backend_selection_accepts_explicit_external_cli() {
        assert_eq!(
            select_solver_backend_kind(Some("external-z3-cli"), false)
                .expect("explicit external backend"),
            SolverBackendKind::ExternalZ3Cli
        );
    }

    #[test]
    fn solver_backend_selection_rejects_rust_backend_when_feature_disabled() {
        let err = select_solver_backend_kind(Some("rust-z3-lib"), false)
            .expect_err("rust backend without feature should fail");
        assert!(err.to_string().contains("does not enable feature `rust-z3-lib`"));
    }

    #[cfg(feature = "rust-z3-lib")]
    #[test]
    fn solver_backend_selection_accepts_rust_backend_when_feature_enabled() {
        assert_eq!(
            select_solver_backend_kind(Some("rust-z3-lib"), true)
                .expect("rust backend with feature"),
            SolverBackendKind::RustZ3Lib
        );
    }

    #[test]
    fn solver_backend_selection_rejects_unknown_backend() {
        let err = select_solver_backend_kind(Some("z3"), false)
            .expect_err("unknown backend must fail");
        assert!(err.to_string().contains("unsupported solver backend"));
        assert!(err.to_string().contains("external-z3-cli"));
        assert!(err.to_string().contains("rust-z3-lib"));
    }

    #[test]
    fn solver_backend_effective_policy_keeps_external_default() {
        assert_eq!(
            effective_solver_backend_kind(SolverBackendKind::ExternalZ3Cli, false),
            SolverBackendKind::ExternalZ3Cli
        );
        assert_eq!(
            effective_solver_backend_kind(SolverBackendKind::ExternalZ3Cli, true),
            SolverBackendKind::ExternalZ3Cli
        );
    }

    #[test]
    fn solver_backend_effective_policy_falls_back_until_cutover_enabled() {
        assert_eq!(
            effective_solver_backend_kind(SolverBackendKind::RustZ3Lib, false),
            SolverBackendKind::ExternalZ3Cli
        );
        assert_eq!(
            effective_solver_backend_kind(SolverBackendKind::RustZ3Lib, true),
            SolverBackendKind::RustZ3Lib
        );
    }

    #[test]
    fn rust_z3_cutover_flag_defaults_to_false() {
        assert!(!parse_bool_cutover_flag(None).expect("default false"));
        assert!(!parse_bool_cutover_flag(Some("")).expect("empty false"));
        assert!(!parse_bool_cutover_flag(Some("   ")).expect("whitespace false"));
    }

    #[test]
    fn rust_z3_cutover_flag_accepts_true_and_false_values() {
        for value in ["1", "true", "on", "yes", "TRUE", "On"] {
            assert!(
                parse_bool_cutover_flag(Some(value)).expect("parse true value"),
                "expected true for `{value}`"
            );
        }
        for value in ["0", "false", "off", "no", "FALSE", "Off"] {
            assert!(
                !parse_bool_cutover_flag(Some(value)).expect("parse false value"),
                "expected false for `{value}`"
            );
        }
    }

    #[test]
    fn rust_z3_cutover_flag_rejects_invalid_values() {
        let err = parse_bool_cutover_flag(Some("maybe")).expect_err("invalid cutover flag");
        assert!(err.to_string().contains("invalid boolean value"));
        assert!(err.to_string().contains("CLG_SOLVER_RUST_Z3_CUTOVER"));
    }
