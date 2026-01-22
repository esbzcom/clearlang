use clg_ir::{Function as IrFunction, TrapCode};
use wasm_encoder::{Function, InstructionSink};

use crate::ir::{ERROR_CODE_GLOBAL, ERROR_DETAIL_GLOBAL, ERROR_END_GLOBAL, ERROR_START_GLOBAL};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrapOperand {
    Local(u32),
    Immediate(i32),
}

impl TrapOperand {
    pub const fn local(idx: u32) -> Self {
        Self::Local(idx)
    }

    pub const fn imm(value: i32) -> Self {
        Self::Immediate(value)
    }

    pub const fn zero() -> Self {
        Self::Immediate(0)
    }

    fn emit(self, insts: &mut InstructionSink<'_>) {
        match self {
            TrapOperand::Local(idx) => {
                insts.local_get(idx);
            }
            TrapOperand::Immediate(value) => {
                insts.i32_const(value);
            }
        }
    }
}

pub fn emit_runtime_trap(
    insts: &mut InstructionSink<'_>,
    code: TrapCode,
    start: TrapOperand,
    end: TrapOperand,
    detail: i32,
) {
    insts.i32_const(code.as_i32());
    insts.global_set(ERROR_CODE_GLOBAL);
    start.emit(insts);
    insts.global_set(ERROR_START_GLOBAL);
    end.emit(insts);
    insts.global_set(ERROR_END_GLOBAL);
    insts.i32_const(detail);
    insts.global_set(ERROR_DETAIL_GLOBAL);
    insts.unreachable();
}

pub fn emit_guard_trap(
    insts: &mut InstructionSink<'_>,
    code: TrapCode,
    span: Option<(u32, u32)>,
    detail: i32,
) {
    let (start, end) = span.unwrap_or((0, 0));
    emit_runtime_trap(
        insts,
        code,
        TrapOperand::imm(start as i32),
        TrapOperand::imm(end as i32),
        detail,
    );
}

pub fn encode_intrinsic_identity(_f: &IrFunction) -> anyhow::Result<Function> {
    let mut fenc = Function::new(Vec::new());
    let mut insts = fenc.instructions();
    insts.local_get(0);
    insts.end();
    Ok(fenc)
}
