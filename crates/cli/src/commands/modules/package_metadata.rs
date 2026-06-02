use std::collections::{BTreeSet, HashMap, HashSet};
use std::fs;
use std::path::{Component, Path};

use anyhow::{anyhow, Context, Result};
use clg_ast::{Effect, Param, ParamKind, Type};
use clg_typer::StdTypeInfo;
use serde::Deserialize;

use super::std_metadata::std_metadata;
use super::ExternalImportBinding;

pub(super) const LEGACY_PACKAGE_METADATA_FILE: &str = "clg-packages.json";
pub(super) const CANONICAL_PACKAGE_METADATA_FILE: &str = "clg.package-metadata.json";
pub(super) const CANONICAL_PACKAGE_ABI_FILE: &str = "clg.package-abi.json";
pub(super) const PACKAGE_METADATA_FILE: &str = CANONICAL_PACKAGE_METADATA_FILE;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawPackageMetadataRootV0 {
    schema_version: u32,
    #[serde(default)]
    packages: Vec<RawPackageV0>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawPackageV0 {
    name: String,
    version: String,
    digest: String,
    artifact: RawPackageArtifact,
    abi_id: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawPackageMetadataRootV1 {
    schema_version: u32,
    #[serde(default)]
    packages: Vec<RawPackageV1>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawPackageV1 {
    name: String,
    version: String,
    digest: String,
    artifact: RawPackageArtifact,
    abi_id: String,
    #[serde(default)]
    dependencies: Vec<RawPackageDependencyV1>,
    #[serde(default)]
    signature: Option<RawPackageSignatureV1>,
    #[serde(default)]
    trust: Option<RawPackageTrustV1>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawPackageDependencyV1 {
    name: String,
    requirement: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawPackageSignatureV1 {
    format: String,
    key_id: String,
    signed_at: String,
    signature: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawPackageTrustV1 {
    trusted_anchor_ids: Vec<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawPackageArtifact {
    format: String,
    path: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawAbiRoot {
    schema_version: u32,
    #[serde(default)]
    contracts: Vec<RawAbiContract>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawAbiContract {
    abi_id: String,
    package: String,
    version: String,
    #[serde(default)]
    imports: Vec<RawAbiImport>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawAbiImport {
    symbol: String,
    effect: String,
    #[serde(default)]
    params: Vec<String>,
    ret: String,
    #[serde(default)]
    capability: Option<String>,
}

struct CanonicalPackageEntry {
    name: String,
    version: String,
    artifact: RawPackageArtifact,
    abi_id: String,
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
        let legacy_path = root.join(LEGACY_PACKAGE_METADATA_FILE);
        let metadata_path = root.join(CANONICAL_PACKAGE_METADATA_FILE);
        let abi_path = root.join(CANONICAL_PACKAGE_ABI_FILE);

        if legacy_path.exists() {
            return Err(anyhow!(
                "legacy package metadata `{}` is no longer supported in standard/permissive import indexing; use canonical `{}` + `{}`",
                LEGACY_PACKAGE_METADATA_FILE,
                CANONICAL_PACKAGE_METADATA_FILE,
                CANONICAL_PACKAGE_ABI_FILE
            ));
        }

        if !metadata_path.exists() && !abi_path.exists() {
            return Ok(Self::default());
        }
        if !metadata_path.exists() {
            return Err(anyhow!(
                "missing canonical package metadata `{}` required by `{}`",
                CANONICAL_PACKAGE_METADATA_FILE,
                CANONICAL_PACKAGE_ABI_FILE
            ));
        }
        if !abi_path.exists() {
            return Err(anyhow!(
                "missing canonical package ABI `{}` required by `{}`",
                CANONICAL_PACKAGE_ABI_FILE,
                CANONICAL_PACKAGE_METADATA_FILE
            ));
        }

        let metadata_content = fs::read_to_string(&metadata_path)
            .with_context(|| format!("reading {}", metadata_path.display()))?;
        let metadata_value: serde_json::Value = serde_json::from_str(&metadata_content)
            .with_context(|| format!("parsing {}", metadata_path.display()))?;
        let schema_version = metadata_value
            .get("schema_version")
            .and_then(|value| value.as_u64())
            .ok_or_else(|| {
                anyhow!(
                    "{} is missing integer `schema_version`",
                    metadata_path.display()
                )
            })?;

        let packages = match schema_version {
            0 => {
                let root_v0: RawPackageMetadataRootV0 = serde_json::from_value(metadata_value)
                    .with_context(|| {
                        format!(
                            "parsing {} as canonical package metadata schema v0",
                            metadata_path.display()
                        )
                    })?;
                debug_assert_eq!(root_v0.schema_version, 0);
                root_v0
                    .packages
                    .into_iter()
                    .map(|entry| {
                        let _ = entry.digest;
                        CanonicalPackageEntry {
                            name: entry.name,
                            version: entry.version,
                            artifact: entry.artifact,
                            abi_id: entry.abi_id,
                        }
                    })
                    .collect::<Vec<_>>()
            }
            1 => {
                let root_v1: RawPackageMetadataRootV1 = serde_json::from_value(metadata_value)
                    .with_context(|| {
                        format!(
                            "parsing {} as canonical package metadata schema v1",
                            metadata_path.display()
                        )
                    })?;
                debug_assert_eq!(root_v1.schema_version, 1);
                root_v1
                    .packages
                    .into_iter()
                    .map(|entry| {
                        let _ = entry.digest;
                        for dep in &entry.dependencies {
                            let _ = (&dep.name, &dep.requirement);
                        }
                        if let Some(signature) = entry.signature.as_ref() {
                            let _ = (
                                &signature.format,
                                &signature.key_id,
                                &signature.signed_at,
                                &signature.signature,
                            );
                        }
                        if let Some(trust) = entry.trust.as_ref() {
                            let _ = &trust.trusted_anchor_ids;
                        }
                        CanonicalPackageEntry {
                            name: entry.name,
                            version: entry.version,
                            artifact: entry.artifact,
                            abi_id: entry.abi_id,
                        }
                    })
                    .collect::<Vec<_>>()
            }
            other => {
                return Err(anyhow!(
                    "unsupported package metadata schema version {} (expected 0 or 1)",
                    other
                ));
            }
        };

        let mut package_name_dupes = BTreeSet::new();
        let mut seen_package_names = HashSet::new();
        let mut abi_id_dupes = BTreeSet::new();
        let mut packages_by_abi: HashMap<String, (String, String)> =
            HashMap::with_capacity(packages.len());

        for package in packages {
            validate_package_name(&package.name)?;
            validate_semver(&package.version).with_context(|| {
                format!(
                    "package `{}` has invalid version `{}`",
                    package.name, package.version
                )
            })?;
            validate_artifact(&package.name, &package.artifact, root)?;
            if package.abi_id.trim().is_empty() {
                return Err(anyhow!("package `{}` has empty abi_id", package.name));
            }
            if !seen_package_names.insert(package.name.clone()) {
                package_name_dupes.insert(package.name.clone());
            }
            if packages_by_abi
                .insert(
                    package.abi_id.clone(),
                    (package.name.clone(), package.version.clone()),
                )
                .is_some()
            {
                abi_id_dupes.insert(package.abi_id.clone());
            }
        }

        if let Some(dupe) = package_name_dupes.iter().next() {
            return Err(anyhow!(
                "duplicate package name `{}` in package metadata",
                dupe
            ));
        }
        if let Some(dupe) = abi_id_dupes.iter().next() {
            return Err(anyhow!("duplicate abi_id `{}` in package metadata", dupe));
        }

        let abi_content = fs::read_to_string(&abi_path)
            .with_context(|| format!("reading {}", abi_path.display()))?;
        let raw_abi: RawAbiRoot = serde_json::from_str(&abi_content)
            .with_context(|| format!("parsing {}", abi_path.display()))?;
        if raw_abi.schema_version != 0 {
            return Err(anyhow!(
                "unsupported package ABI schema version {} (expected 0)",
                raw_abi.schema_version
            ));
        }

        let mut seen_contract_ids = HashSet::with_capacity(raw_abi.contracts.len());
        let mut duplicate_contract_ids = BTreeSet::new();
        for contract in &raw_abi.contracts {
            if contract.abi_id.trim().is_empty() {
                return Err(anyhow!("package ABI contract has empty abi_id"));
            }
            if !seen_contract_ids.insert(contract.abi_id.clone()) {
                duplicate_contract_ids.insert(contract.abi_id.clone());
            }
        }
        if let Some(dupe) = duplicate_contract_ids.iter().next() {
            return Err(anyhow!("duplicate ABI contract abi_id `{}`", dupe));
        }

        let mut modules = HashMap::new();
        let mut external_imports = Vec::new();
        let mut seen_external_functions = HashSet::new();
        let mut contract_ids_used = HashSet::new();

        for contract in raw_abi.contracts {
            let Some((package_name, package_version)) = packages_by_abi.get(&contract.abi_id)
            else {
                return Err(anyhow!(
                    "ABI contract `{}` is not referenced by package metadata",
                    contract.abi_id
                ));
            };
            if &contract.package != package_name {
                return Err(anyhow!(
                    "ABI contract `{}` package `{}` does not match metadata package `{}`",
                    contract.abi_id,
                    contract.package,
                    package_name
                ));
            }
            if &contract.version != package_version {
                return Err(anyhow!(
                    "ABI contract `{}` version `{}` does not match metadata version `{}`",
                    contract.abi_id,
                    contract.version,
                    package_version
                ));
            }
            contract_ids_used.insert(contract.abi_id.clone());

            for import in contract.imports {
                let RawAbiImport {
                    symbol,
                    effect,
                    params: raw_params,
                    ret: raw_ret,
                    capability,
                } = import;

                if let Some(capability) = capability.as_ref() {
                    if capability.trim().is_empty() {
                        return Err(anyhow!(
                            "ABI contract `{}` symbol `{}` has empty capability",
                            contract.abi_id,
                            symbol
                        ));
                    }
                }

                let (module_path, export_name) = split_symbol(symbol.as_str())?;
                let module_path = module_path.to_string();
                let export_name = export_name.to_string();
                validate_module_path(module_path.as_str()).with_context(|| {
                    format!(
                        "ABI contract `{}` symbol `{}` has invalid module path",
                        contract.abi_id, symbol
                    )
                })?;
                validate_identifier(export_name.as_str()).with_context(|| {
                    format!(
                        "ABI contract `{}` symbol `{}` has invalid exported name",
                        contract.abi_id, symbol
                    )
                })?;

                let effect = parse_effect(Some(effect.as_str())).with_context(|| {
                    format!(
                        "ABI contract `{}` symbol `{}` has invalid effect",
                        contract.abi_id, symbol
                    )
                })?;
                let ret = parse_type(raw_ret.as_str()).with_context(|| {
                    format!(
                        "ABI contract `{}` symbol `{}` has invalid return type",
                        contract.abi_id, symbol
                    )
                })?;
                let mut params = Vec::with_capacity(raw_params.len());
                for (idx, ty) in raw_params.iter().enumerate() {
                    params.push(Param {
                        kind: ParamKind::Borrow,
                        name: format!("p{}", idx),
                        ty: parse_type(ty).with_context(|| {
                            format!(
                                "ABI contract `{}` symbol `{}` has invalid parameter type",
                                contract.abi_id, symbol
                            )
                        })?,
                    });
                }

                if !seen_external_functions.insert(symbol.clone()) {
                    return Err(anyhow!(
                        "duplicate value export `{}` in package ABI contracts",
                        symbol
                    ));
                }

                let module =
                    modules
                        .entry(module_path.clone())
                        .or_insert_with(|| PackageModuleIndex {
                            values: HashSet::new(),
                            types: HashSet::new(),
                        });
                if !module.values.insert(export_name.clone()) {
                    return Err(anyhow!(
                        "duplicate value export `{}` in module `{}`",
                        export_name,
                        module_path
                    ));
                }

                external_imports.push(ExternalImportBinding {
                    function: symbol,
                    import_module: module_path,
                    import_name: export_name,
                    params,
                    ret,
                    effect,
                    route: clg_typer::BuiltinRoute::PackageImport,
                });
            }
        }

        let mut missing_contract_ids = packages_by_abi
            .keys()
            .filter(|abi_id| !contract_ids_used.contains(*abi_id))
            .cloned()
            .collect::<Vec<_>>();
        missing_contract_ids.sort();
        if let Some(missing) = missing_contract_ids.first() {
            return Err(anyhow!(
                "package metadata references abi_id `{}` but package ABI is missing that contract",
                missing
            ));
        }

        Ok(Self {
            modules,
            external_imports,
            type_layouts: HashMap::new(),
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
            match out.get(name) {
                Some(existing)
                    if existing.byte_len == info.byte_len && existing.align == info.align => {}
                Some(_) => {
                    return Err(anyhow!(
                        "compiled package type `{}` conflicts with an existing type layout",
                        name
                    ));
                }
                None => {
                    out.insert(name.clone(), info.clone());
                }
            }
        }
        Ok(())
    }

    pub(super) fn extend_external_imports(
        &mut self,
        imports: impl IntoIterator<Item = ExternalImportBinding>,
    ) -> Result<()> {
        for binding in imports {
            if let Some(existing) = self
                .external_imports
                .iter()
                .find(|existing| existing.function == binding.function)
            {
                if external_import_bindings_match(existing, &binding) {
                    continue;
                }
                return Err(anyhow!(
                    "duplicate value export `{}` in package ABI contracts",
                    binding.function
                ));
            }

            let module = self
                .modules
                .entry(binding.import_module.clone())
                .or_insert_with(|| PackageModuleIndex {
                    values: HashSet::new(),
                    types: HashSet::new(),
                });
            module.values.insert(binding.import_name.clone());

            self.external_imports.push(binding);
        }

        Ok(())
    }

    pub(super) fn extend_std_modules(
        &mut self,
        modules: impl IntoIterator<Item = String>,
    ) -> Result<()> {
        let std_index = std_metadata()?;
        for module_path in modules {
            let std_module = std_index
                .module(module_path.as_str())
                .ok_or_else(|| anyhow!("bundled std metadata is missing module `{module_path}`"))?;
            let entry =
                self.modules
                    .entry(module_path.clone())
                    .or_insert_with(|| PackageModuleIndex {
                        values: HashSet::new(),
                        types: HashSet::new(),
                    });
            entry.values.extend(std_module.values.iter().cloned());
            for ty in &std_module.types {
                entry.types.insert(ty.clone());
                let qualified = format!("{}::{}", module_path, ty);
                let Some(layout) = std_index.type_layout(qualified.as_str()) else {
                    return Err(anyhow!(
                        "bundled std metadata is missing layout for type `{qualified}`"
                    ));
                };
                match self.type_layouts.get(qualified.as_str()) {
                    Some(existing)
                        if existing.byte_len == layout.byte_len
                            && existing.align == layout.align => {}
                    Some(_) => {
                        return Err(anyhow!(
                            "bundled std type `{qualified}` conflicts with an existing type layout"
                        ));
                    }
                    None => {
                        self.type_layouts.insert(qualified, layout.clone());
                    }
                }
            }
        }
        Ok(())
    }
}

fn external_import_bindings_match(
    left: &ExternalImportBinding,
    right: &ExternalImportBinding,
) -> bool {
    left.function == right.function
        && left.import_module == right.import_module
        && left.import_name == right.import_name
        && left.params.len() == right.params.len()
        && left
            .params
            .iter()
            .zip(right.params.iter())
            .all(|(lhs, rhs)| lhs.kind == rhs.kind && lhs.ty == rhs.ty)
        && left.ret == right.ret
        && left.effect == right.effect
}

fn split_symbol(symbol: &str) -> Result<(&str, &str)> {
    let (module_path, name) = symbol
        .rsplit_once("::")
        .ok_or_else(|| anyhow!("symbol `{}` must use `module::name` format", symbol))?;
    if module_path.trim().is_empty() || name.trim().is_empty() {
        return Err(anyhow!(
            "symbol `{}` must use non-empty `module::name` segments",
            symbol
        ));
    }
    Ok((module_path, name))
}

fn validate_package_name(name: &str) -> Result<()> {
    validate_module_path_like_type(name).with_context(|| format!("invalid package name `{name}`"))
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
    if version.contains('-') || version.contains('+') {
        return Err(anyhow!(
            "version must use exact MAJOR.MINOR.PATCH without pre-release/build metadata"
        ));
    }
    let parts: Vec<&str> = version.split('.').collect();
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

fn validate_artifact(name: &str, artifact: &RawPackageArtifact, _root: &Path) -> Result<()> {
    if artifact.format != "wasm" {
        return Err(anyhow!("package `{}` artifact format must be `wasm`", name));
    }
    if artifact.path.trim().is_empty() {
        return Err(anyhow!("package `{}` artifact path is empty", name));
    }
    let artifact_path = Path::new(&artifact.path);
    if artifact_path.is_absolute() {
        return Err(anyhow!(
            "package `{}` artifact path `{}` must be relative",
            name,
            artifact.path
        ));
    }
    for component in artifact_path.components() {
        match component {
            Component::ParentDir => {
                return Err(anyhow!(
                    "package `{}` artifact path `{}` must not contain parent-directory traversal (`..`)",
                    name,
                    artifact.path
                ));
            }
            Component::Prefix(_) | Component::RootDir => {
                return Err(anyhow!(
                    "package `{}` artifact path `{}` must be relative",
                    name,
                    artifact.path
                ));
            }
            Component::CurDir | Component::Normal(_) => {}
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use clg_ast::{Effect, Param, ParamKind, Type};
    use clg_typer::BuiltinRoute;

    use super::{ExternalImportBinding, PackageMetadataIndex};

    #[test]
    fn extend_external_imports_indexes_modules_and_bindings() {
        let mut index = PackageMetadataIndex::default();
        index
            .extend_external_imports([ExternalImportBinding {
                function: "std::str::len".to_string(),
                import_module: "std::str".to_string(),
                import_name: "len".to_string(),
                params: vec![Param {
                    kind: ParamKind::Borrow,
                    name: "s".to_string(),
                    ty: Type::String,
                }],
                ret: Type::Int,
                effect: Effect::Pure,
                route: BuiltinRoute::Intrinsic,
            }])
            .expect("bundled external import should index cleanly");

        let module = index
            .module("std::str")
            .expect("std::str module should exist");
        assert!(module.values.contains("len"));
        assert_eq!(index.external_imports().len(), 1);
    }

    #[test]
    fn extend_external_imports_accepts_identical_duplicate_symbols() {
        let mut index = PackageMetadataIndex::default();
        let binding = ExternalImportBinding {
            function: "std::str::len".to_string(),
            import_module: "std::str".to_string(),
            import_name: "len".to_string(),
            params: vec![Param {
                kind: ParamKind::Borrow,
                name: "s".to_string(),
                ty: Type::String,
            }],
            ret: Type::Int,
            effect: Effect::Pure,
            route: BuiltinRoute::Intrinsic,
        };
        index
            .extend_external_imports([binding.clone()])
            .expect("first import should succeed");
        index
            .extend_external_imports([binding])
            .expect("identical duplicate import should be treated as idempotent");
        assert_eq!(index.external_imports().len(), 1);
    }

    #[test]
    fn extend_external_imports_rejects_conflicting_duplicate_symbols() {
        let mut index = PackageMetadataIndex::default();
        index
            .extend_external_imports([ExternalImportBinding {
                function: "std::str::len".to_string(),
                import_module: "std::str".to_string(),
                import_name: "len".to_string(),
                params: vec![Param {
                    kind: ParamKind::Borrow,
                    name: "s".to_string(),
                    ty: Type::String,
                }],
                ret: Type::Int,
                effect: Effect::Pure,
                route: BuiltinRoute::Intrinsic,
            }])
            .expect("first import should succeed");
        let err = index
            .extend_external_imports([ExternalImportBinding {
                function: "std::str::len".to_string(),
                import_module: "std::str".to_string(),
                import_name: "len".to_string(),
                params: vec![Param {
                    kind: ParamKind::Borrow,
                    name: "s".to_string(),
                    ty: Type::Bytes,
                }],
                ret: Type::Int,
                effect: Effect::Pure,
                route: BuiltinRoute::Intrinsic,
            }])
            .expect_err("conflicting duplicate import should fail");
        assert!(
            err.to_string()
                .contains("duplicate value export `std::str::len`"),
            "expected duplicate symbol rejection, got: {err}"
        );
    }

    #[test]
    fn extend_std_modules_indexes_types_and_layouts() {
        let mut index = PackageMetadataIndex::default();
        index
            .extend_std_modules(["std::encoder".to_string()])
            .expect("bundled std module should index cleanly");

        let module = index
            .module("std::encoder")
            .expect("std::encoder module should exist");
        assert!(module.values.contains("new"));
        assert!(module.values.contains("finish"));
        assert!(module.types.contains("Encoder"));

        let mut layouts = std::collections::HashMap::new();
        index
            .merge_type_layouts(&mut layouts)
            .expect("type layouts should merge cleanly");
        let layout = layouts
            .get("std::encoder::Encoder")
            .expect("encoder layout should exist");
        assert_eq!(layout.byte_len, 4);
        assert_eq!(layout.align, 4);
    }

    #[test]
    fn extend_external_imports_accepts_preloaded_bundled_std_values() {
        let mut index = PackageMetadataIndex::default();
        index
            .extend_std_modules(["std::bytes".to_string()])
            .expect("bundled std module should preload cleanly");
        index
            .extend_external_imports([ExternalImportBinding {
                function: "std::bytes::len".to_string(),
                import_module: "std::bytes".to_string(),
                import_name: "len".to_string(),
                params: vec![Param {
                    kind: ParamKind::Borrow,
                    name: "b".to_string(),
                    ty: Type::Bytes,
                }],
                ret: Type::Int,
                effect: Effect::Pure,
                route: BuiltinRoute::Intrinsic,
            }])
            .expect("preloaded bundled std value should accept external binding attachment");
        assert_eq!(index.external_imports().len(), 1);
    }

    #[test]
    fn merge_type_layouts_accepts_identical_preloaded_std_layouts() {
        let mut index = PackageMetadataIndex::default();
        index
            .extend_std_modules(["std::encoder".to_string()])
            .expect("bundled std module should preload cleanly");

        let mut layouts = std::collections::HashMap::new();
        layouts.insert(
            "std::encoder::Encoder".to_string(),
            clg_typer::StdTypeInfo {
                byte_len: 4,
                align: 4,
            },
        );
        index
            .merge_type_layouts(&mut layouts)
            .expect("identical preloaded layout should merge idempotently");
        assert_eq!(layouts.len(), 1);
    }
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
        || value.contains(';')
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
