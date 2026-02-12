use crate::check::{
    base_type, substitute_type, AliasMap, StdTypeInfo, StdTypeMap, TypeDefs, TypeSubst,
};
use anyhow::Result;
use clg_ast::{StructField, Type, TypeParam};
use std::collections::HashMap;

#[derive(Clone, Copy)]
pub(super) struct ArrayLayout {
    pub(super) stride: u32,
    pub(super) size: u32,
    pub(super) align: u32,
}

pub(super) struct TupleLayout {
    pub(super) offsets: Vec<u32>,
    pub(super) size: u32,
    pub(super) align: u32,
}

fn align_up(value: u32, align: u32) -> Result<u32> {
    if align <= 1 {
        return Ok(value);
    }
    let value = value as u64;
    let align = align as u64;
    let aligned = (value + (align - 1)) / align * align;
    if aligned > u32::MAX as u64 {
        anyhow::bail!("layout size exceeds u32 limits");
    }
    Ok(aligned as u32)
}

pub(super) fn std_type_info_for(
    ty: &Type,
    aliases: &AliasMap,
    std_types: &StdTypeMap,
) -> Result<Option<StdTypeInfo>> {
    let resolved = base_type(ty, aliases)?;
    if let Type::Named { name, .. } = resolved {
        if let Some(info) = std_types.get(name.as_str()) {
            return Ok(Some(info.clone()));
        }
    }
    Ok(None)
}

pub(super) fn std_type_info_for_module(
    module: &str,
    std_types: &StdTypeMap,
) -> Result<StdTypeInfo> {
    let prefix = format!("{}::", module);
    let mut found: Option<StdTypeInfo> = None;
    for (name, info) in std_types {
        if name.starts_with(&prefix) {
            if found.is_some() {
                anyhow::bail!(
                    "std module `{}` exports multiple types; constructor is ambiguous",
                    module
                );
            }
            found = Some(info.clone());
        }
    }
    found.ok_or_else(|| anyhow::anyhow!("std module `{}` has no value type metadata", module))
}

fn layout_for_type(ty: &Type, aliases: &AliasMap, std_types: &StdTypeMap) -> Result<(u32, u32)> {
    let resolved = base_type(ty, aliases)?;
    match resolved {
        Type::Int | Type::Bool => Ok((4, 4)),
        Type::U8 => Ok((1, 1)),
        Type::U64 => Ok((8, 8)),
        Type::U128 | Type::U256 => Ok((4, 4)),
        Type::String | Type::Bytes => Ok((4, 4)),
        Type::Named { name, .. } => {
            if let Some(info) = std_types.get(name.as_str()) {
                Ok((info.byte_len, info.align))
            } else {
                Ok((4, 4))
            }
        }
        Type::Option(_) | Type::Result(_, _) => Ok((4, 4)),
        Type::List(_) | Type::Set(_) | Type::Map(_, _) => Ok((4, 4)),
        Type::Array(_, _) | Type::Slice(_) | Type::Tuple(_) | Type::Fn { .. } => Ok((4, 4)),
    }
}

pub(super) fn array_layout(
    inner: &Type,
    len: u32,
    aliases: &AliasMap,
    std_types: &StdTypeMap,
) -> Result<ArrayLayout> {
    let (elem_size, elem_align) = layout_for_type(inner, aliases, std_types)?;
    let stride = align_up(elem_size, elem_align)?;
    let total = (stride as u64) * (len as u64);
    if total > u32::MAX as u64 {
        anyhow::bail!("array allocation size exceeds u32 limits");
    }
    Ok(ArrayLayout {
        stride,
        size: total as u32,
        align: elem_align,
    })
}

pub(super) fn tuple_layout(
    elems: &[Type],
    aliases: &AliasMap,
    std_types: &StdTypeMap,
) -> Result<TupleLayout> {
    let mut offsets = Vec::with_capacity(elems.len());
    let mut offset: u32 = 0;
    let mut max_align: u32 = 1;
    for elem in elems {
        let (size, align) = layout_for_type(elem, aliases, std_types)?;
        max_align = max_align.max(align);
        offset = align_up(offset, align)?;
        offsets.push(offset);
        offset = offset
            .checked_add(size)
            .ok_or_else(|| anyhow::anyhow!("tuple size overflow"))?;
    }
    let size = align_up(offset, max_align)?;
    Ok(TupleLayout {
        offsets,
        size,
        align: max_align,
    })
}

pub(super) fn collection_layout(
    elem_ty: &Type,
    aliases: &AliasMap,
    std_types: &StdTypeMap,
) -> Result<(u32, u32, u32)> {
    let (size, align) = layout_for_type(elem_ty, aliases, std_types)?;
    let stride = align_up(size, align)?;
    Ok((size, align, stride))
}

pub(super) fn map_entry_layout(
    key_ty: &Type,
    val_ty: &Type,
    aliases: &AliasMap,
    std_types: &StdTypeMap,
) -> Result<(u32, u32, u32, u32)> {
    let tuple = tuple_layout(&[key_ty.clone(), val_ty.clone()], aliases, std_types)?;
    let key_offset = *tuple
        .offsets
        .get(0)
        .ok_or_else(|| anyhow::anyhow!("map key offset missing"))?;
    let val_offset = *tuple
        .offsets
        .get(1)
        .ok_or_else(|| anyhow::anyhow!("map value offset missing"))?;
    Ok((tuple.size, tuple.align, key_offset, val_offset))
}

pub(super) fn build_type_param_subst(params: &[TypeParam], args: &[Type]) -> Result<TypeSubst> {
    if params.len() != args.len() {
        anyhow::bail!(
            "type argument count mismatch: expected {}, found {}",
            params.len(),
            args.len()
        );
    }
    let mut subst: TypeSubst = HashMap::with_capacity(params.len());
    for (param, arg) in params.iter().zip(args.iter()) {
        subst.insert(param.name.clone(), arg.clone());
    }
    Ok(subst)
}

pub(super) fn struct_layout<'a>(
    type_defs: &'a TypeDefs,
    aliases: &AliasMap,
    std_types: &StdTypeMap,
    name: &str,
    args: &[Type],
) -> Result<(Vec<StructField>, TupleLayout)> {
    let info = type_defs
        .structs
        .get(name)
        .ok_or_else(|| anyhow::anyhow!("unknown struct `{}`", name))?;
    let subst = build_type_param_subst(&info.decl.type_params, args)?;
    let mut fields = Vec::with_capacity(info.decl.fields.len());
    let mut field_tys = Vec::with_capacity(info.decl.fields.len());
    for field in &info.decl.fields {
        let mut cloned = field.clone();
        cloned.ty = substitute_type(&cloned.ty, &subst);
        fields.push(cloned);
        field_tys.push(base_type(&fields.last().unwrap().ty, aliases)?);
    }
    let layout = tuple_layout(&field_tys, aliases, std_types)?;
    Ok((fields, layout))
}

pub(super) fn enum_variant_info<'a>(
    type_defs: &'a TypeDefs,
    enum_name: &str,
    args: &[Type],
    variant_name: &str,
) -> Result<(usize, Vec<Type>, usize)> {
    let info = type_defs
        .enums
        .get(enum_name)
        .ok_or_else(|| anyhow::anyhow!("unknown enum `{}`", enum_name))?;
    let subst = build_type_param_subst(&info.decl.type_params, args)?;
    let mut idx = None;
    for (i, variant) in info.decl.variants.iter().enumerate() {
        if variant.name == variant_name {
            idx = Some(i);
            break;
        }
    }
    let index = idx
        .ok_or_else(|| anyhow::anyhow!("unknown enum variant `{}::{}`", enum_name, variant_name))?;
    let variant = &info.decl.variants[index];
    let mut fields = Vec::with_capacity(variant.fields.len());
    for ty in &variant.fields {
        fields.push(substitute_type(ty, &subst));
    }
    Ok((index, fields, info.decl.variants.len()))
}
