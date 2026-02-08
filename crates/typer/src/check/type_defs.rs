use anyhow::Result;
use clg_ast::{EnumVariant, Program, StructField};
use std::collections::{HashMap, HashSet};

use super::{EnumInfo, StructInfo, TypeDefs};
use crate::errors::TyperError;

pub(super) fn build_type_defs(program: &Program) -> Result<TypeDefs<'_>> {
    let mut resources: HashSet<&str> = HashSet::with_capacity(program.resources.len());
    for res in &program.resources {
        resources.insert(res.name.as_str());
    }

    let alias_names: HashSet<&str> = program
        .refined_aliases
        .iter()
        .map(|alias| alias.name.as_str())
        .collect();

    let mut structs: HashMap<&str, StructInfo<'_>> = HashMap::with_capacity(program.structs.len());
    let mut enums: HashMap<&str, EnumInfo<'_>> = HashMap::with_capacity(program.enums.len());

    for s in &program.structs {
        let name = s.name.as_str();
        if resources.contains(name) {
            return Err(TyperError::type_conflicts_with_resource(name, s.name_span).into());
        }
        if alias_names.contains(name) || structs.contains_key(name) || enums.contains_key(name) {
            return Err(TyperError::duplicate_type(name, s.name_span).into());
        }
        let mut fields: HashMap<&str, &StructField> = HashMap::with_capacity(s.fields.len());
        for field in &s.fields {
            if fields.insert(field.name.as_str(), field).is_some() {
                return Err(
                    TyperError::duplicate_struct_field(field.name.as_str(), field.span).into(),
                );
            }
        }
        structs.insert(name, StructInfo { decl: s, fields });
    }

    for e in &program.enums {
        let name = e.name.as_str();
        if resources.contains(name) {
            return Err(TyperError::type_conflicts_with_resource(name, e.name_span).into());
        }
        if alias_names.contains(name) || structs.contains_key(name) || enums.contains_key(name) {
            return Err(TyperError::duplicate_type(name, e.name_span).into());
        }
        let mut variants: HashMap<&str, &EnumVariant> = HashMap::with_capacity(e.variants.len());
        for variant in &e.variants {
            if variants.insert(variant.name.as_str(), variant).is_some() {
                return Err(TyperError::duplicate_enum_variant(
                    variant.name.as_str(),
                    variant.span,
                )
                .into());
            }
        }
        enums.insert(name, EnumInfo { decl: e, variants });
    }

    Ok(TypeDefs {
        resources,
        structs,
        enums,
    })
}
