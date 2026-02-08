use clg_ir::TrapCode;
use wasm_encoder::{BlockType, InstructionSink};

use crate::intrinsics::runtime::{emit_runtime_trap, TrapOperand};

use super::FUEL_GLOBAL;

pub(super) fn emit_fuel_tick(insts: &mut InstructionSink<'_>, cost: i32) {
    insts.global_get(FUEL_GLOBAL);
    insts.i32_const(cost);
    insts.i32_sub();
    insts.global_set(FUEL_GLOBAL);

    insts.global_get(FUEL_GLOBAL);
    insts.i32_const(0);
    insts.i32_le_s();
    insts.if_(BlockType::Empty);
    emit_runtime_trap(
        insts,
        TrapCode::LimitsExceeded,
        TrapOperand::zero(),
        TrapOperand::zero(),
        0,
    );
    insts.end();
}
