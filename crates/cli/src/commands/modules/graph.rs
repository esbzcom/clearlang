use std::collections::{HashMap, HashSet, VecDeque};
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

use anyhow::{anyhow, Context, Result};
use clg_ast::{Program, Span};
use clg_parser::{parse as parse_src, parse_errors as parse_src_errs};

use crate::commands::helpers::{make_parse_json_error, CommandError};

use super::error::module_error;
use super::imports::build_import_env;
use super::resolve::resolve_program;
use super::{Exports, ModuleUnit};

pub(super) fn load_program(entry: &Path, json_errors: bool) -> Result<Program> {
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

        if !is_entry && path.first().map(|s| s.as_str()) == Some("std") {
            return Err(module_error(
                "C026",
                format!("module path `{}` is reserved for std", path_str),
                &file,
                Span { start: 0, end: 0 },
                json_errors,
            ));
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
            if import.path.first().map(|s| s.as_str()) == Some("std") {
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

    detect_cycles(&modules, json_errors)?;

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

    let mut modules_by_name: HashMap<String, &ModuleUnit> = HashMap::with_capacity(modules.len());
    for module in &modules {
        modules_by_name.insert(module.path_str.clone(), module);
    }

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
            if import.path.first().map(|s| s.as_str()) == Some("std") {
                continue;
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
    let mut values = HashSet::with_capacity(program.funcs.len());
    for func in &program.funcs {
        values.insert(func.name.clone());
    }

    let mut types = HashSet::with_capacity(
        program.refined_aliases.len()
            + program.resources.len()
            + program.structs.len()
            + program.enums.len()
            + program.traits.len(),
    );
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
