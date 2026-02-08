use std::collections::{HashMap, VecDeque};

use anyhow::Result;
use clg_ast::{Func, Program};

use super::{AliasMap, FnSig, TraitEnv, TypeDefs};

mod dispatch;
mod helpers;
mod mangle;
mod rewrite;
#[cfg(test)]
mod tests;

use helpers::instantiate_func;

pub(super) fn monomorphize_program<'a>(
    program: &'a Program,
    fns: &'a HashMap<&'a str, FnSig>,
    trait_env: &'a TraitEnv<'a>,
    aliases: &'a AliasMap,
    type_defs: &'a TypeDefs<'a>,
) -> Result<Program> {
    let mut base_funcs: HashMap<&'a str, &'a Func> = HashMap::with_capacity(program.funcs.len());
    for func in &program.funcs {
        base_funcs.insert(func.name.as_str(), func);
    }

    let mut mono = Monomorphizer {
        base_fns: fns,
        base_funcs,
        trait_env,
        aliases,
        type_defs,
        mono_funcs: Vec::new(),
        mono_map: HashMap::new(),
        queue: VecDeque::new(),
    };

    for func in &program.funcs {
        if func.type_params.is_empty() {
            let inst = instantiate_func(func, &HashMap::new(), func.name.clone());
            mono.register_func(inst);
        }
    }

    mono.process_queue()?;

    let mut out = program.clone();
    out.funcs = mono.mono_funcs;
    out.traits = Vec::new();
    out.impls = Vec::new();
    Ok(out)
}

struct Monomorphizer<'a> {
    base_fns: &'a HashMap<&'a str, FnSig>,
    base_funcs: HashMap<&'a str, &'a Func>,
    trait_env: &'a TraitEnv<'a>,
    aliases: &'a AliasMap,
    type_defs: &'a TypeDefs<'a>,
    mono_funcs: Vec<Func>,
    mono_map: HashMap<String, usize>,
    queue: VecDeque<usize>,
}

impl<'a> Monomorphizer<'a> {
    fn register_func(&mut self, func: Func) -> usize {
        if let Some(idx) = self.mono_map.get(&func.name).copied() {
            return idx;
        }
        let idx = self.mono_funcs.len();
        self.mono_map.insert(func.name.clone(), idx);
        self.mono_funcs.push(func);
        self.queue.push_back(idx);
        idx
    }

    fn process_queue(&mut self) -> Result<()> {
        while let Some(idx) = self.queue.pop_front() {
            let mut func = self
                .mono_funcs
                .get(idx)
                .cloned()
                .ok_or_else(|| anyhow::anyhow!("missing monomorphized function"))?;
            self.rewrite_func(&mut func)?;
            self.mono_funcs[idx] = func;
        }
        Ok(())
    }

    fn is_enum_constructor(&self, callee: &str) -> bool {
        let Some((enum_name, variant_name)) = callee.rsplit_once("::") else {
            return false;
        };
        let Some(info) = self.type_defs.enums.get(enum_name) else {
            return false;
        };
        info.variants.contains_key(variant_name)
    }
}
