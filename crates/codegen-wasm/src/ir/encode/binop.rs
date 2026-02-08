use anyhow::Result;
use clg_ir::{BinOpIR, IrType, Value};
use wasm_encoder::InstructionSink;

pub(super) fn emit_binop(
    insts: &mut InstructionSink<'_>,
    dst: Value,
    op: BinOpIR,
    lhs: Value,
    rhs: Value,
    ty: IrType,
) -> Result<()> {
    if matches!(ty, IrType::U128 | IrType::U256) {
        return Err(anyhow::anyhow!(
            "U128/U256 operations are not supported in codegen"
        ));
    }

    insts.local_get(lhs.0);
    insts.local_get(rhs.0);
    match op {
        BinOpIR::Add => match ty {
            IrType::U64 => insts.i64_add(),
            IrType::U8 | IrType::Int | IrType::Bool | IrType::U128 | IrType::U256 => {
                insts.i32_add()
            }
        },
        BinOpIR::Sub => match ty {
            IrType::U64 => insts.i64_sub(),
            IrType::U8 | IrType::Int | IrType::Bool | IrType::U128 | IrType::U256 => {
                insts.i32_sub()
            }
        },
        BinOpIR::Mul => match ty {
            IrType::U64 => insts.i64_mul(),
            IrType::U8 | IrType::Int | IrType::Bool | IrType::U128 | IrType::U256 => {
                insts.i32_mul()
            }
        },
        BinOpIR::Div => match ty {
            IrType::U64 => insts.i64_div_u(),
            IrType::U8 | IrType::Int | IrType::Bool | IrType::U128 | IrType::U256 => {
                insts.i32_div_s()
            }
        },
        BinOpIR::Shl => match ty {
            IrType::U64 => insts.i64_shl(),
            IrType::U8 | IrType::Int | IrType::Bool | IrType::U128 | IrType::U256 => {
                insts.i32_shl()
            }
        },
        BinOpIR::Shr => match ty {
            IrType::U64 => insts.i64_shr_u(),
            IrType::U8 | IrType::Int | IrType::Bool | IrType::U128 | IrType::U256 => {
                insts.i32_shr_s()
            }
        },
        BinOpIR::Lt => match ty {
            IrType::U64 => insts.i64_lt_u(),
            IrType::U8 | IrType::Int | IrType::Bool | IrType::U128 | IrType::U256 => {
                insts.i32_lt_s()
            }
        },
        BinOpIR::Le => match ty {
            IrType::U64 => insts.i64_le_u(),
            IrType::U8 | IrType::Int | IrType::Bool | IrType::U128 | IrType::U256 => {
                insts.i32_le_s()
            }
        },
        BinOpIR::LeU => match ty {
            IrType::U64 => insts.i64_le_u(),
            IrType::U8 | IrType::Int | IrType::Bool | IrType::U128 | IrType::U256 => {
                insts.i32_le_u()
            }
        },
        BinOpIR::Gt => match ty {
            IrType::U64 => insts.i64_gt_u(),
            IrType::U8 | IrType::Int | IrType::Bool | IrType::U128 | IrType::U256 => {
                insts.i32_gt_s()
            }
        },
        BinOpIR::Ge => match ty {
            IrType::U64 => insts.i64_ge_u(),
            IrType::U8 | IrType::Int | IrType::Bool | IrType::U128 | IrType::U256 => {
                insts.i32_ge_s()
            }
        },
        BinOpIR::Eq => match ty {
            IrType::U64 => insts.i64_eq(),
            IrType::U8 | IrType::Int | IrType::Bool | IrType::U128 | IrType::U256 => insts.i32_eq(),
        },
        BinOpIR::Neq => match ty {
            IrType::U64 => insts.i64_ne(),
            IrType::U8 | IrType::Int | IrType::Bool | IrType::U128 | IrType::U256 => insts.i32_ne(),
        },
        BinOpIR::And => match ty {
            IrType::U64 => insts.i64_and(),
            IrType::U8 | IrType::Int | IrType::Bool | IrType::U128 | IrType::U256 => {
                insts.i32_and()
            }
        },
        BinOpIR::Or => match ty {
            IrType::U64 => insts.i64_or(),
            IrType::U8 | IrType::Int | IrType::Bool | IrType::U128 | IrType::U256 => insts.i32_or(),
        },
        BinOpIR::Xor => match ty {
            IrType::U64 => insts.i64_xor(),
            IrType::U8 | IrType::Int | IrType::Bool | IrType::U128 | IrType::U256 => {
                insts.i32_xor()
            }
        },
    };
    insts.local_set(dst.0);
    Ok(())
}
