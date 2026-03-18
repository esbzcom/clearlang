use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;

use anyhow::{anyhow, Context, Result};
use clg_ast::{Effect, Param, ParamKind, Type};
use clg_typer::StdTypeInfo;
use serde::Deserialize;

use super::ExternalImportBinding;

pub(super) const LEGACY_PACKAGE_METADATA_FILE: &str = "clg-packages.json";
pub(super) const CANONICAL_PACKAGE_METADATA_FILE: &str = "clg.package-metadata.json";
pub(super) const PACKAGE_METADATA_FILE: &str = LEGACY_PACKAGE_METADATA_FILE;

#[derive(Deserialize)]
struct RawPackageMetadata {
    schema_version: u32,
    #[serde(default)]
    packages: Vec<RawPackage>,
}

#[derive(Deserialize)]
struct RawPackage {
    name: String,
    version: String,
    artifact: RawPackageArtifact,
    #[serde(default)]
    modules: Vec<RawModule>,
}

#[derive(Deserialize)]
struct RawPackageArtifact {
    format: String,
    path: String,
}

#[derive(Deserialize)]
struct RawModule {
    path: String,
    #[serde(default)]
    exports: Vec<RawExport>,
}

#[derive(Deserialize)]
struct RawExport {
    name: String,
    kind: RawExportKind,
    #[serde(default)]
    layout: Option<RawTypeLayout>,
    #[serde(default)]
    effect: Option<String>,
    #[serde(default)]
    params: Vec<RawParam>,
    #[serde(default)]
    ret: Option<String>,
    #[serde(default)]
    import: Option<RawImportTarget>,
}

#[derive(Deserialize)]
#[serde(rename_all = "lowercase")]
enum RawExportKind {
    Type,
    Value,
}

#[derive(Deserialize)]
struct RawTypeLayout {
    bytes: u32,
    align: u32,
}

#[derive(Deserialize)]
struct RawParam {
    name: String,
    #[serde(rename = "type")]
    ty: String,
    #[serde(default)]
    kind: Option<String>,
}

#[derive(Deserialize)]
struct RawImportTarget {
    module: String,
    name: String,
}

pub(super) struct PackageModuleIndex {
    pub(super) values: HashSet<String>,
    pub(super) types: HashSet<String>,
}

#[derive(Default)]
pub(super) struct PackageMetadataIndex {
    modules: HashMap<String, PackageModuleIndex>,
    external_imports: Vec<ExternalImportBinding>,
    type_layouts: HashMap<String, StdTypeInfo>,
}

impl PackageMetadataIndex {
    pub(super) fn load(root: &Path) -> Result<Self> {
        let metadata_path = root.join(PACKAGE_METADATA_FILE);
        let canonical_metadata_path = root.join(CANONICAL_PACKAGE_METADATA_FILE);
        if metadata_path.exists() && canonical_metadata_path.exists() {
            return Err(anyhow!(
                "package metadata migration conflict: both legacy `{}` and canonical `{}` exist; remove legacy `{}` and keep canonical strict metadata/ABI inputs only",
                LEGACY_PACKAGE_METADATA_FILE,
                CANONICAL_PACKAGE_METADATA_FILE,
                LEGACY_PACKAGE_METADATA_FILE
            ));
        }
        if !metadata_path.exists() {
            return Ok(Self::default());
        }
        let content = fs::read_to_string(&metadata_path)
            .with_context(|| format!("reading {}", metadata_path.display()))?;
        let raw: RawPackageMetadata = serde_json::from_str(&content)
            .with_context(|| format!("parsing {}", metadata_path.display()))?;
        if raw.schema_version != 1 {
            return Err(anyhow!(
                "unsupported package metadata schema version {} (expected 1)",
                raw.schema_version
            ));
        }

        let mut modules = HashMap::new();
        let mut external_imports = Vec::new();
        let mut type_layouts = HashMap::new();
        let mut seen_external_functions = HashSet::new();

        for package in raw.packages {
            validate_package_name(&package.name)?;
            validate_semver(&package.version).with_context(|| {
                format!(
                    "package `{}` has invalid version `{}`",
                    package.name, package.version
                )
            })?;
            validate_artifact(&package, root)?;

            for module in package.modules {
                validate_module_path(&module.path).with_context(|| {
                    format!(
                        "package `{}` has invalid module path `{}`",
                        package.name, module.path
                    )
                })?;
                if module.path.starts_with("std::") {
                    return Err(anyhow!(
                        "package module path `{}` is reserved for std",
                        module.path
                    ));
                }
                let root_segment = module.path.split("::").next().unwrap_or_default();
                if root_segment != package.name {
                    return Err(anyhow!(
                        "package `{}` module `{}` must start with `{}`",
                        package.name,
                        module.path,
                        package.name
                    ));
                }

                let mut values = HashSet::new();
                let mut types = HashSet::new();
                let mut export_names = HashSet::new();
                for export in module.exports {
                    validate_identifier(&export.name)?;
                    if !export_names.insert(export.name.clone()) {
                        return Err(anyhow!(
                            "duplicate export `{}` in module `{}`",
                            export.name,
                            module.path
                        ));
                    }
                    match export.kind {
                        RawExportKind::Type => {
                            let layout = export.layout.ok_or_else(|| {
                                anyhow!(
                                    "type export `{}` in `{}` is missing layout metadata",
                                    export.name,
                                    module.path
                                )
                            })?;
                            if layout.bytes == 0 || layout.align == 0 {
                                return Err(anyhow!(
                                    "type export `{}` in `{}` has invalid layout",
                                    export.name,
                                    module.path
                                ));
                            }
                            if !types.insert(export.name.clone()) {
                                return Err(anyhow!(
                                    "duplicate type export `{}` in `{}`",
                                    export.name,
                                    module.path
                                ));
                            }
                            let qualified = format!("{}::{}", module.path, export.name);
                            if type_layouts
                                .insert(
                                    qualified,
                                    StdTypeInfo {
                                        byte_len: layout.bytes,
                                        align: layout.align,
                                    },
                                )
                                .is_some()
                            {
                                return Err(anyhow!(
                                    "duplicate type export `{}` in package metadata",
                                    export.name
                                ));
                            }
                        }
                        RawExportKind::Value => {
                            let effect =
                                parse_effect(export.effect.as_deref()).with_context(|| {
                                    format!(
                                        "value export `{}` in `{}` has invalid effect",
                                        export.name, module.path
                                    )
                                })?;
                            let ret = parse_type(export.ret.as_deref().ok_or_else(|| {
                                anyhow!(
                                    "value export `{}` in `{}` is missing return type",
                                    export.name,
                                    module.path
                                )
                            })?)?;
                            let mut params = Vec::with_capacity(export.params.len());
                            for p in export.params {
                                validate_identifier(&p.name)?;
                                params.push(Param {
                                    kind: parse_param_kind(p.kind.as_deref()).with_context(
                                        || {
                                            format!(
                                            "value export `{}` in `{}` has invalid parameter kind",
                                            export.name, module.path
                                        )
                                        },
                                    )?,
                                    name: p.name,
                                    ty: parse_type(&p.ty).with_context(|| {
                                        format!(
                                            "value export `{}` in `{}` has invalid parameter type",
                                            export.name, module.path
                                        )
                                    })?,
                                });
                            }
                            let import = export.import.ok_or_else(|| {
                                anyhow!(
                                    "value export `{}` in `{}` is missing import target",
                                    export.name,
                                    module.path
                                )
                            })?;
                            if import.module.trim().is_empty() || import.name.trim().is_empty() {
                                return Err(anyhow!(
                                    "value export `{}` in `{}` has invalid import target",
                                    export.name,
                                    module.path
                                ));
                            }
                            if !values.insert(export.name.clone()) {
                                return Err(anyhow!(
                                    "duplicate value export `{}` in `{}`",
                                    export.name,
                                    module.path
                                ));
                            }
                            let function = format!("{}::{}", module.path, export.name);
                            if !seen_external_functions.insert(function.clone()) {
                                return Err(anyhow!(
                                    "duplicate value export `{}` in package metadata",
                                    function
                                ));
                            }
                            external_imports.push(ExternalImportBinding {
                                function,
                                import_module: import.module,
                                import_name: import.name,
                                params,
                                ret,
                                effect,
                            });
                        }
                    }
                }

                if modules
                    .insert(module.path, PackageModuleIndex { values, types })
                    .is_some()
                {
                    return Err(anyhow!("duplicate module path in package metadata"));
                }
            }
        }

        Ok(Self {
            modules,
            external_imports,
            type_layouts,
        })
    }

    pub(super) fn module(&self, path: &str) -> Option<&PackageModuleIndex> {
        self.modules.get(path)
    }

    pub(super) fn has_module(&self, path: &str) -> bool {
        self.modules.contains_key(path)
    }

    pub(super) fn external_imports(&self) -> &[ExternalImportBinding] {
        &self.external_imports
    }

    pub(super) fn merge_type_layouts(&self, out: &mut HashMap<String, StdTypeInfo>) -> Result<()> {
        for (name, info) in &self.type_layouts {
            if out.insert(name.clone(), info.clone()).is_some() {
                return Err(anyhow!(
                    "compiled package type `{}` conflicts with an existing type layout",
                    name
                ));
            }
        }
        Ok(())
    }
}

fn validate_package_name(name: &str) -> Result<()> {
    validate_identifier(name).with_context(|| format!("invalid package name `{name}`"))
}

fn validate_module_path(path: &str) -> Result<()> {
    let parts: Vec<&str> = path.split("::").collect();
    if parts.len() < 2 {
        return Err(anyhow!("module path must include at least one `::`"));
    }
    for part in parts {
        validate_identifier(part)?;
    }
    Ok(())
}

fn validate_identifier(name: &str) -> Result<()> {
    if name.is_empty() {
        return Err(anyhow!("identifier is empty"));
    }
    let mut chars = name.chars();
    let first = chars.next().unwrap();
    if !(first == '_' || first.is_ascii_alphabetic()) {
        return Err(anyhow!("identifier `{}` must start with [A-Za-z_]", name));
    }
    for ch in chars {
        if !(ch == '_' || ch.is_ascii_alphanumeric()) {
            return Err(anyhow!(
                "identifier `{}` has invalid character `{}`",
                name,
                ch
            ));
        }
    }
    Ok(())
}

fn validate_semver(version: &str) -> Result<()> {
    let core = version.split('-').next().unwrap_or(version);
    let parts: Vec<&str> = core.split('.').collect();
    if parts.len() != 3 {
        return Err(anyhow!("version must use MAJOR.MINOR.PATCH"));
    }
    for p in parts {
        if p.is_empty() || !p.chars().all(|ch| ch.is_ascii_digit()) {
            return Err(anyhow!("version segment `{}` is not numeric", p));
        }
    }
    Ok(())
}

fn validate_artifact(pkg: &RawPackage, root: &Path) -> Result<()> {
    if pkg.artifact.format != "wasm" {
        return Err(anyhow!(
            "package `{}` artifact format must be `wasm`",
            pkg.name
        ));
    }
    if pkg.artifact.path.trim().is_empty() {
        return Err(anyhow!("package `{}` artifact path is empty", pkg.name));
    }
    let artifact_path = root.join(&pkg.artifact.path);
    if !artifact_path.exists() {
        return Err(anyhow!(
            "package `{}` artifact path `{}` does not exist",
            pkg.name,
            pkg.artifact.path
        ));
    }
    if !artifact_path.is_file() {
        return Err(anyhow!(
            "package `{}` artifact path `{}` is not a file",
            pkg.name,
            pkg.artifact.path
        ));
    }
    Ok(())
}

fn parse_effect(value: Option<&str>) -> Result<Effect> {
    match value.unwrap_or("pure") {
        "pure" => Ok(Effect::Pure),
        "mut" => Ok(Effect::Mut),
        "io" => Ok(Effect::Io),
        "none" => Ok(Effect::None),
        other => Err(anyhow!("unsupported effect `{}`", other)),
    }
}

fn parse_param_kind(value: Option<&str>) -> Result<ParamKind> {
    match value.unwrap_or("borrow") {
        "borrow" => Ok(ParamKind::Borrow),
        "consume" => Ok(ParamKind::Consume),
        other => Err(anyhow!("unsupported parameter kind `{}`", other)),
    }
}

fn parse_type(raw: &str) -> Result<Type> {
    let value = raw.trim();
    if value.is_empty() {
        return Err(anyhow!("type is empty"));
    }
    if value.contains('<')
        || value.contains('>')
        || value.contains('[')
        || value.contains(']')
        || value.contains('(')
        || value.contains(')')
        || value.contains(',')
    {
        return Err(anyhow!(
            "type `{}` uses unsupported generic/compound syntax in package metadata v1",
            value
        ));
    }
    let ty = match value {
        "Int" => Type::Int,
        "Bool" => Type::Bool,
        "String" => Type::String,
        "Bytes" => Type::Bytes,
        "U8" => Type::U8,
        "U64" => Type::U64,
        "U128" => Type::U128,
        "U256" => Type::U256,
        other => {
            validate_module_path_like_type(other)?;
            Type::Named {
                name: other.to_string(),
                args: Vec::new(),
            }
        }
    };
    Ok(ty)
}

fn validate_module_path_like_type(value: &str) -> Result<()> {
    for part in value.split("::") {
        validate_identifier(part)?;
    }
    Ok(())
}
