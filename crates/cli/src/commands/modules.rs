use std::collections::{HashMap, HashSet, VecDeque};
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use anyhow::{anyhow, Context, Result};
use clg_ast::{
    Block, Expr, Func, ImportKind, MatchArm, MatchPat, Program, Span, Stmt, TraitBound, Type,
};
use clg_parser::{parse as parse_src, parse_errors as parse_src_errs};
use clg_typer::StdTypeInfo;
use serde::Deserialize;

use super::helpers::{make_parse_json_error, make_single_json_error, CommandError};

struct Exports {
    values: HashSet<String>,
    types: HashSet<String>,
}

struct ModuleUnit {
    path_str: String,
    file: PathBuf,
    program: Program,
    is_entry: bool,
    exports: Exports,
    local_values: HashSet<String>,
    local_types: HashSet<String>,
}

struct ImportEnv {
    module_aliases: HashMap<String, String>,
    imported_values: HashMap<String, String>,
    imported_types: HashMap<String, String>,
}

#[derive(Deserialize)]
struct StdMetadata {
    schema_version: u32,
    modules: Vec<StdModule>,
}

#[derive(Deserialize)]
struct StdModule {
    path: String,
    exports: Vec<StdExport>,
}

#[derive(Deserialize)]
struct StdExport {
    name: String,
    kind: StdExportKind,
    #[serde(default)]
    layout: Option<StdTypeLayout>,
}

#[derive(Deserialize, Clone, Copy)]
#[serde(rename_all = "lowercase")]
enum StdExportKind {
    Type,
    Value,
}

#[derive(Deserialize, Clone)]
struct StdTypeLayout {
    bytes: u32,
    align: u32,
}

struct StdModuleIndex {
    values: HashSet<String>,
    types: HashSet<String>,
}

struct StdMetadataIndex {
    modules: HashMap<String, StdModuleIndex>,
    types: HashMap<String, StdTypeInfo>,
}

impl StdMetadataIndex {
    fn load() -> Self {
        let raw: StdMetadata = serde_json::from_str(include_str!("../../assets/std-metadata.json"))
            .expect("invalid std metadata");
        if raw.schema_version != 2 {
            panic!(
                "unsupported std metadata schema version {}",
                raw.schema_version
            );
        }
        let mut modules = HashMap::new();
        let mut types = HashMap::new();
        for module in raw.modules {
            let mut values = HashSet::new();
            let mut types_set = HashSet::new();
            for export in module.exports {
                match export.kind {
                    StdExportKind::Value => {
                        values.insert(export.name);
                    }
                    StdExportKind::Type => {
                        let layout = export.layout.unwrap_or_else(|| {
                            panic!("std type `{}` is missing layout metadata", export.name)
                        });
                        if layout.bytes == 0 {
                            panic!("std type `{}` has zero-byte layout", export.name);
                        }
                        if layout.align == 0 {
                            panic!("std type `{}` has zero alignment", export.name);
                        }
                        let qualified = format!("{}::{}", module.path, export.name);
                        if types
                            .insert(
                                qualified,
                                StdTypeInfo {
                                    byte_len: layout.bytes,
                                    align: layout.align,
                                },
                            )
                            .is_some()
                        {
                            panic!("duplicate std type `{}`", export.name);
                        }
                        types_set.insert(export.name);
                    }
                }
            }
            modules.insert(
                module.path,
                StdModuleIndex {
                    values,
                    types: types_set,
                },
            );
        }
        StdMetadataIndex { modules, types }
    }

    fn module(&self, path: &str) -> Option<&StdModuleIndex> {
        self.modules.get(path)
    }
}

fn std_metadata() -> &'static StdMetadataIndex {
    static STD_METADATA: OnceLock<StdMetadataIndex> = OnceLock::new();
    STD_METADATA.get_or_init(StdMetadataIndex::load)
}

pub fn std_type_info() -> HashMap<String, StdTypeInfo> {
    std_metadata().types.clone()
}

struct ResolveCtx<'a> {
    prefix: &'a str,
    local_values: &'a HashMap<String, String>,
    local_types: &'a HashMap<String, String>,
    imported_values: &'a HashMap<String, String>,
    imported_types: &'a HashMap<String, String>,
    module_aliases: &'a HashMap<String, String>,
}

pub fn load_program(entry: &Path, json_errors: bool) -> Result<Program> {
    let root = entry.parent().unwrap_or_else(|| Path::new("."));
    let root = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());
    let entry_abs = entry.canonicalize().unwrap_or_else(|_| entry.to_path_buf());
    let mut modules: Vec<ModuleUnit> = Vec::new();
    let mut by_path: HashMap<String, usize> = HashMap::new();
    let mut module_files: HashMap<String, PathBuf> = HashMap::new();
    let mut queue: VecDeque<PathBuf> = VecDeque::new();
    queue.push_back(entry_abs.clone());

    while let Some(file) = queue.pop_front() {
        let is_entry = same_path(&entry_abs, &file);
        let path = if is_entry {
            Vec::new()
        } else {
            module_path_for(&root, &file)?
        };
        let path_str = path.join("::");
        if let Some(existing) = module_files.get(&path_str) {
            if !same_path(existing, &file) {
                return Err(module_error(
                    "C024",
                    format!(
                        "module `{}` is defined by multiple files (`{}` and `{}`)",
                        path_str,
                        existing.display(),
                        file.display()
                    ),
                    &file,
                    Span { start: 0, end: 0 },
                    json_errors,
                ));
            }
            continue;
        }

        let program = parse_file(&file, json_errors)?;

        if !is_entry {
            if let Some(first) = path.first() {
                if first == "std" {
                    return Err(module_error(
                        "C026",
                        format!("module path `{}` is reserved for std", path_str),
                        &file,
                        Span { start: 0, end: 0 },
                        json_errors,
                    ));
                }
            }
        }

        if let Some(decl) = &program.module {
            if is_entry {
                return Err(module_error(
                    "C023",
                    "entry file cannot declare a module header".to_string(),
                    &file,
                    decl.span,
                    json_errors,
                ));
            }
            if decl.path != path {
                return Err(module_error(
                    "C023",
                    format!(
                        "module header does not match file path (expected `{}`)",
                        path_str
                    ),
                    &file,
                    decl.span,
                    json_errors,
                ));
            }
        }

        if by_path.contains_key(&path_str) {
            continue;
        }

        for import in &program.imports {
            if let Some(first) = import.path.first() {
                if first == "std" {
                    let import_file = module_file_for(&root, &import.path);
                    if import_file.exists() {
                        return Err(module_error(
                            "C026",
                            format!(
                                "module path `{}` is reserved for std",
                                import.path.join("::")
                            ),
                            &file,
                            import.path_span,
                            json_errors,
                        ));
                    }
                    continue;
                }
            }
            let import_file = module_file_for(&root, &import.path);
            if !import_file.exists() {
                return Err(module_error(
                    "C020",
                    format!("unknown module `{}`", import.path.join("::")),
                    &file,
                    import.path_span,
                    json_errors,
                ));
            }
            queue.push_back(import_file);
        }

        let (local_values, local_types) = collect_locals(&program);
        let exports = collect_exports(&program);

        by_path.insert(path_str.clone(), modules.len());
        module_files.insert(path_str.clone(), file.clone());
        modules.push(ModuleUnit {
            path_str,
            file,
            program,
            is_entry,
            exports,
            local_values,
            local_types,
        });
    }

    let mut resolved = Program {
        module: None,
        imports: Vec::new(),
        refined_aliases: Vec::new(),
        resources: Vec::new(),
        structs: Vec::new(),
        enums: Vec::new(),
        traits: Vec::new(),
        impls: Vec::new(),
        funcs: Vec::new(),
    };

    let mut modules_by_name: HashMap<String, &ModuleUnit> = HashMap::new();
    for module in &modules {
        modules_by_name.insert(module.path_str.clone(), module);
    }

    detect_cycles(&modules, json_errors)?;

    for module in &modules {
        let env = build_import_env(module, &modules_by_name, json_errors)?;
        let mut program = resolve_program(module, &env);
        resolved
            .refined_aliases
            .append(&mut program.refined_aliases);
        resolved.resources.append(&mut program.resources);
        resolved.structs.append(&mut program.structs);
        resolved.enums.append(&mut program.enums);
        resolved.traits.append(&mut program.traits);
        resolved.impls.append(&mut program.impls);
        resolved.funcs.append(&mut program.funcs);
    }

    Ok(resolved)
}

fn parse_file(path: &Path, json_errors: bool) -> Result<Program> {
    let mut src = String::new();
    fs::File::open(path)
        .with_context(|| format!("opening {}", path.display()))?
        .read_to_string(&mut src)
        .with_context(|| format!("reading {}", path.display()))?;
    if json_errors {
        match parse_src_errs(&src) {
            Ok(ast) => Ok(ast),
            Err(errs) => {
                let json = make_parse_json_error(path, &errs);
                Err(CommandError::json(json).into())
            }
        }
    } else {
        parse_src(&src).map_err(|e| anyhow!("parse failed: {}", e))
    }
}

fn module_path_for(root: &Path, file: &Path) -> Result<Vec<String>> {
    let rel = file.strip_prefix(root).unwrap_or(file).to_path_buf();
    let mut parts: Vec<String> = Vec::new();
    let mut components = rel.components().peekable();
    while let Some(comp) = components.next() {
        if components.peek().is_none() {
            break;
        }
        let name = comp
            .as_os_str()
            .to_str()
            .ok_or_else(|| anyhow!("non-utf8 module path component"))?;
        parts.push(name.to_string());
    }
    let stem = file
        .file_stem()
        .and_then(|s| s.to_str())
        .ok_or_else(|| anyhow!("module file name is not valid utf-8"))?;
    parts.push(stem.to_string());
    Ok(parts)
}

fn module_file_for(root: &Path, path: &[String]) -> PathBuf {
    let mut out = PathBuf::from(root);
    for segment in path {
        out.push(segment);
    }
    out.set_extension("clear");
    out
}

fn detect_cycles(modules: &[ModuleUnit], json_errors: bool) -> Result<()> {
    let mut edges: HashMap<String, Vec<(String, Span, PathBuf)>> = HashMap::new();
    for module in modules {
        let from = module.path_str.clone();
        for import in &module.program.imports {
            if let Some(first) = import.path.first() {
                if first == "std" {
                    continue;
                }
            }
            let target = import.path.join("::");
            edges
                .entry(from.clone())
                .or_default()
                .push((target, import.span, module.file.clone()));
        }
    }

    let mut state: HashMap<String, u8> = HashMap::new();
    let mut stack: Vec<String> = Vec::new();
    let mut stack_set: HashMap<String, usize> = HashMap::new();

    for module in modules {
        let name = module.path_str.clone();
        if state.get(&name).copied().unwrap_or(0) == 0 {
            dfs_cycle(
                &name,
                &edges,
                &mut state,
                &mut stack,
                &mut stack_set,
                json_errors,
            )?;
        }
    }
    Ok(())
}

fn dfs_cycle(
    node: &str,
    edges: &HashMap<String, Vec<(String, Span, PathBuf)>>,
    state: &mut HashMap<String, u8>,
    stack: &mut Vec<String>,
    stack_set: &mut HashMap<String, usize>,
    json_errors: bool,
) -> Result<()> {
    state.insert(node.to_string(), 1);
    stack_set.insert(node.to_string(), stack.len());
    stack.push(node.to_string());

    if let Some(out_edges) = edges.get(node) {
        for (target, span, file) in out_edges {
            match state.get(target).copied().unwrap_or(0) {
                0 => {
                    dfs_cycle(target, edges, state, stack, stack_set, json_errors)?;
                }
                1 => {
                    let start_idx = stack_set.get(target).copied().unwrap_or(0);
                    let mut cycle = stack[start_idx..].to_vec();
                    cycle.push(target.clone());
                    let msg = format!("import cycle detected: {}", cycle.join(" -> "));
                    return Err(module_error("C025", msg, file, *span, json_errors));
                }
                _ => {}
            }
        }
    }

    stack.pop();
    stack_set.remove(node);
    state.insert(node.to_string(), 2);
    Ok(())
}

fn same_path(a: &Path, b: &Path) -> bool {
    let a_abs = a.canonicalize();
    let b_abs = b.canonicalize();
    match (a_abs, b_abs) {
        (Ok(a_abs), Ok(b_abs)) => a_abs == b_abs,
        (Ok(a_abs), Err(_)) => a_abs == b,
        (Err(_), Ok(b_abs)) => a == b_abs,
        (Err(_), Err(_)) => a == b,
    }
}

fn collect_locals(program: &Program) -> (HashSet<String>, HashSet<String>) {
    let mut values = HashSet::new();
    for func in &program.funcs {
        values.insert(func.name.clone());
    }
    let mut types = HashSet::new();
    for alias in &program.refined_aliases {
        types.insert(alias.name.clone());
    }
    for res in &program.resources {
        types.insert(res.name.clone());
    }
    for strukt in &program.structs {
        types.insert(strukt.name.clone());
    }
    for enm in &program.enums {
        types.insert(enm.name.clone());
    }
    for tr in &program.traits {
        types.insert(tr.name.clone());
    }
    (values, types)
}

fn collect_exports(program: &Program) -> Exports {
    let mut values = HashSet::new();
    for func in &program.funcs {
        if func.is_exported {
            values.insert(func.name.clone());
        }
    }
    let mut types = HashSet::new();
    for alias in &program.refined_aliases {
        if alias.is_exported {
            types.insert(alias.name.clone());
        }
    }
    for res in &program.resources {
        if res.is_exported {
            types.insert(res.name.clone());
        }
    }
    for strukt in &program.structs {
        if strukt.is_exported {
            types.insert(strukt.name.clone());
        }
    }
    for enm in &program.enums {
        if enm.is_exported {
            types.insert(enm.name.clone());
        }
    }
    for tr in &program.traits {
        if tr.is_exported {
            types.insert(tr.name.clone());
        }
    }
    Exports { values, types }
}

fn build_import_env(
    module: &ModuleUnit,
    modules: &HashMap<String, &ModuleUnit>,
    json_errors: bool,
) -> Result<ImportEnv> {
    let mut module_aliases = HashMap::new();
    let mut imported_values = HashMap::new();
    let mut imported_types = HashMap::new();

    for import in &module.program.imports {
        let is_std = import.path.first().map(|seg| seg == "std").unwrap_or(false);
        let target_path = import.path.join("::");
        if is_std {
            let std_module = std_metadata().module(&target_path).ok_or_else(|| {
                module_error(
                    "C020",
                    format!("unknown module `{}`", target_path),
                    &module.file,
                    import.path_span,
                    json_errors,
                )
            })?;
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
                    if module.local_values.contains(&alias_name)
                        || module.local_types.contains(&alias_name)
                        || module_aliases.contains_key(&alias_name)
                        || imported_values.contains_key(&alias_name)
                        || imported_types.contains_key(&alias_name)
                    {
                        return Err(module_error(
                            "C022",
                            format!("import name `{}` conflicts with existing name", alias_name),
                            &module.file,
                            import.span,
                            json_errors,
                        ));
                    }
                    module_aliases.insert(alias_name, target_path.clone());
                }
                ImportKind::Items { items } => {
                    for item in items {
                        let name = item.name.clone();
                        let is_value = std_module.values.contains(&name);
                        let is_type = std_module.types.contains(&name);
                        if !is_value && !is_type {
                            return Err(module_error(
                                "C021",
                                format!("module `{}` does not export item `{}`", target_path, name),
                                &module.file,
                                item.span,
                                json_errors,
                            ));
                        }
                        if is_value && is_type {
                            return Err(module_error(
                                "C021",
                                format!(
                                    "module `{}` exports `{}` as both value and type",
                                    target_path, name
                                ),
                                &module.file,
                                item.span,
                                json_errors,
                            ));
                        }
                        if module.local_values.contains(&name)
                            || module.local_types.contains(&name)
                            || module_aliases.contains_key(&name)
                            || imported_values.contains_key(&name)
                            || imported_types.contains_key(&name)
                        {
                            return Err(module_error(
                                "C022",
                                format!("import name `{}` conflicts with existing name", name),
                                &module.file,
                                item.span,
                                json_errors,
                            ));
                        }
                        let qualified = format!("{}::{}", target_path, name);
                        if is_value {
                            imported_values.insert(name, qualified);
                        } else {
                            imported_types.insert(name, qualified);
                        }
                    }
                }
            }
            continue;
        }
        let Some(target) = modules.get(&target_path) else {
            return Err(module_error(
                "C020",
                format!("unknown module `{}`", target_path),
                &module.file,
                import.path_span,
                json_errors,
            ));
        };
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
                if module.local_values.contains(&alias_name)
                    || module.local_types.contains(&alias_name)
                    || module_aliases.contains_key(&alias_name)
                    || imported_values.contains_key(&alias_name)
                    || imported_types.contains_key(&alias_name)
                {
                    return Err(module_error(
                        "C022",
                        format!("import name `{}` conflicts with existing name", alias_name),
                        &module.file,
                        import.span,
                        json_errors,
                    ));
                }
                module_aliases.insert(alias_name, target_path.clone());
            }
            ImportKind::Items { items } => {
                for item in items {
                    let name = item.name.clone();
                    let is_value = target.exports.values.contains(&name);
                    let is_type = target.exports.types.contains(&name);
                    if !is_value && !is_type {
                        return Err(module_error(
                            "C021",
                            format!("module `{}` does not export item `{}`", target_path, name),
                            &module.file,
                            item.span,
                            json_errors,
                        ));
                    }
                    if is_value && is_type {
                        return Err(module_error(
                            "C021",
                            format!(
                                "module `{}` exports `{}` as both value and type",
                                target_path, name
                            ),
                            &module.file,
                            item.span,
                            json_errors,
                        ));
                    }
                    if module.local_values.contains(&name)
                        || module.local_types.contains(&name)
                        || module_aliases.contains_key(&name)
                        || imported_values.contains_key(&name)
                        || imported_types.contains_key(&name)
                    {
                        return Err(module_error(
                            "C022",
                            format!("import name `{}` conflicts with existing name", name),
                            &module.file,
                            item.span,
                            json_errors,
                        ));
                    }
                    let qualified = qualify_name(target_prefix(target), &name);
                    if is_value {
                        imported_values.insert(name, qualified);
                    } else {
                        imported_types.insert(name, qualified);
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

fn resolve_program(module: &ModuleUnit, env: &ImportEnv) -> Program {
    let prefix = target_prefix(module);
    let local_values = module
        .local_values
        .iter()
        .map(|name| (name.clone(), qualify_name(prefix, name)))
        .collect::<HashMap<_, _>>();
    let local_types = module
        .local_types
        .iter()
        .map(|name| (name.clone(), qualify_name(prefix, name)))
        .collect::<HashMap<_, _>>();
    let ctx = ResolveCtx {
        prefix,
        local_values: &local_values,
        local_types: &local_types,
        imported_values: &env.imported_values,
        imported_types: &env.imported_types,
        module_aliases: &env.module_aliases,
    };

    let mut program = module.program.clone();
    program.module = None;
    program.imports.clear();

    for alias in &mut program.refined_aliases {
        alias.name = qualify_name(prefix, &alias.name);
        let mut params = HashSet::new();
        for name in &alias.type_params {
            params.insert(name.clone());
        }
        resolve_type(&mut alias.base, &ctx, &params);
        resolve_expr(&mut alias.predicate, &ctx, &params);
    }

    for res in &mut program.resources {
        res.name = qualify_name(prefix, &res.name);
        let params = HashSet::new();
        for field in &mut res.fields {
            resolve_type(&mut field.ty, &ctx, &params);
        }
        resolve_block(&mut res.drop_block, &ctx, &params);
    }

    for strukt in &mut program.structs {
        strukt.name = qualify_name(prefix, &strukt.name);
        let params = type_param_set(&strukt.type_params);
        for field in &mut strukt.fields {
            resolve_type(&mut field.ty, &ctx, &params);
        }
    }

    for enm in &mut program.enums {
        enm.name = qualify_name(prefix, &enm.name);
        let params = type_param_set(&enm.type_params);
        for variant in &mut enm.variants {
            for field in &mut variant.fields {
                resolve_type(field, &ctx, &params);
            }
        }
    }

    for tr in &mut program.traits {
        tr.name = qualify_name(prefix, &tr.name);
        let mut params = type_param_set(&tr.type_params);
        params.insert("Self".to_string());
        for method in &mut tr.methods {
            for param in &mut method.params {
                resolve_type(&mut param.ty, &ctx, &params);
            }
            resolve_type(&mut method.ret, &ctx, &params);
        }
    }

    for imp in &mut program.impls {
        imp.trait_name = resolve_type_name(&imp.trait_name, &ctx);
        let mut params = type_param_set(&imp.type_params);
        params.insert("Self".to_string());
        resolve_type(&mut imp.for_type, &ctx, &params);
        for bound in &mut imp.where_bounds {
            resolve_trait_bound(bound, &ctx);
        }
        for method in &mut imp.methods {
            resolve_func(method, &ctx, &params, false);
        }
    }

    for func in &mut program.funcs {
        resolve_func(func, &ctx, &type_param_set(&func.type_params), true);
    }

    program
}

fn resolve_func(func: &mut Func, ctx: &ResolveCtx<'_>, params: &HashSet<String>, rename: bool) {
    if rename {
        func.name = qualify_name(ctx.prefix, &func.name);
    }
    for param in &mut func.params {
        resolve_type(&mut param.ty, ctx, params);
    }
    resolve_type(&mut func.ret, ctx, params);
    for bound in &mut func.where_bounds {
        resolve_trait_bound(bound, ctx);
    }
    for req in &mut func.requires {
        resolve_expr(&mut req.expr, ctx, params);
    }
    resolve_expr(&mut func.body, ctx, params);
    for ens in &mut func.ensures {
        resolve_expr(&mut ens.expr, ctx, params);
    }
}

fn resolve_trait_bound(bound: &mut TraitBound, ctx: &ResolveCtx<'_>) {
    bound.trait_name = resolve_type_name(&bound.trait_name, ctx);
}

fn resolve_block(block: &mut Block, ctx: &ResolveCtx<'_>, params: &HashSet<String>) {
    for stmt in &mut block.statements {
        resolve_stmt(stmt, ctx, params);
    }
    if let Some(tail) = &mut block.tail {
        resolve_expr(tail, ctx, params);
    }
}

fn resolve_stmt(stmt: &mut Stmt, ctx: &ResolveCtx<'_>, params: &HashSet<String>) {
    match stmt {
        Stmt::Let { expr, .. } | Stmt::Expr { expr, .. } => {
            resolve_expr(expr, ctx, params);
        }
        Stmt::While {
            cond,
            invariant,
            variant,
            body,
            ..
        } => {
            resolve_expr(cond, ctx, params);
            resolve_expr(invariant, ctx, params);
            if let Some(v) = variant {
                resolve_expr(v, ctx, params);
            }
            resolve_block(body, ctx, params);
        }
    }
}

fn resolve_expr(expr: &mut Expr, ctx: &ResolveCtx<'_>, params: &HashSet<String>) {
    match expr {
        Expr::Int(_, _) | Expr::Bool(_, _) | Expr::String(_, _) | Expr::Var(_, _) => {}
        Expr::ArrayLit { elems, .. } | Expr::TupleLit { elems, .. } => {
            for elem in elems {
                resolve_expr(elem, ctx, params);
            }
        }
        Expr::StructLit { name, fields, .. } => {
            *name = resolve_type_name(name, ctx);
            for field in fields {
                resolve_expr(&mut field.expr, ctx, params);
            }
        }
        Expr::FieldAccess { base, .. } => resolve_expr(base, ctx, params),
        Expr::Block { block } => resolve_block(block, ctx, params),
        Expr::Bin { lhs, rhs, .. } => {
            resolve_expr(lhs, ctx, params);
            resolve_expr(rhs, ctx, params);
        }
        Expr::Call { callee, args, .. } => {
            *callee = resolve_callee(callee, ctx);
            for arg in args {
                resolve_expr(arg, ctx, params);
            }
        }
        Expr::Return { expr, .. } | Expr::Unary { expr, .. } | Expr::Try { expr, .. } => {
            resolve_expr(expr, ctx, params);
        }
        Expr::Match {
            scrutinee, arms, ..
        } => {
            resolve_expr(scrutinee, ctx, params);
            for arm in arms {
                resolve_arm(arm, ctx, params);
            }
        }
        Expr::If {
            cond,
            then_br,
            else_br,
            ..
        } => {
            resolve_expr(cond, ctx, params);
            resolve_expr(then_br, ctx, params);
            resolve_expr(else_br, ctx, params);
        }
        Expr::Index { base, index, .. } => {
            resolve_expr(base, ctx, params);
            resolve_expr(index, ctx, params);
        }
    }
}

fn resolve_arm(arm: &mut MatchArm, ctx: &ResolveCtx<'_>, params: &HashSet<String>) {
    resolve_pattern(&mut arm.pat, ctx, params);
    resolve_expr(&mut arm.expr, ctx, params);
}

fn resolve_pattern(pat: &mut MatchPat, ctx: &ResolveCtx<'_>, _params: &HashSet<String>) {
    match pat {
        MatchPat::EnumVariant { enum_name, .. } => {
            *enum_name = resolve_type_name(enum_name, ctx);
        }
        MatchPat::Some(_)
        | MatchPat::None
        | MatchPat::Ok(_)
        | MatchPat::Err(_)
        | MatchPat::Wildcard => {}
    }
}

fn resolve_type(ty: &mut Type, ctx: &ResolveCtx<'_>, params: &HashSet<String>) {
    match ty {
        Type::Named { name, args } => {
            if !params.contains(name) && name.as_str() != "Self" {
                *name = resolve_type_name(name, ctx);
            }
            for arg in args {
                resolve_type(arg, ctx, params);
            }
        }
        Type::Option(inner) | Type::Array(inner, _) | Type::Slice(inner) => {
            resolve_type(inner, ctx, params);
        }
        Type::Result(ok, err) | Type::Map(ok, err) => {
            resolve_type(ok, ctx, params);
            resolve_type(err, ctx, params);
        }
        Type::List(inner) | Type::Set(inner) => resolve_type(inner, ctx, params),
        Type::Tuple(elems) => {
            for elem in elems {
                resolve_type(elem, ctx, params);
            }
        }
        Type::Int
        | Type::U8
        | Type::U64
        | Type::U128
        | Type::U256
        | Type::Bool
        | Type::String
        | Type::Bytes => {}
    }
}

fn resolve_type_name(name: &str, ctx: &ResolveCtx<'_>) -> String {
    if let Some(idx) = name.find("::") {
        let first = &name[..idx];
        if let Some(module) = ctx.module_aliases.get(first) {
            let rest = &name[idx + 2..];
            return format!("{}::{}", module, rest);
        }
        return name.to_string();
    }
    if let Some(q) = ctx.local_types.get(name) {
        return q.clone();
    }
    if let Some(q) = ctx.imported_types.get(name) {
        return q.clone();
    }
    name.to_string()
}

fn resolve_callee(name: &str, ctx: &ResolveCtx<'_>) -> String {
    if let Some(idx) = name.find("::") {
        let first = &name[..idx];
        if let Some(module) = ctx.module_aliases.get(first) {
            let rest = &name[idx + 2..];
            return format!("{}::{}", module, rest);
        }
        if let Some((enum_path, variant)) = name.rsplit_once("::") {
            let resolved_enum = resolve_type_name(enum_path, ctx);
            if resolved_enum != enum_path {
                return format!("{}::{}", resolved_enum, variant);
            }
        }
        return name.to_string();
    }
    if let Some(q) = ctx.local_values.get(name) {
        return q.clone();
    }
    if let Some(q) = ctx.imported_values.get(name) {
        return q.clone();
    }
    name.to_string()
}

fn type_param_set(params: &[clg_ast::TypeParam]) -> HashSet<String> {
    let mut out = HashSet::with_capacity(params.len());
    for param in params {
        out.insert(param.name.clone());
    }
    out
}

fn target_prefix(module: &ModuleUnit) -> &str {
    if module.is_entry {
        ""
    } else {
        module.path_str.as_str()
    }
}

fn qualify_name(prefix: &str, name: &str) -> String {
    if prefix.is_empty() {
        name.to_string()
    } else {
        format!("{}::{}", prefix, name)
    }
}

fn module_error(
    code: &'static str,
    message: impl Into<String>,
    file: &Path,
    span: Span,
    json_errors: bool,
) -> anyhow::Error {
    if json_errors {
        CommandError::json(make_single_json_error(
            code, "build", message, file, span.start, span.end, None,
        ))
        .into()
    } else {
        anyhow!(message.into())
    }
}
