use std::collections::HashMap;

use anyhow::Result;
use clg_ast::{ImportKind, Span};

use super::error::module_error;
use super::package_metadata::{PackageMetadataIndex, PackageModuleIndex};
use super::std_metadata::{std_metadata, StdModuleIndex};
use super::{qualify_name, target_prefix, Exports, ImportEnv, ModuleUnit};

pub(super) fn build_import_env(
    module: &ModuleUnit,
    modules: &HashMap<String, &ModuleUnit>,
    packages: &PackageMetadataIndex,
    json_errors: bool,
) -> Result<ImportEnv> {
    let mut module_aliases = HashMap::with_capacity(module.program.imports.len());
    let mut imported_values = HashMap::new();
    let mut imported_types = HashMap::new();

    for import in &module.program.imports {
        let is_std = import.path.first().map(|seg| seg == "std").unwrap_or(false);
        let target_path = import.path.join("::");

        match &import.kind {
            ImportKind::Module { alias } => {
                let alias_name = alias
                    .clone()
                    .unwrap_or_else(|| import.path.last().cloned().unwrap_or_default());
                if alias_name.is_empty() || alias_name == "std" {
                    return Err(module_error(
                        "C020",
                        "invalid module alias".to_string(),
                        &module.file,
                        import.path_span,
                        json_errors,
                    ));
                }
                ensure_name_available(
                    module,
                    &module_aliases,
                    &imported_values,
                    &imported_types,
                    &alias_name,
                    import.span,
                    json_errors,
                )?;

                if is_std {
                    let std_index = std_metadata().map_err(|err| {
                        module_error(
                            "C027",
                            format!("invalid std metadata: {err:#}"),
                            &module.file,
                            import.path_span,
                            json_errors,
                        )
                    })?;
                    if std_index.module(&target_path).is_none() {
                        return Err(module_error(
                            "C020",
                            format!("unknown module `{}`", target_path),
                            &module.file,
                            import.path_span,
                            json_errors,
                        ));
                    }
                } else {
                    let has_local = modules.contains_key(&target_path);
                    let has_package = packages.has_module(&target_path);
                    if has_local && has_package {
                        return Err(module_error(
                            "C028",
                            format!(
                                "module `{}` is provided by both source files and package metadata",
                                target_path
                            ),
                            &module.file,
                            import.path_span,
                            json_errors,
                        ));
                    }
                    if !has_local && !has_package {
                        return Err(module_error(
                            "C020",
                            format!("unknown module `{}`", target_path),
                            &module.file,
                            import.path_span,
                            json_errors,
                        ));
                    }
                }

                module_aliases.insert(alias_name, target_path.clone());
            }
            ImportKind::Items { items } => {
                if is_std {
                    let std_index = std_metadata().map_err(|err| {
                        module_error(
                            "C027",
                            format!("invalid std metadata: {err:#}"),
                            &module.file,
                            import.path_span,
                            json_errors,
                        )
                    })?;
                    let Some(std_module) = std_index.module(&target_path) else {
                        return Err(module_error(
                            "C020",
                            format!("unknown module `{}`", target_path),
                            &module.file,
                            import.path_span,
                            json_errors,
                        ));
                    };
                    let source = ExportSource::Std(std_module);
                    for item in items {
                        import_item(
                            module,
                            &module_aliases,
                            &mut imported_values,
                            &mut imported_types,
                            &source,
                            &target_path,
                            &target_path,
                            &item.name,
                            item.span,
                            json_errors,
                        )?;
                    }
                } else {
                    let local_target = modules.get(&target_path);
                    let package_target = packages.module(&target_path);
                    match (local_target, package_target) {
                        (Some(_), Some(_)) => {
                            return Err(module_error(
                                "C028",
                                format!(
                                    "module `{}` is provided by both source files and package metadata",
                                    target_path
                                ),
                                &module.file,
                                import.path_span,
                                json_errors,
                            ));
                        }
                        (Some(target), None) => {
                            let source = ExportSource::Local(&target.exports);
                            for item in items {
                                import_item(
                                    module,
                                    &module_aliases,
                                    &mut imported_values,
                                    &mut imported_types,
                                    &source,
                                    &target_path,
                                    target_prefix(target),
                                    &item.name,
                                    item.span,
                                    json_errors,
                                )?;
                            }
                        }
                        (None, Some(target)) => {
                            let source = ExportSource::Package(target);
                            for item in items {
                                import_item(
                                    module,
                                    &module_aliases,
                                    &mut imported_values,
                                    &mut imported_types,
                                    &source,
                                    &target_path,
                                    &target_path,
                                    &item.name,
                                    item.span,
                                    json_errors,
                                )?;
                            }
                        }
                        (None, None) => {
                            return Err(module_error(
                                "C020",
                                format!("unknown module `{}`", target_path),
                                &module.file,
                                import.path_span,
                                json_errors,
                            ));
                        }
                    }
                }
            }
        }
    }

    Ok(ImportEnv {
        module_aliases,
        imported_values,
        imported_types,
    })
}

enum ExportSource<'a> {
    Std(&'a StdModuleIndex),
    Local(&'a Exports),
    Package(&'a PackageModuleIndex),
}

impl ExportSource<'_> {
    fn has_value(&self, name: &str) -> bool {
        match self {
            ExportSource::Std(module) => module.values.contains(name),
            ExportSource::Local(exports) => exports.values.contains(name),
            ExportSource::Package(module) => module.values.contains(name),
        }
    }

    fn has_type(&self, name: &str) -> bool {
        match self {
            ExportSource::Std(module) => module.types.contains(name),
            ExportSource::Local(exports) => exports.types.contains(name),
            ExportSource::Package(module) => module.types.contains(name),
        }
    }

    fn qualify(&self, module_path_or_prefix: &str, name: &str) -> String {
        match self {
            ExportSource::Std(_) => format!("{}::{}", module_path_or_prefix, name),
            ExportSource::Local(_) => qualify_name(module_path_or_prefix, name),
            ExportSource::Package(_) => format!("{}::{}", module_path_or_prefix, name),
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn import_item(
    module: &ModuleUnit,
    module_aliases: &HashMap<String, String>,
    imported_values: &mut HashMap<String, String>,
    imported_types: &mut HashMap<String, String>,
    source: &ExportSource<'_>,
    module_path: &str,
    module_prefix: &str,
    name: &str,
    span: Span,
    json_errors: bool,
) -> Result<()> {
    let is_value = source.has_value(name);
    let is_type = source.has_type(name);

    if !is_value && !is_type {
        return Err(module_error(
            "C021",
            format!("module `{}` does not export item `{}`", module_path, name),
            &module.file,
            span,
            json_errors,
        ));
    }
    if is_value && is_type {
        return Err(module_error(
            "C021",
            format!(
                "module `{}` exports `{}` as both value and type",
                module_path, name
            ),
            &module.file,
            span,
            json_errors,
        ));
    }

    ensure_name_available(
        module,
        module_aliases,
        imported_values,
        imported_types,
        name,
        span,
        json_errors,
    )?;

    let qualified = source.qualify(module_prefix, name);
    if is_value {
        imported_values.insert(name.to_string(), qualified);
    } else {
        imported_types.insert(name.to_string(), qualified);
    }

    Ok(())
}

fn ensure_name_available(
    module: &ModuleUnit,
    module_aliases: &HashMap<String, String>,
    imported_values: &HashMap<String, String>,
    imported_types: &HashMap<String, String>,
    name: &str,
    span: Span,
    json_errors: bool,
) -> Result<()> {
    if module.local_values.contains(name)
        || module.local_types.contains(name)
        || module_aliases.contains_key(name)
        || imported_values.contains_key(name)
        || imported_types.contains_key(name)
    {
        return Err(module_error(
            "C022",
            format!("import name `{}` conflicts with existing name", name),
            &module.file,
            span,
            json_errors,
        ));
    }

    Ok(())
}
