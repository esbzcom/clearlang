use anyhow::Result;
use clg_ir::{Function as IrFunction, Instr as IrInstr};
use wasm_encoder::{BlockType, Function, InstructionSink, ValType};

use crate::intrinsics::runtime::emit_guard_trap;

use super::fuel::emit_fuel_tick;
use super::infer::infer_value_types;
use super::{StringPool, FUNCTION_FUEL_COST, LOOP_FUEL_COST};

mod binop;
mod memory;
mod wide;

pub(super) fn encode_ir_function(
    f: &IrFunction,
    funcs: &[IrFunction],
    strs: &StringPool<'_>,
    func_index_offset: u32,
) -> Result<Function> {
    let params_len = f.params.len() as u32;
    let max_id = compute_max_id(f, params_len);
    let value_types = infer_value_types(f, funcs, max_id)?;
    let locals = build_locals(params_len, max_id, &value_types);

    let mut fenc = Function::new(locals);
    let mut insts = fenc.instructions();
    emit_fuel_tick(&mut insts, FUNCTION_FUEL_COST);

    for ins in &f.body {
        emit_instruction(&mut insts, ins, &value_types, strs, func_index_offset)?;
    }

    insts.end();
    Ok(fenc)
}

fn compute_max_id(f: &IrFunction, params_len: u32) -> u32 {
    let mut max_id = params_len.saturating_sub(1);
    for ins in &f.body {
        match ins {
            IrInstr::IConst { dst, .. } => max_id = max_id.max(dst.0),
            IrInstr::IStringConst { dst, .. } => max_id = max_id.max(dst.0),
            IrInstr::Alloc { dst, .. } => max_id = max_id.max(dst.0),
            IrInstr::AllocDyn { dst, size, .. } => {
                max_id = max_id.max(dst.0).max(size.0);
            }
            IrInstr::Load { dst, ptr, .. } => max_id = max_id.max(dst.0).max(ptr.0),
            IrInstr::StateRead { dst, .. } => max_id = max_id.max(dst.0),
            IrInstr::Store { ptr, src, .. } => max_id = max_id.max(ptr.0).max(src.0),
            IrInstr::StateWrite { src, .. } => max_id = max_id.max(src.0),
            IrInstr::EventEmit { args, .. } => {
                for arg in args {
                    max_id = max_id.max(arg.0);
                }
            }
            IrInstr::ExternalCall { args, .. } => {
                for arg in args {
                    max_id = max_id.max(arg.0);
                }
            }
            IrInstr::IBin { dst, lhs, rhs, .. } => {
                max_id = max_id.max(dst.0).max(lhs.0).max(rhs.0);
            }
            IrInstr::Guard { cond, .. } => {
                max_id = max_id.max(cond.0);
            }
            IrInstr::ISelect {
                dst,
                cond,
                then_v,
                else_v,
            } => {
                max_id = max_id.max(dst.0).max(cond.0).max(then_v.0).max(else_v.0);
            }
            IrInstr::VariantInit {
                dst,
                tag,
                payload_lo,
                payload_hi,
            } => {
                max_id = max_id
                    .max(dst.0)
                    .max(tag.0)
                    .max(payload_lo.0)
                    .max(payload_hi.0);
            }
            IrInstr::VariantLoadTag { dst, variant, .. }
            | IrInstr::VariantLoadPayloadLo { dst, variant }
            | IrInstr::VariantLoadPayloadHi { dst, variant } => {
                max_id = max_id.max(dst.0).max(variant.0);
            }
            IrInstr::U128Init {
                dst,
                limb_lo,
                limb_hi,
            } => {
                max_id = max_id.max(dst.0).max(limb_lo.0).max(limb_hi.0);
            }
            IrInstr::U128LoadLimb { dst, value, .. } => {
                max_id = max_id.max(dst.0).max(value.0);
            }
            IrInstr::U256Init {
                dst,
                limb0,
                limb1,
                limb2,
                limb3,
            } => {
                max_id = max_id
                    .max(dst.0)
                    .max(limb0.0)
                    .max(limb1.0)
                    .max(limb2.0)
                    .max(limb3.0);
            }
            IrInstr::U256LoadLimb { dst, value, .. } => {
                max_id = max_id.max(dst.0).max(value.0);
            }
            IrInstr::MemorySize { dst } => {
                max_id = max_id.max(dst.0);
            }
            IrInstr::ReturnIf { cond, ret } => {
                max_id = max_id.max(cond.0).max(ret.0);
            }
            IrInstr::Call { dst, args, .. } => {
                if let Some(d) = dst {
                    max_id = max_id.max(d.0);
                }
                for a in args {
                    max_id = max_id.max(a.0);
                }
            }
            IrInstr::BrIf { cond, .. } | IrInstr::BrIfEqz { cond, .. } => {
                max_id = max_id.max(cond.0);
            }
            IrInstr::BlockBegin
            | IrInstr::BlockEnd
            | IrInstr::LoopBegin
            | IrInstr::LoopEnd
            | IrInstr::Br { .. } => {}
            IrInstr::Ret { val } => max_id = max_id.max(val.0),
        }
    }
    max_id
}

fn build_locals(params_len: u32, max_id: u32, value_types: &[ValType]) -> Vec<(u32, ValType)> {
    let mut locals: Vec<(u32, ValType)> = Vec::new();
    if max_id.saturating_add(1) <= params_len {
        return locals;
    }

    for idx in params_len..=max_id {
        let ty = value_types[idx as usize];
        if let Some((count, last_ty)) = locals.last_mut() {
            if *last_ty == ty {
                *count += 1;
                continue;
            }
        }
        locals.push((1, ty));
    }
    locals
}

fn emit_instruction(
    insts: &mut InstructionSink<'_>,
    ins: &IrInstr,
    value_types: &[ValType],
    strs: &StringPool<'_>,
    func_index_offset: u32,
) -> Result<()> {
    match ins {
        IrInstr::IConst { dst, n, ty } => {
            memory::emit_iconst(insts, *dst, *n, *ty);
        }
        IrInstr::IStringConst { dst, s } => {
            memory::emit_istring_const(insts, *dst, s, strs)?;
        }
        IrInstr::Alloc { dst, size, align } => {
            memory::emit_alloc(insts, *dst, *size, *align);
        }
        IrInstr::AllocDyn { dst, size, align } => {
            memory::emit_alloc_dyn(insts, *dst, *size, *align);
        }
        IrInstr::Load {
            dst,
            ptr,
            offset,
            ty,
        } => {
            memory::emit_load(insts, *dst, *ptr, *offset, *ty);
        }
        IrInstr::StateRead {
            contract, field, ..
        } => {
            anyhow::bail!(
                "Wasm backend has no contract state adapter for read `{contract}.{field}`"
            )
        }
        IrInstr::Store {
            ptr,
            src,
            offset,
            ty,
        } => {
            memory::emit_store(insts, *ptr, *src, *offset, *ty);
        }
        IrInstr::StateWrite {
            contract, field, ..
        } => {
            anyhow::bail!(
                "Wasm backend has no contract state adapter for write `{contract}.{field}`"
            )
        }
        IrInstr::EventEmit {
            contract, event, ..
        } => anyhow::bail!(
            "Wasm backend has no contract event adapter for emit `{contract}.{event}`"
        ),
        IrInstr::ExternalCall {
            contract,
            interface,
            method,
            ..
        } => anyhow::bail!(
            "Wasm backend has no external call adapter for `{contract}` calling `{interface}.{method}`"
        ),
        IrInstr::IBin {
            dst,
            op,
            lhs,
            rhs,
            ty,
        } => {
            binop::emit_binop(insts, *dst, *op, *lhs, *rhs, *ty)?;
        }
        IrInstr::Guard {
            cond,
            trap,
            span,
            detail,
        } => {
            insts.local_get(cond.0);
            insts.i32_eqz();
            insts.if_(BlockType::Empty);
            emit_guard_trap(insts, *trap, *span, detail.as_i32());
            insts.end();
        }
        IrInstr::ISelect {
            dst,
            cond,
            then_v,
            else_v,
        } => {
            let dst_ty = value_types[dst.0 as usize];
            insts.local_get(cond.0);
            insts.if_(BlockType::Result(dst_ty));
            insts.local_get(then_v.0);
            insts.else_();
            insts.local_get(else_v.0);
            insts.end();
            insts.local_set(dst.0);
        }
        IrInstr::MemorySize { dst } => {
            insts.memory_size(0);
            insts.local_set(dst.0);
        }
        IrInstr::VariantInit {
            dst,
            tag,
            payload_lo,
            payload_hi,
        } => {
            wide::emit_variant_init(insts, *dst, *tag, *payload_lo, *payload_hi);
        }
        IrInstr::VariantLoadTag { dst, variant, kind } => {
            wide::emit_variant_load_tag(insts, *dst, *variant, kind);
        }
        IrInstr::VariantLoadPayloadLo { dst, variant } => {
            wide::emit_variant_load_payload(insts, *dst, *variant, 4);
        }
        IrInstr::VariantLoadPayloadHi { dst, variant } => {
            wide::emit_variant_load_payload(insts, *dst, *variant, 8);
        }
        IrInstr::U128Init {
            dst,
            limb_lo,
            limb_hi,
        } => {
            wide::emit_u128_init(insts, *dst, *limb_lo, *limb_hi);
        }
        IrInstr::U128LoadLimb { dst, value, limb } => {
            wide::emit_u128_load_limb(insts, *dst, *value, *limb)?;
        }
        IrInstr::U256Init {
            dst,
            limb0,
            limb1,
            limb2,
            limb3,
        } => {
            wide::emit_u256_init(insts, *dst, *limb0, *limb1, *limb2, *limb3);
        }
        IrInstr::U256LoadLimb { dst, value, limb } => {
            wide::emit_u256_load_limb(insts, *dst, *value, *limb)?;
        }
        IrInstr::ReturnIf { cond, ret } => {
            insts.local_get(cond.0);
            insts.if_(BlockType::Empty);
            insts.local_get(ret.0);
            insts.return_();
            insts.end();
        }
        IrInstr::Call { dst, callee, args } => {
            for a in args {
                insts.local_get(a.0);
            }
            insts.call(*callee + func_index_offset);
            if let Some(d) = dst {
                insts.local_set(d.0);
            }
        }
        IrInstr::BlockBegin => {
            insts.block(BlockType::Empty);
        }
        IrInstr::BlockEnd => {
            insts.end();
        }
        IrInstr::LoopBegin => {
            insts.loop_(BlockType::Empty);
            emit_fuel_tick(insts, LOOP_FUEL_COST);
        }
        IrInstr::LoopEnd => {
            insts.end();
        }
        IrInstr::Br { depth } => {
            insts.br(*depth);
        }
        IrInstr::BrIf { cond, depth } => {
            insts.local_get(cond.0);
            insts.br_if(*depth);
        }
        IrInstr::BrIfEqz { cond, depth } => {
            insts.local_get(cond.0);
            insts.i32_eqz();
            insts.br_if(*depth);
        }
        IrInstr::Ret { val } => {
            insts.local_get(val.0);
        }
    }

    Ok(())
}
