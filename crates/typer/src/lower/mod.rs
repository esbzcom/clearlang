use crate::check::{
    base_type, AliasMap, BoundsMap, FnSig as CheckFnSig, LocalBinding, StdTypeMap, TraitEnv,
    TypeDefs,
};
use anyhow::Result;
use clg_ast::{Expr, Func, ParamKind, Type};
use clg_ir::{
    Function as IrFunction, GuardKind, Instr, IrType, TrapCode, Value, VariantKind, VariantParts,
};
use std::collections::{HashMap, HashSet};
use std::fmt::Write as _;

mod array;
mod atoms;
mod binops;
mod block;
mod calls;
mod closures;
mod codec;
mod collection_types;
mod collections;
mod collections_helpers;
mod collections_list;
mod collections_map;
mod collections_set;
mod collections_slice;
mod contract;
mod control;
mod crypto;
mod emit;
mod eq;
mod eq_primitives;
mod host;
mod index;
mod intrinsics;
mod layout;
mod literals;
mod r#match;
mod structs;
mod u64_ops;

use array::emit_array_len_guard;
use atoms::{lower_return_expr, lower_unary_expr, lower_var_expr};
use binops::lower_bin_expr;
use block::lower_block_expr;
use calls::lower_call_expr;
use closures::lower_lambda_expr;
use control::{lower_if_expr, lower_try_expr};
use emit::{
    emit_alloc, emit_int_const, emit_memcpy_bytes, emit_ptr_add, emit_zero_for_mem_ty, fresh,
};
pub(super) use emit::{
    emit_alloc_dyn, emit_bool_const, emit_load_i32, emit_store_i32, mem_layout_for_ir,
};
use index::lower_index_expr;
use layout::std_type_info_for;
use literals::{lower_array_lit, lower_bool_lit, lower_int_lit, lower_string_lit, lower_tuple_lit};
use r#match::lower_match_expr;
use structs::{lower_field_access, lower_struct_lit};
pub(crate) use u64_ops::{
    emit_u64_bin, emit_u64_const, emit_u64_overflow_flag, emit_u64_overflow_guard,
};

type FnSig = CheckFnSig;

const COLLECTION_HEADER_SIZE: u32 = 16;
const COLLECTION_HEADER_ALIGN: u32 = 4;
const COLLECTION_LEN_OFFSET: u32 = 0;
const COLLECTION_CAP_OFFSET: u32 = 4;
const COLLECTION_FLAGS_OFFSET: u32 = 8;
const COLLECTION_DATA_OFFSET: u32 = 12;
pub(super) const CLOSURE_RECORD_SIZE: u32 = 8;
pub(super) const CLOSURE_RECORD_ALIGN: u32 = 4;
pub(super) const CLOSURE_CODE_ID_OFFSET: u32 = 0;
pub(super) const CLOSURE_ENV_PTR_OFFSET: u32 = 4;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct DispatcherSignature {
    pub params: Vec<Type>,
    pub ret: Type,
}

#[derive(Clone, Debug)]
pub(crate) struct LambdaDispatchCase {
    pub code_id: u32,
    pub function_name: String,
    pub signature: DispatcherSignature,
}

#[derive(Clone, Debug)]
pub(crate) struct DispatcherCallPatch {
    pub function_name: String,
    pub instr_index: usize,
    pub signature: DispatcherSignature,
}

pub(crate) struct LoweredFuncArtifacts {
    pub function: IrFunction,
    pub generated_functions: Vec<IrFunction>,
    pub lambda_cases: Vec<LambdaDispatchCase>,
    pub dispatcher_patches: Vec<DispatcherCallPatch>,
}

fn signature_sort_key(sig: &DispatcherSignature) -> String {
    let params = sig
        .params
        .iter()
        .cloned()
        .map(show_type_for_dispatch)
        .collect::<Vec<_>>()
        .join(", ");
    format!(
        "function({params}) -> {}",
        show_type_for_dispatch(sig.ret.clone())
    )
}

fn show_type_for_dispatch(ty: Type) -> String {
    match ty {
        Type::Int => "Int".to_string(),
        Type::U8 => "U8".to_string(),
        Type::U64 => "U64".to_string(),
        Type::U128 => "U128".to_string(),
        Type::U256 => "U256".to_string(),
        Type::Bool => "Bool".to_string(),
        Type::String => "String".to_string(),
        Type::Bytes => "Bytes".to_string(),
        Type::Named { name, args } => {
            if args.is_empty() {
                name
            } else {
                let rendered = args
                    .into_iter()
                    .map(show_type_for_dispatch)
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("{}<{}>", name, rendered)
            }
        }
        Type::Option(inner) => format!("Option<{}>", show_type_for_dispatch(*inner)),
        Type::Result(ok, err) => format!(
            "Result<{}, {}>",
            show_type_for_dispatch(*ok),
            show_type_for_dispatch(*err)
        ),
        Type::List(inner) => format!("List<{}>", show_type_for_dispatch(*inner)),
        Type::Set(inner) => format!("Set<{}>", show_type_for_dispatch(*inner)),
        Type::Map(key, val) => format!(
            "Map<{}, {}>",
            show_type_for_dispatch(*key),
            show_type_for_dispatch(*val)
        ),
        Type::Array(inner, Some(len)) => format!("[{}; {}]", show_type_for_dispatch(*inner), len),
        Type::Array(inner, None) => format!("Array<{}>", show_type_for_dispatch(*inner)),
        Type::Slice(inner) => format!("Slice<{}>", show_type_for_dispatch(*inner)),
        Type::Tuple(elements) => {
            let rendered = elements
                .into_iter()
                .map(show_type_for_dispatch)
                .collect::<Vec<_>>()
                .join(", ");
            format!("({})", rendered)
        }
        Type::Fn { params, ret } => {
            let rendered = params
                .into_iter()
                .map(show_type_for_dispatch)
                .collect::<Vec<_>>()
                .join(", ");
            format!("function({}) -> {}", rendered, show_type_for_dispatch(*ret))
        }
    }
}

pub(crate) fn dispatcher_name(sig: &DispatcherSignature) -> String {
    let mut out = String::from("__clg_dispatch_");
    // Encode the canonical signature bytes as hex to avoid lossy name collisions.
    for byte in signature_sort_key(sig).bytes() {
        let _ = write!(&mut out, "{byte:02x}");
    }
    out
}

pub(crate) fn build_dispatcher_function(
    sig: &DispatcherSignature,
    cases: &[LambdaDispatchCase],
    lambda_fn_indices: &HashMap<String, u32>,
) -> Result<IrFunction> {
    let mut ordered_cases = cases.to_vec();
    ordered_cases.sort_by_key(|case| case.code_id);

    let mut body: Vec<Instr> = Vec::new();
    let mut next: u32 = (2 + sig.params.len()) as u32;
    let code_id_param = Value(0);
    let env_ptr_param = Value(1);
    let user_params = (0..sig.params.len())
        .map(|idx| Value(2 + idx as u32))
        .collect::<Vec<_>>();

    for case in &ordered_cases {
        let Some(lambda_idx) = lambda_fn_indices.get(case.function_name.as_str()).copied() else {
            anyhow::bail!(
                "missing lambda function `{}` while building dispatcher",
                case.function_name
            );
        };
        let case_id = Value(next);
        next += 1;
        body.push(Instr::IConst {
            dst: case_id,
            ty: IrType::Int,
            n: case.code_id as i64,
        });
        let cond = Value(next);
        next += 1;
        body.push(Instr::IBin {
            dst: cond,
            op: clg_ir::BinOpIR::Eq,
            lhs: code_id_param,
            rhs: case_id,
            ty: IrType::Int,
        });
        body.push(Instr::BlockBegin);
        body.push(Instr::BrIfEqz { cond, depth: 0 });
        let mut call_args: Vec<Value> = Vec::with_capacity(1 + user_params.len());
        call_args.push(env_ptr_param);
        call_args.extend(user_params.iter().copied());
        let call_dst = Value(next);
        next += 1;
        body.push(Instr::Call {
            dst: Some(call_dst),
            callee: lambda_idx,
            args: call_args,
        });
        body.push(Instr::ReturnIf {
            cond,
            ret: call_dst,
        });
        body.push(Instr::BlockEnd);
    }

    let trap_cond = Value(next);
    next += 1;
    body.push(Instr::IConst {
        dst: trap_cond,
        ty: IrType::Bool,
        n: 0,
    });
    body.push(Instr::Guard {
        cond: trap_cond,
        trap: TrapCode::ClosureDispatchUnknownCode,
        span: None,
        detail: GuardKind::Require,
    });
    let ret_zero = Value(next);
    body.push(Instr::IConst {
        dst: ret_zero,
        ty: ir_ty(sig.ret.clone()),
        n: 0,
    });
    body.push(Instr::Ret { val: ret_zero });

    Ok(IrFunction {
        name: dispatcher_name(sig),
        params: {
            let mut params = Vec::with_capacity(2 + sig.params.len());
            params.push(IrType::Int); // code_id
            params.push(IrType::Int); // env_ptr
            params.extend(sig.params.iter().cloned().map(ir_ty));
            params
        },
        ret: Some(ir_ty(sig.ret.clone())),
        body,
    })
}

fn ir_ty(t: Type) -> IrType {
    match t {
        Type::Int => IrType::Int,
        Type::U8 => IrType::Int,
        Type::U64 => IrType::U64,
        Type::U128 => IrType::U128,
        Type::U256 => IrType::U256,
        Type::Bool => IrType::Bool,
        Type::String => IrType::Int, // Runtime value is a pointer-like handle carried in i32.
        Type::Bytes => IrType::Int,
        Type::Named { .. } => IrType::Int,
        Type::Option(_) => IrType::Int,
        Type::Result(_, _) => IrType::Int,
        Type::List(_)
        | Type::Set(_)
        | Type::Map(_, _)
        | Type::Array(_, _)
        | Type::Slice(_)
        | Type::Tuple(_)
        | Type::Fn { .. } => IrType::Int,
    }
}

fn mem_ir_type(ty: &Type, aliases: &AliasMap) -> Result<IrType> {
    let resolved = base_type(ty, aliases)?;
    let ir = match resolved {
        Type::U8 => IrType::U8,
        Type::U64 => IrType::U64,
        Type::Bool => IrType::Bool,
        _ => IrType::Int,
    };
    Ok(ir)
}

fn emit_ptr_add_const(ctx: &mut LowerCtx<'_>, ptr: Value, offset: u32) -> Value {
    if offset == 0 {
        return ptr;
    }
    let off = emit_int_const(ctx, offset as i64);
    emit_ptr_add(ctx, ptr, off)
}

pub(super) fn load_value_borrow(
    ctx: &mut LowerCtx<'_>,
    ty: &Type,
    ptr: Value,
    offset: u32,
) -> Result<Value> {
    if std_type_info_for(ty, ctx.aliases, ctx.std_types)?.is_some() {
        return Ok(emit_ptr_add_const(ctx, ptr, offset));
    }
    let mem_ty = mem_ir_type(ty, ctx.aliases)?;
    let dst = fresh(ctx);
    ctx.body.push(Instr::Load {
        dst,
        ptr,
        offset,
        ty: mem_ty,
    });
    Ok(dst)
}

fn load_value_copy(ctx: &mut LowerCtx<'_>, ty: &Type, ptr: Value, offset: u32) -> Result<Value> {
    if let Some(info) = std_type_info_for(ty, ctx.aliases, ctx.std_types)? {
        let src_ptr = emit_ptr_add_const(ctx, ptr, offset);
        let dst = emit_alloc(ctx, info.byte_len, info.align);
        let len_val = emit_int_const(ctx, info.byte_len as i64);
        emit_memcpy_bytes(ctx, src_ptr, dst, len_val)?;
        return Ok(dst);
    }
    load_value_borrow(ctx, ty, ptr, offset)
}

pub(super) fn store_value(
    ctx: &mut LowerCtx<'_>,
    ty: &Type,
    ptr: Value,
    offset: u32,
    value: Value,
) -> Result<()> {
    if let Some(info) = std_type_info_for(ty, ctx.aliases, ctx.std_types)? {
        let dst_ptr = emit_ptr_add_const(ctx, ptr, offset);
        let len_val = emit_int_const(ctx, info.byte_len as i64);
        emit_memcpy_bytes(ctx, value, dst_ptr, len_val)?;
        return Ok(());
    }
    let mem_ty = mem_ir_type(ty, ctx.aliases)?;
    ctx.body.push(Instr::Store {
        ptr,
        src: value,
        offset,
        ty: mem_ty,
    });
    Ok(())
}

fn zero_value_for_type(ctx: &mut LowerCtx<'_>, ty: &Type) -> Result<Value> {
    if std_type_info_for(ty, ctx.aliases, ctx.std_types)?.is_some() {
        return Ok(emit_int_const(ctx, 0));
    }
    let mem_ty = mem_ir_type(ty, ctx.aliases)?;
    Ok(emit_zero_for_mem_ty(ctx, mem_ty))
}

pub(super) fn pack_variant_payload(
    ctx: &mut LowerCtx<'_>,
    ty: &Type,
    value: Value,
) -> Result<Value> {
    let resolved = base_type(ty, ctx.aliases)?;
    match resolved {
        Type::U64 => {
            let ptr = emit_alloc(ctx, 8, 8);
            ctx.body.push(Instr::Store {
                ptr,
                src: value,
                offset: 0,
                ty: IrType::U64,
            });
            Ok(ptr)
        }
        _ => Ok(value),
    }
}

pub(super) fn unpack_variant_payload(
    ctx: &mut LowerCtx<'_>,
    ty: &Type,
    payload_lo: Value,
) -> Result<Value> {
    let resolved = base_type(ty, ctx.aliases)?;
    match resolved {
        Type::U64 => {
            let dst = fresh(ctx);
            ctx.body.push(Instr::Load {
                dst,
                ptr: payload_lo,
                offset: 0,
                ty: IrType::U64,
            });
            Ok(dst)
        }
        _ => Ok(payload_lo),
    }
}

pub(crate) struct LowerCtx<'a> {
    pub next: u32,
    pub env: HashMap<&'a str, Value>,
    pub type_env: HashMap<&'a str, LocalBinding>,
    pub fns: HashMap<&'a str, FnSig>,      // for call return types
    pub fn_indices: HashMap<&'a str, u32>, // for resolving callee indices (user + intrinsics)
    pub aliases: &'a AliasMap,
    pub trait_env: &'a TraitEnv<'a>,
    pub type_defs: &'a TypeDefs<'a>,
    pub std_types: &'a StdTypeMap,
    pub type_params: HashSet<String>,
    pub bounds: BoundsMap,
    pub body: Vec<Instr>,
    pub ret_ty: Type,
    pub next_closure_code_id: u32,
    pub function_name: String,
    pub generated_functions: Vec<IrFunction>,
    pub lambda_cases: Vec<LambdaDispatchCase>,
    pub dispatcher_patches: Vec<DispatcherCallPatch>,
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn lower_func<'a>(
    f: &'a Func,
    fns: &HashMap<&'a str, FnSig>,
    fn_indices: &HashMap<&'a str, u32>,
    aliases: &'a AliasMap,
    trait_env: &'a TraitEnv<'a>,
    type_defs: &'a TypeDefs<'a>,
    std_types: &'a StdTypeMap,
    next_closure_code_id: &mut u32,
) -> Result<LoweredFuncArtifacts> {
    let mut env: HashMap<&str, Value> = HashMap::new();
    let mut type_env: HashMap<&str, LocalBinding> = HashMap::new();
    type_env.insert(
        "$return",
        LocalBinding {
            ty: f.ret.clone(),
            kind: ParamKind::Borrow,
        },
    );
    for (i, p) in f.params.iter().enumerate() {
        env.insert(p.name.as_str(), Value(i as u32));
        type_env.insert(
            p.name.as_str(),
            LocalBinding {
                ty: p.ty.clone(),
                kind: p.kind,
            },
        );
    }
    let mut ctx = LowerCtx {
        next: f.params.len() as u32,
        env,
        type_env,
        fns: fns.clone(),
        fn_indices: fn_indices.clone(),
        aliases,
        trait_env,
        type_defs,
        std_types,
        type_params: f.type_params.iter().map(|p| p.name.clone()).collect(),
        bounds: {
            let mut map: BoundsMap = HashMap::new();
            for bound in &f.where_bounds {
                map.entry(bound.param.clone())
                    .or_default()
                    .insert(bound.trait_name.clone());
            }
            map
        },
        body: Vec::new(),
        ret_ty: f.ret.clone(),
        next_closure_code_id: *next_closure_code_id,
        function_name: f.name.clone(),
        generated_functions: Vec::new(),
        lambda_cases: Vec::new(),
        dispatcher_patches: Vec::new(),
    };

    for (i, p) in f.params.iter().enumerate() {
        if let Type::Array(_, Some(len)) = base_type(&p.ty, aliases)? {
            emit_array_len_guard(&mut ctx, Value(i as u32), len);
        }
    }

    for req in &f.requires {
        let cond = lower_expr(&mut ctx, &req.expr, None)?;
        ctx.body.push(Instr::Guard {
            cond,
            trap: TrapCode::ContractViolation,
            span: Some((req.span.start as u32, req.span.end as u32)),
            detail: GuardKind::Require,
        });
    }

    let ret_val = lower_expr(&mut ctx, &f.body, Some(f.ret.clone()))?;

    if let Type::Array(_, Some(len)) = base_type(&f.ret, aliases)? {
        emit_array_len_guard(&mut ctx, ret_val, len);
    }

    if !f.ensures.is_empty() {
        ctx.env.insert("result", ret_val);
        for ens in &f.ensures {
            let cond = lower_expr(&mut ctx, &ens.expr, None)?;
            ctx.body.push(Instr::Guard {
                cond,
                trap: TrapCode::ContractViolation,
                span: Some((ens.span.start as u32, ens.span.end as u32)),
                detail: GuardKind::Ensure,
            });
        }
        ctx.env.remove("result");
    }

    ctx.body.push(Instr::Ret { val: ret_val });
    *next_closure_code_id = ctx.next_closure_code_id;

    Ok(LoweredFuncArtifacts {
        function: IrFunction {
            name: f.name.clone(),
            params: f.params.iter().map(|p| ir_ty(p.ty.clone())).collect(),
            ret: Some(ir_ty(f.ret.clone())),
            body: ctx.body,
        },
        generated_functions: ctx.generated_functions,
        lambda_cases: ctx.lambda_cases,
        dispatcher_patches: ctx.dispatcher_patches,
    })
}

fn lower_expr<'a>(ctx: &mut LowerCtx<'a>, e: &'a Expr, expected: Option<Type>) -> Result<Value> {
    match e {
        Expr::Int(n, _) => lower_int_lit(ctx, *n, expected),
        Expr::Block { block } => lower_block_expr(ctx, block, expected),
        Expr::Return { expr, .. } => lower_return_expr(ctx, expr, expected),
        Expr::Bool(b, _) => lower_bool_lit(ctx, *b),
        Expr::String(s, _) => lower_string_lit(ctx, s),
        Expr::ArrayLit { elems, .. } => lower_array_lit(ctx, elems),
        Expr::TupleLit { elems, .. } => lower_tuple_lit(ctx, elems),
        Expr::StructLit {
            name: _, fields, ..
        } => lower_struct_lit(ctx, e, fields),
        Expr::FieldAccess { base, field, .. } => lower_field_access(ctx, base, field.as_str()),
        Expr::Unary { .. } => lower_unary_expr(),
        Expr::Match {
            scrutinee, arms, ..
        } => lower_match_expr(ctx, scrutinee, arms, expected),
        Expr::Try { expr, .. } => lower_try_expr(ctx, expr),
        Expr::If {
            cond,
            then_br,
            else_br,
            ..
        } => lower_if_expr(ctx, cond, then_br, else_br, expected),
        Expr::Var(name, _) => lower_var_expr(ctx, name.as_str()),
        Expr::Index { base, index, span } => lower_index_expr(ctx, base, index, span),
        Expr::Bin { op, lhs, rhs, .. } => lower_bin_expr(ctx, e, op, lhs, rhs, expected),
        Expr::Call { callee, args, .. } => {
            lower_call_expr(ctx, e, callee.as_str(), args, expected.as_ref())
        }
        Expr::Lambda { .. } => lower_lambda_expr(ctx, e, expected.as_ref()),
    }
}

impl<'a> LowerCtx<'a> {
    fn next_lambda_code_id(&mut self) -> u32 {
        let out = self.next_closure_code_id;
        self.next_closure_code_id += 1;
        out
    }

    fn variant_init(&mut self, tag: Value, payload_lo: Value, payload_hi: Value) -> Value {
        let dst = fresh(self);
        self.body.push(Instr::VariantInit {
            dst,
            tag,
            payload_lo,
            payload_hi,
        });
        dst
    }

    #[allow(dead_code)]
    fn variant_destructure(&mut self, variant: Value, kind: VariantKind) -> VariantParts {
        let tag = fresh(self);
        self.body.push(Instr::VariantLoadTag {
            dst: tag,
            variant,
            kind,
        });
        let payload_lo = fresh(self);
        self.body.push(Instr::VariantLoadPayloadLo {
            dst: payload_lo,
            variant,
        });
        let payload_hi = fresh(self);
        self.body.push(Instr::VariantLoadPayloadHi {
            dst: payload_hi,
            variant,
        });
        VariantParts {
            tag,
            payload_lo,
            payload_hi,
        }
    }
}
