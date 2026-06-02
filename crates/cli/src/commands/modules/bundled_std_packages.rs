use std::collections::BTreeSet;
use std::sync::OnceLock;

use clg_typer::{builtin_route, non_abi_builtin_sigs};
use serde::Deserialize;

use super::ExternalImportBinding;

const BUNDLED_STD_PACKAGE_IDS: &[&str] = &["std::text", "std::int"];

#[derive(Deserialize)]
struct ExternalStdPackagePlanFile {
    schema_version: u32,
    packages: Vec<ExternalStdPackagePlanEntry>,
}

#[derive(Deserialize)]
struct ExternalStdPackagePlanEntry {
    package_id: String,
    modules: Vec<String>,
}

fn parse_bundled_std_package_plan(raw: &str) -> Result<ExternalStdPackagePlanFile, String> {
    let raw: ExternalStdPackagePlanFile =
        serde_json::from_str(raw).map_err(|err| format!("invalid std package plan JSON: {err}"))?;
    if raw.schema_version != 1 {
        return Err(format!(
            "unsupported std package plan schema version {}",
            raw.schema_version
        ));
    }
    Ok(raw)
}

fn load_bundled_std_package_modules_from_str(
    raw: &str,
    package_id: &str,
) -> Result<BTreeSet<String>, String> {
    let raw = parse_bundled_std_package_plan(raw)?;
    let Some(entry) = raw
        .packages
        .into_iter()
        .find(|entry| entry.package_id == package_id)
    else {
        return Err(format!("std package plan is missing `{package_id}`"));
    };
    Ok(entry.modules.into_iter().collect())
}

fn load_all_bundled_std_package_modules_from_str(raw: &str) -> Result<BTreeSet<String>, String> {
    let mut out = BTreeSet::new();
    for package_id in BUNDLED_STD_PACKAGE_IDS {
        out.extend(load_bundled_std_package_modules_from_str(raw, package_id)?);
    }
    Ok(out)
}

#[cfg(test)]
pub(super) fn bundled_std_text_modules() -> &'static BTreeSet<String> {
    static BUNDLED_STD_TEXT_MODULES: OnceLock<BTreeSet<String>> = OnceLock::new();
    BUNDLED_STD_TEXT_MODULES.get_or_init(|| {
        load_bundled_std_package_modules_from_str(
            include_str!("../../../../../docs/design/phase-27.2-external-std-package-plan.v1.json"),
            "std::text",
        )
        .expect("bundled std package plan must expose std::text")
    })
}

#[cfg(test)]
pub(super) fn bundled_std_int_modules() -> &'static BTreeSet<String> {
    static BUNDLED_STD_INT_MODULES: OnceLock<BTreeSet<String>> = OnceLock::new();
    BUNDLED_STD_INT_MODULES.get_or_init(|| {
        load_bundled_std_package_modules_from_str(
            include_str!("../../../../../docs/design/phase-27.2-external-std-package-plan.v1.json"),
            "std::int",
        )
        .expect("bundled std package plan must expose std::int")
    })
}

pub(super) fn bundled_std_package_modules() -> &'static BTreeSet<String> {
    static BUNDLED_STD_PACKAGE_MODULES: OnceLock<BTreeSet<String>> = OnceLock::new();
    BUNDLED_STD_PACKAGE_MODULES.get_or_init(|| {
        load_all_bundled_std_package_modules_from_str(include_str!(
            "../../../../../docs/design/phase-27.2-external-std-package-plan.v1.json"
        ))
        .expect("bundled std package plan must expose configured wave1 packages")
    })
}

pub(super) fn is_bundled_std_package_module(path: &str) -> bool {
    bundled_std_package_modules().contains(path)
}

pub(super) fn bundled_std_package_external_imports() -> Vec<ExternalImportBinding> {
    non_abi_builtin_sigs()
        .into_iter()
        .filter(|(function, _, _, _)| {
            function
                .rsplit_once("::")
                .map(|(module, _)| is_bundled_std_package_module(module))
                .unwrap_or(false)
        })
        .map(|(function, params, ret, effect)| {
            let (import_module, import_name) = function
                .rsplit_once("::")
                .map(|(module, name)| (module.to_string(), name.to_string()))
                .expect("std builtin symbol must contain module and name");
            ExternalImportBinding {
                function: function.clone(),
                import_module,
                import_name,
                params,
                ret,
                effect,
                route: builtin_route(function.as_str()),
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::{
        bundled_std_int_modules, bundled_std_package_external_imports, bundled_std_package_modules,
        bundled_std_text_modules, load_bundled_std_package_modules_from_str,
    };

    #[test]
    fn rejects_unsupported_schema_version() {
        let err = load_bundled_std_package_modules_from_str(
            r#"{
              "schema_version": 2,
              "packages": []
            }"#,
            "std::text",
        )
        .expect_err("invalid schema version should fail");
        assert!(
            err.contains("unsupported std package plan schema version"),
            "expected schema validation error, got: {err}"
        );
    }

    #[test]
    fn bundled_std_text_modules_follow_phase27_plan() {
        let expected: BTreeSet<String> = ["std::bytes", "std::str", "std::str_pattern"]
            .into_iter()
            .map(str::to_string)
            .collect();
        assert_eq!(
            bundled_std_text_modules(),
            &expected,
            "std::text bundled module set must stay aligned with the phase 27 plan"
        );
    }

    #[test]
    fn bundled_std_int_modules_follow_phase27_plan() {
        let expected: BTreeSet<String> = ["std::u128", "std::u256", "std::u64"]
            .into_iter()
            .map(str::to_string)
            .collect();
        assert_eq!(
            bundled_std_int_modules(),
            &expected,
            "std::int bundled module set must stay aligned with the phase 27 plan"
        );
    }

    #[test]
    fn bundled_std_package_modules_cover_current_wave1_cut() {
        let expected: BTreeSet<String> = [
            "std::bytes",
            "std::str",
            "std::str_pattern",
            "std::u64",
            "std::u128",
            "std::u256",
        ]
        .into_iter()
        .map(str::to_string)
        .collect();
        assert_eq!(
            bundled_std_package_modules(),
            &expected,
            "bundled std package module set must stay aligned with the active Wave 1 cut"
        );
    }

    #[test]
    fn bundled_std_package_external_imports_cover_text_and_int_surface() {
        let symbols: BTreeSet<String> = bundled_std_package_external_imports()
            .iter()
            .map(|binding| binding.function.clone())
            .collect();
        assert!(symbols.contains("std::str::len"));
        assert!(symbols.contains("std::bytes::eq_ct"));
        assert!(symbols.contains("std::str_pattern::matches"));
        assert!(symbols.contains("std::u64::rotl"));
        assert!(symbols.contains("std::u128::from_limbs"));
        assert!(symbols.contains("std::u256::limb3"));
    }
}
