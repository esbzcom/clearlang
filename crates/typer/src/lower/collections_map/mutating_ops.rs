fn lower_map_insert_take<'a>(ctx: &mut LowerCtx<'a>, args: &'a [Expr]) -> Result<Option<Value>> {
    if args.len() != 3 {
        anyhow::bail!("`std::map::insert_take` expects three arguments");
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
    let zero = emit_int_const(ctx, 0);
    let safe_idx = fresh(ctx);
    ctx.body.push(Instr::ISelect {
        dst: safe_idx,
        cond: found,
        then_v: found_idx,
        else_v: zero,
    });
    let stride_val = emit_int_const(ctx, entry_size as i64);
    let old_offset = fresh(ctx);
    ctx.body.push(Instr::IBin {
        dst: old_offset,
        op: BinOpIR::Mul,
        lhs: safe_idx,
        rhs: stride_val,
        ty: IrType::Int,
    });
    let old_base_ptr = emit_ptr_add(ctx, data_ptr, old_offset);
    let old_val_ptr = if val_offset == 0 {
        old_base_ptr
    } else {
        let val_off_val = emit_int_const(ctx, val_offset as i64);
        emit_ptr_add(ctx, old_base_ptr, val_off_val)
    };
    let old_val_loaded = load_value_copy(ctx, &val_ty, old_val_ptr, 0)?;
    let zero_payload = zero_value_for_type(ctx, &val_ty)?;
    let replaced_payload = fresh(ctx);
    ctx.body.push(Instr::ISelect {
        dst: replaced_payload,
        cond: found,
        then_v: old_val_loaded,
        else_v: zero_payload,
    });
    let zero_hi = emit_int_const(ctx, 0);
    let replaced_opt = ctx.variant_init(found, replaced_payload, zero_hi);
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
    let map_out_ty = Type::Map(Box::new(key_ty.clone()), Box::new(val_ty.clone()));
    let replaced_out_ty = Type::Option(Box::new(val_ty));
    let out = emit_tuple_pair(ctx, map_out_ty, header, replaced_out_ty, replaced_opt)?;
    Ok(Some(out))
}

fn lower_map_remove<'a>(ctx: &mut LowerCtx<'a>, args: &'a [Expr]) -> Result<Option<Value>> {
    if args.len() != 2 {
        anyhow::bail!("`std::map::remove` expects two arguments");
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
    let len_minus_one = fresh(ctx);
    ctx.body.push(Instr::IBin {
        dst: len_minus_one,
        op: BinOpIR::Sub,
        lhs: len,
        rhs: one,
        ty: IrType::Int,
    });
    let new_len = fresh(ctx);
    ctx.body.push(Instr::ISelect {
        dst: new_len,
        cond: found,
        then_v: len_minus_one,
        else_v: len,
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
    ctx.body.push(Instr::BlockBegin);
    ctx.body.push(Instr::BlockBegin);
    ctx.body.push(Instr::BrIfEqz {
        cond: found,
        depth: 0,
    });
    let bytes_before = fresh(ctx);
    ctx.body.push(Instr::IBin {
        dst: bytes_before,
        op: BinOpIR::Mul,
        lhs: found_idx,
        rhs: stride_val,
        ty: IrType::Int,
    });
    emit_memcpy_bytes(ctx, data_ptr, new_data, bytes_before)?;
    let idx_plus_one = fresh(ctx);
    ctx.body.push(Instr::IBin {
        dst: idx_plus_one,
        op: BinOpIR::Add,
        lhs: found_idx,
        rhs: one,
        ty: IrType::Int,
    });
    let remaining = fresh(ctx);
    ctx.body.push(Instr::IBin {
        dst: remaining,
        op: BinOpIR::Sub,
        lhs: len,
        rhs: idx_plus_one,
        ty: IrType::Int,
    });
    let bytes_after = fresh(ctx);
    ctx.body.push(Instr::IBin {
        dst: bytes_after,
        op: BinOpIR::Mul,
        lhs: remaining,
        rhs: stride_val,
        ty: IrType::Int,
    });
    let src_offset = fresh(ctx);
    ctx.body.push(Instr::IBin {
        dst: src_offset,
        op: BinOpIR::Mul,
        lhs: idx_plus_one,
        rhs: stride_val,
        ty: IrType::Int,
    });
    let src_ptr = emit_ptr_add(ctx, data_ptr, src_offset);
    let dst_offset = fresh(ctx);
    ctx.body.push(Instr::IBin {
        dst: dst_offset,
        op: BinOpIR::Mul,
        lhs: found_idx,
        rhs: stride_val,
        ty: IrType::Int,
    });
    let dst_ptr = emit_ptr_add(ctx, new_data, dst_offset);
    emit_memcpy_bytes(ctx, src_ptr, dst_ptr, bytes_after)?;
    ctx.body.push(Instr::Br { depth: 1 });
    ctx.body.push(Instr::BlockEnd);
    let copy_bytes = fresh(ctx);
    ctx.body.push(Instr::IBin {
        dst: copy_bytes,
        op: BinOpIR::Mul,
        lhs: len,
        rhs: stride_val,
        ty: IrType::Int,
    });
    emit_memcpy_bytes(ctx, data_ptr, new_data, copy_bytes)?;
    ctx.body.push(Instr::BlockEnd);
    let header = emit_collection_header(ctx, new_len, new_cap, new_data);
    Ok(Some(header))
}

fn lower_map_remove_take<'a>(ctx: &mut LowerCtx<'a>, args: &'a [Expr]) -> Result<Option<Value>> {
    if args.len() != 2 {
        anyhow::bail!("`std::map::remove_take` expects two arguments");
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
    let old_offset = fresh(ctx);
    ctx.body.push(Instr::IBin {
        dst: old_offset,
        op: BinOpIR::Mul,
        lhs: safe_idx,
        rhs: stride_val,
        ty: IrType::Int,
    });
    let old_base_ptr = emit_ptr_add(ctx, data_ptr, old_offset);
    let old_val_ptr = if val_offset == 0 {
        old_base_ptr
    } else {
        let val_off_val = emit_int_const(ctx, val_offset as i64);
        emit_ptr_add(ctx, old_base_ptr, val_off_val)
    };
    let old_val_loaded = load_value_copy(ctx, &val_ty, old_val_ptr, 0)?;
    let zero_payload = zero_value_for_type(ctx, &val_ty)?;
    let removed_payload = fresh(ctx);
    ctx.body.push(Instr::ISelect {
        dst: removed_payload,
        cond: found,
        then_v: old_val_loaded,
        else_v: zero_payload,
    });
    let zero_hi = emit_int_const(ctx, 0);
    let removed_opt = ctx.variant_init(found, removed_payload, zero_hi);
    let one = emit_int_const(ctx, 1);
    let len_minus_one = fresh(ctx);
    ctx.body.push(Instr::IBin {
        dst: len_minus_one,
        op: BinOpIR::Sub,
        lhs: len,
        rhs: one,
        ty: IrType::Int,
    });
    let new_len = fresh(ctx);
    ctx.body.push(Instr::ISelect {
        dst: new_len,
        cond: found,
        then_v: len_minus_one,
        else_v: len,
    });
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
    ctx.body.push(Instr::BlockBegin);
    ctx.body.push(Instr::BlockBegin);
    ctx.body.push(Instr::BrIfEqz {
        cond: found,
        depth: 0,
    });
    let bytes_before = fresh(ctx);
    ctx.body.push(Instr::IBin {
        dst: bytes_before,
        op: BinOpIR::Mul,
        lhs: found_idx,
        rhs: stride_val,
        ty: IrType::Int,
    });
    emit_memcpy_bytes(ctx, data_ptr, new_data, bytes_before)?;
    let idx_plus_one = fresh(ctx);
    ctx.body.push(Instr::IBin {
        dst: idx_plus_one,
        op: BinOpIR::Add,
        lhs: found_idx,
        rhs: one,
        ty: IrType::Int,
    });
    let remaining = fresh(ctx);
    ctx.body.push(Instr::IBin {
        dst: remaining,
        op: BinOpIR::Sub,
        lhs: len,
        rhs: idx_plus_one,
        ty: IrType::Int,
    });
    let bytes_after = fresh(ctx);
    ctx.body.push(Instr::IBin {
        dst: bytes_after,
        op: BinOpIR::Mul,
        lhs: remaining,
        rhs: stride_val,
        ty: IrType::Int,
    });
    let src_offset = fresh(ctx);
    ctx.body.push(Instr::IBin {
        dst: src_offset,
        op: BinOpIR::Mul,
        lhs: idx_plus_one,
        rhs: stride_val,
        ty: IrType::Int,
    });
    let src_ptr = emit_ptr_add(ctx, data_ptr, src_offset);
    let dst_offset = fresh(ctx);
    ctx.body.push(Instr::IBin {
        dst: dst_offset,
        op: BinOpIR::Mul,
        lhs: found_idx,
        rhs: stride_val,
        ty: IrType::Int,
    });
    let dst_ptr = emit_ptr_add(ctx, new_data, dst_offset);
    emit_memcpy_bytes(ctx, src_ptr, dst_ptr, bytes_after)?;
    ctx.body.push(Instr::Br { depth: 1 });
    ctx.body.push(Instr::BlockEnd);
    let copy_bytes = fresh(ctx);
    ctx.body.push(Instr::IBin {
        dst: copy_bytes,
        op: BinOpIR::Mul,
        lhs: len,
        rhs: stride_val,
        ty: IrType::Int,
    });
    emit_memcpy_bytes(ctx, data_ptr, new_data, copy_bytes)?;
    ctx.body.push(Instr::BlockEnd);
    let header = emit_collection_header(ctx, new_len, new_cap, new_data);
    let map_out_ty = Type::Map(Box::new(key_ty.clone()), Box::new(val_ty.clone()));
    let removed_out_ty = Type::Option(Box::new(val_ty));
    let out = emit_tuple_pair(ctx, map_out_ty, header, removed_out_ty, removed_opt)?;
    Ok(Some(out))
}
