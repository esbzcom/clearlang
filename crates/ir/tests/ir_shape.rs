use clg_ir::{BinOpIR, Function, Instr, IrType, Module, Value};

// Phase 3.3 — purpose: construct a minimal function IR with const, bin, and ret
#[test]
fn ir_builds_const_add_ret() {
    let mut body = Vec::new();
    let v0 = Value(0);
    let v1 = Value(1);
    let v2 = Value(2);
    body.push(Instr::IConst {
        dst: v0,
        ty: IrType::Int,
        n: 20,
    });
    body.push(Instr::IConst {
        dst: v1,
        ty: IrType::Int,
        n: 22,
    });
    body.push(Instr::IBin {
        dst: v2,
        op: BinOpIR::Add,
        lhs: v0,
        rhs: v1,
    });
    body.push(Instr::Ret { val: v2 });

    let f = Function {
        name: "main".into(),
        params: vec![],
        ret: Some(IrType::Int),
        body,
    };

    let m = Module {
        funcs: vec![f.clone()],
    };
    assert_eq!(m.funcs.len(), 1);
    assert_eq!(m.funcs[0].name, "main");
    assert_eq!(m.funcs[0].params.len(), 0);
    assert_eq!(m.funcs[0].ret, Some(IrType::Int));
    assert_eq!(m.funcs[0].body.len(), 4);
}

// Phase 3.3 — purpose: allow call with/without destination (void vs value)
#[test]
fn ir_call_instr_variants() {
    let a = Value(0);
    let res = Value(1);
    // Callee indices are function indices in the module (e.g., 0 = first function)
    let call_void = Instr::Call {
        dst: None,
        callee: 0,
        args: vec![a],
    };
    let call_val = Instr::Call {
        dst: Some(res),
        callee: 1,
        args: vec![a],
    };
    matches!(call_void, Instr::Call { dst: None, .. });
    matches!(call_val, Instr::Call { dst: Some(_), .. });
}
