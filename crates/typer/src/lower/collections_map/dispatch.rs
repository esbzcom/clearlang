pub(super) fn lower_map_call<'a>(
    ctx: &mut LowerCtx<'a>,
    call_expr: &'a Expr,
    callee: &str,
    args: &'a [Expr],
    expected: Option<&Type>,
) -> Result<Option<Value>> {
    match callee {
        "std::map::new" => {
            if !args.is_empty() {
                anyhow::bail!("`std::map::new` expects no arguments");
            }
            let (key_ty, val_ty) = match expected {
                Some(Type::Map(key, val)) => (*key.clone(), *val.clone()),
                _ => {
                    let call_ty = infer_expr_type(
                        call_expr,
                        &ctx.type_env,
                        &ctx.fns,
                        ctx.trait_env,
                        ctx.aliases,
                        ctx.type_defs,
                        &ctx.type_params,
                        &ctx.bounds,
                    )?;
                    let call_ty = base_type(&call_ty, ctx.aliases)?;
                    match call_ty {
                        Type::Map(key, val) => (*key, *val),
                        other => anyhow::bail!("cannot infer map type: {:?}", other),
                    }
                }
            };
            let (entry_size, entry_align, _key_offset, _val_offset) =
                map_entry_layout(&key_ty, &val_ty, ctx.aliases, ctx.std_types)?;
            let len = emit_int_const(ctx, 0);
            let cap = emit_int_const(ctx, 1);
            let buf_bytes = emit_int_const(ctx, entry_size as i64);
            let data_ptr = emit_alloc_dyn(ctx, buf_bytes, entry_align);
            let header = emit_collection_header(ctx, len, cap, data_ptr);
            Ok(Some(header))
        }
        "std::map::len" => {
            if args.len() != 1 {
                anyhow::bail!("`std::map::len` expects one argument");
            }
            let map_val = lower_expr(ctx, &args[0], None)?;
            Ok(Some(emit_collection_len(ctx, map_val)))
        }
        "std::map::is_empty" => {
            if args.len() != 1 {
                anyhow::bail!("`std::map::is_empty` expects one argument");
            }
            let map_val = lower_expr(ctx, &args[0], None)?;
            let len = emit_collection_len(ctx, map_val);
            let zero = emit_int_const(ctx, 0);
            let out = fresh(ctx);
            ctx.body.push(Instr::IBin {
                dst: out,
                op: BinOpIR::Eq,
                lhs: len,
                rhs: zero,
                ty: IrType::Int,
            });
            Ok(Some(out))
        }
        "std::map::contains" => {
            if args.len() != 2 {
                anyhow::bail!("`std::map::contains` expects two arguments");
            }
            let (key_ty, val_ty) = map_key_val_type(ctx, &args[0])?;
            let map_val = lower_expr(ctx, &args[0], None)?;
            let key_val = lower_expr(ctx, &args[1], Some(key_ty.clone()))?;
            let len = emit_collection_len(ctx, map_val);
            let data_ptr = emit_collection_data_ptr(ctx, map_val);
            let (entry_size, entry_align, key_offset, _val_offset) =
                map_entry_layout(&key_ty, &val_ty, ctx.aliases, ctx.std_types)?;
            let cap = emit_collection_cap(ctx, map_val);
            emit_collection_payload_guard(ctx, data_ptr, len, cap, entry_size, entry_align);
            let (found, _idx) = emit_find_index(
                ctx,
                data_ptr,
                len,
                entry_size,
                key_val,
                &key_ty,
                key_offset,
                ctx.aliases,
            )?;
            Ok(Some(found))
        }
        "std::map::get" => {
            if args.len() != 2 {
                anyhow::bail!("`std::map::get` expects two arguments");
            }
            let (key_ty, val_ty) = map_key_val_type(ctx, &args[0])?;
            let map_val = lower_expr(ctx, &args[0], None)?;
            let key_val = lower_expr(ctx, &args[1], Some(key_ty.clone()))?;
            let len = emit_collection_len(ctx, map_val);
            let data_ptr = emit_collection_data_ptr(ctx, map_val);
            let (entry_size, entry_align, key_offset, val_offset) =
                map_entry_layout(&key_ty, &val_ty, ctx.aliases, ctx.std_types)?;
            let cap = emit_collection_cap(ctx, map_val);
            emit_collection_payload_guard(ctx, data_ptr, len, cap, entry_size, entry_align);
            let (found, found_idx) = emit_find_index(
                ctx,
                data_ptr,
                len,
                entry_size,
                key_val,
                &key_ty,
                key_offset,
                ctx.aliases,
            )?;
            let zero = emit_int_const(ctx, 0);
            let safe_idx = fresh(ctx);
            ctx.body.push(Instr::ISelect {
                dst: safe_idx,
                cond: found,
                then_v: found_idx,
                else_v: zero,
            });
            let stride_val = emit_int_const(ctx, entry_size as i64);
            let offset = fresh(ctx);
            ctx.body.push(Instr::IBin {
                dst: offset,
                op: BinOpIR::Mul,
                lhs: safe_idx,
                rhs: stride_val,
                ty: IrType::Int,
            });
            let base_ptr = emit_ptr_add(ctx, data_ptr, offset);
            let val_ptr = if val_offset == 0 {
                base_ptr
            } else {
                let val_off_val = emit_int_const(ctx, val_offset as i64);
                emit_ptr_add(ctx, base_ptr, val_off_val)
            };
            let val_loaded = load_value_copy(ctx, &val_ty, val_ptr, 0)?;
            let zero_payload = zero_value_for_type(ctx, &val_ty)?;
            let payload = fresh(ctx);
            ctx.body.push(Instr::ISelect {
                dst: payload,
                cond: found,
                then_v: val_loaded,
                else_v: zero_payload,
            });
            let zero_hi = emit_int_const(ctx, 0);
            Ok(Some(ctx.variant_init(found, payload, zero_hi)))
        }
        "std::map::insert" => {
            if args.len() != 3 {
                anyhow::bail!("`std::map::insert` expects three arguments");
            }
            let (key_ty, val_ty) = map_key_val_type(ctx, &args[0])?;
            let map_val = lower_expr(ctx, &args[0], None)?;
            let key_val = lower_expr(ctx, &args[1], Some(key_ty.clone()))?;
            let val_val = lower_expr(ctx, &args[2], Some(val_ty.clone()))?;
            let len = emit_collection_len(ctx, map_val);
            let data_ptr = emit_collection_data_ptr(ctx, map_val);
            let (entry_size, entry_align, key_offset, val_offset) =
                map_entry_layout(&key_ty, &val_ty, ctx.aliases, ctx.std_types)?;
            let cap = emit_collection_cap(ctx, map_val);
            emit_collection_payload_guard(ctx, data_ptr, len, cap, entry_size, entry_align);
            let (found, found_idx) = emit_find_index(
                ctx,
                data_ptr,
                len,
                entry_size,
                key_val,
                &key_ty,
                key_offset,
                ctx.aliases,
            )?;
            let one = emit_int_const(ctx, 1);
            let len_plus_one = fresh(ctx);
            ctx.body.push(Instr::IBin {
                dst: len_plus_one,
                op: BinOpIR::Add,
                lhs: len,
                rhs: one,
                ty: IrType::Int,
            });
            let new_len = fresh(ctx);
            ctx.body.push(Instr::ISelect {
                dst: new_len,
                cond: found,
                then_v: len,
                else_v: len_plus_one,
            });
            let stride_val = emit_int_const(ctx, entry_size as i64);
            let new_cap = emit_cap_from_len(ctx, new_len);
            let buf_bytes = fresh(ctx);
            ctx.body.push(Instr::IBin {
                dst: buf_bytes,
                op: BinOpIR::Mul,
                lhs: new_cap,
                rhs: stride_val,
                ty: IrType::Int,
            });
            let new_data = emit_alloc_dyn(ctx, buf_bytes, entry_align);
            let copy_bytes = fresh(ctx);
            ctx.body.push(Instr::IBin {
                dst: copy_bytes,
                op: BinOpIR::Mul,
                lhs: len,
                rhs: stride_val,
                ty: IrType::Int,
            });
            emit_memcpy_bytes(ctx, data_ptr, new_data, copy_bytes)?;
            ctx.body.push(Instr::BlockBegin);
            ctx.body.push(Instr::BlockBegin);
            ctx.body.push(Instr::BrIfEqz {
                cond: found,
                depth: 0,
            });
            let found_offset = fresh(ctx);
            ctx.body.push(Instr::IBin {
                dst: found_offset,
                op: BinOpIR::Mul,
                lhs: found_idx,
                rhs: stride_val,
                ty: IrType::Int,
            });
            let base_ptr = emit_ptr_add(ctx, new_data, found_offset);
            let val_ptr = if val_offset == 0 {
                base_ptr
            } else {
                let val_off_val = emit_int_const(ctx, val_offset as i64);
                emit_ptr_add(ctx, base_ptr, val_off_val)
            };
            store_value(ctx, &val_ty, val_ptr, 0, val_val)?;
            ctx.body.push(Instr::Br { depth: 1 });
            ctx.body.push(Instr::BlockEnd);
            let offset = fresh(ctx);
            ctx.body.push(Instr::IBin {
                dst: offset,
                op: BinOpIR::Mul,
                lhs: len,
                rhs: stride_val,
                ty: IrType::Int,
            });
            let base_ptr = emit_ptr_add(ctx, new_data, offset);
            let key_ptr = if key_offset == 0 {
                base_ptr
            } else {
                let key_off_val = emit_int_const(ctx, key_offset as i64);
                emit_ptr_add(ctx, base_ptr, key_off_val)
            };
            store_value(ctx, &key_ty, key_ptr, 0, key_val)?;
            let val_ptr = if val_offset == 0 {
                base_ptr
            } else {
                let val_off_val = emit_int_const(ctx, val_offset as i64);
                emit_ptr_add(ctx, base_ptr, val_off_val)
            };
            store_value(ctx, &val_ty, val_ptr, 0, val_val)?;
            ctx.body.push(Instr::BlockEnd);
            let header = emit_collection_header(ctx, new_len, new_cap, new_data);
            Ok(Some(header))
        }
        "std::map::insert_take" => lower_map_insert_take(ctx, args),
        "std::map::remove" => lower_map_remove(ctx, args),
        "std::map::remove_take" => lower_map_remove_take(ctx, args),
        _ => Ok(None),
    }
}
