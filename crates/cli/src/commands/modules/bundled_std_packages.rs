use std::collections::BTreeSet;
use std::sync::OnceLock;

use clg_typer::{builtin_route, non_abi_builtin_sigs};
use serde::Deserialize;

use super::ExternalImportBinding;

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

fn load_bundled_std_text_modules_from_str(raw: &str) -> Result<BTreeSet<String>, String> {
    let raw: ExternalStdPackagePlanFile =
        serde_json::from_str(raw).map_err(|err| format!("invalid std package plan JSON: {err}"))?;
    if raw.schema_version != 1 {
        return Err(format!(
            "unsupported std package plan schema version {}",
            raw.schema_version
        ));
    }
    let Some(std_text) = raw
        .packages
        .into_iter()
        .find(|entry| entry.package_id == "std::text")
    else {
        return Err("std package plan is missing `std::text`".to_string());
    };
    Ok(std_text.modules.into_iter().collect())
}

pub(super) fn bundled_std_text_modules() -> &'static BTreeSet<String> {
    static BUNDLED_STD_TEXT_MODULES: OnceLock<BTreeSet<String>> = OnceLock::new();
    BUNDLED_STD_TEXT_MODULES.get_or_init(|| {
        load_bundled_std_text_modules_from_str(include_str!(
            "../../../../../docs/design/phase-27.2-external-std-package-plan.v1.json"
        ))
        .expect("bundled std package plan must expose std::text")
    })
}

pub(super) fn is_bundled_std_text_module(path: &str) -> bool {
    bundled_std_text_modules().contains(path)
}

pub(super) fn bundled_std_text_external_imports() -> Vec<ExternalImportBinding> {
    non_abi_builtin_sigs()
        .into_iter()
        .filter(|(function, _, _, _)| {
            function
                .rsplit_once("::")
                .map(|(module, _)| is_bundled_std_text_module(module))
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
        bundled_std_text_external_imports, bundled_std_text_modules,
        load_bundled_std_text_modules_from_str,
    };

    #[test]
    fn rejects_unsupported_schema_version() {
        let err = load_bundled_std_text_modules_from_str(
            r#"{
              "schema_version": 2,
              "packages": []
            }"#,
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
    fn bundled_std_text_external_imports_cover_text_surface() {
        let symbols: BTreeSet<String> = bundled_std_text_external_imports()
            .iter()
            .map(|binding| binding.function.clone())
            .collect();
        assert!(symbols.contains("std::str::len"));
        assert!(symbols.contains("std::bytes::eq_ct"));
        assert!(symbols.contains("std::str_pattern::matches"));
    }
}
