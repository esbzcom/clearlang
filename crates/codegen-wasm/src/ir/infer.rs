use anyhow::Result;
use clg_ir::{BinOpIR, Function as IrFunction, Instr as IrInstr, Value};
use wasm_encoder::ValType;

use super::val_type_for_ir;

pub(super) fn infer_value_types(
    f: &IrFunction,
    funcs: &[IrFunction],
    max_id: u32,
) -> Result<Vec<ValType>> {
    let mut types: Vec<Option<ValType>> = vec![None; max_id as usize + 1];

    for (i, ty) in f.params.iter().copied().enumerate() {
        types[i] = Some(val_type_for_ir(ty));
    }

    fn set_type(types: &mut [Option<ValType>], value: Value, ty: ValType) -> Result<()> {
        let slot = types
            .get_mut(value.0 as usize)
            .ok_or_else(|| anyhow::anyhow!("value {} out of range", value.0))?;
        if let Some(existing) = *slot {
            if existing != ty {
                return Err(anyhow::anyhow!(
                    "value {} has conflicting types {:?} vs {:?}",
                    value.0,
                    existing,
                    ty
                ));
            }
        } else {
            *slot = Some(ty);
        }
        Ok(())
    }

    for ins in &f.body {
        match ins {
            IrInstr::IConst { dst, ty, .. } => {
                set_type(&mut types, *dst, val_type_for_ir(*ty))?;
            }
            IrInstr::IStringConst { dst, .. } => {
                set_type(&mut types, *dst, ValType::I32)?;
            }
            IrInstr::Alloc { dst, .. } | IrInstr::AllocDyn { dst, .. } => {
                set_type(&mut types, *dst, ValType::I32)?;
            }
            IrInstr::Load { dst, ty, .. } => {
                set_type(&mut types, *dst, val_type_for_ir(*ty))?;
            }
            IrInstr::StateRead { dst, ty, .. } => {
                set_type(&mut types, *dst, val_type_for_ir(*ty))?;
            }
            IrInstr::IBin { dst, op, ty, .. } => {
                let res_ty = match op {
                    BinOpIR::Add
                    | BinOpIR::Sub
                    | BinOpIR::Mul
                    | BinOpIR::Div
                    | BinOpIR::And
                    | BinOpIR::Or
                    | BinOpIR::Xor
                    | BinOpIR::Shl
                    | BinOpIR::Shr => val_type_for_ir(*ty),
                    BinOpIR::Lt
                    | BinOpIR::Le
                    | BinOpIR::LeU
                    | BinOpIR::Gt
                    | BinOpIR::Ge
                    | BinOpIR::Eq
                    | BinOpIR::Neq => ValType::I32,
                };
                set_type(&mut types, *dst, res_ty)?;
            }
            IrInstr::ISelect {
                dst,
                then_v,
                else_v,
                ..
            } => {
                let then_ty = types
                    .get(then_v.0 as usize)
                    .and_then(|ty| *ty)
                    .ok_or_else(|| anyhow::anyhow!("missing type for value {}", then_v.0))?;
                let else_ty = types
                    .get(else_v.0 as usize)
                    .and_then(|ty| *ty)
                    .ok_or_else(|| anyhow::anyhow!("missing type for value {}", else_v.0))?;
                if then_ty != else_ty {
                    return Err(anyhow::anyhow!(
                        "select type mismatch for value {} ({:?} vs {:?})",
                        dst.0,
                        then_ty,
                        else_ty
                    ));
                }
                set_type(&mut types, *dst, then_ty)?;
            }
            IrInstr::VariantInit { dst, .. } => {
                set_type(&mut types, *dst, ValType::I32)?;
            }
            IrInstr::U128Init { dst, .. } | IrInstr::U256Init { dst, .. } => {
                set_type(&mut types, *dst, ValType::I32)?;
            }
            IrInstr::VariantLoadTag { dst, .. }
            | IrInstr::VariantLoadPayloadLo { dst, .. }
            | IrInstr::VariantLoadPayloadHi { dst, .. } => {
                set_type(&mut types, *dst, ValType::I32)?;
            }
            IrInstr::U128LoadLimb { dst, .. } | IrInstr::U256LoadLimb { dst, .. } => {
                set_type(&mut types, *dst, ValType::I64)?;
            }
            IrInstr::MemorySize { dst } => {
                set_type(&mut types, *dst, ValType::I32)?;
            }
            IrInstr::Call { dst, callee, .. } => {
                if let Some(dst) = dst {
                    let ret_ty =
                        funcs
                            .get(*callee as usize)
                            .and_then(|f| f.ret)
                            .ok_or_else(|| {
                                anyhow::anyhow!("missing return type for call {}", callee)
                            })?;
                    set_type(&mut types, *dst, val_type_for_ir(ret_ty))?;
                }
            }
            IrInstr::Guard { .. }
            | IrInstr::ReturnIf { .. }
            | IrInstr::Store { .. }
            | IrInstr::StateWrite { .. }
            | IrInstr::EventEmit { .. }
            | IrInstr::ExternalCall { .. }
            | IrInstr::BrIf { .. }
            | IrInstr::BrIfEqz { .. }
            | IrInstr::BlockBegin
            | IrInstr::BlockEnd
            | IrInstr::LoopBegin
            | IrInstr::LoopEnd
            | IrInstr::Br { .. }
            | IrInstr::Ret { .. } => {}
        }
    }

    let mut resolved = Vec::with_capacity(types.len());
    for (idx, ty) in types.into_iter().enumerate() {
        let ty = ty.ok_or_else(|| anyhow::anyhow!("missing type for value {}", idx))?;
        resolved.push(ty);
    }
    Ok(resolved)
}
