use clg_ir::{BinOpIR, Instr, IrType, Value};
use clg_parser::parse;
use clg_typer::check;

// Phase 3.4 — purpose: lower simple add using param SSA ids
#[test]
fn lowers_add_function() {
    let src = r#"
        pure function add(a: Int, b: Int) -> Int { a + b }
    "#;
    let m = check(&parse(src).expect("parse ok")).expect("type-check+lower ok");
    assert_eq!(m.funcs.len(), 1);
    let f = &m.funcs[0];
    assert_eq!(f.name, "add");
    assert_eq!(f.params, vec![IrType::Int, IrType::Int]);
    assert_eq!(f.ret, Some(IrType::Int));
    assert_eq!(f.body.len(), 2); // IBin, Ret
    match &f.body[0] {
        Instr::IBin {
            dst,
            op,
            lhs,
            rhs,
            ..
        } => {
            assert_eq!(*op, BinOpIR::Add);
            assert_eq!(*lhs, Value(0)); // a
            assert_eq!(*rhs, Value(1)); // b
            assert_eq!(*dst, Value(2)); // next id after params
        }
        _ => panic!("expected IBin as first instr"),
    }
    match &f.body[1] {
        Instr::Ret { val } => assert_eq!(*val, Value(2)),
        _ => panic!("expected Ret as second instr"),
    }
}

// Phase 3.4 — purpose: lower call returning a value
#[test]
fn lowers_call_and_const() {
    let src = r#"
        pure function id(x: Int) -> Int { x }
        function main() -> Int { id(42) }
    "#;
    let m = check(&parse(src).expect("parse ok")).expect("type-check+lower ok");
    assert_eq!(m.funcs.len(), 2);
    let main = m
        .funcs
        .iter()
        .find(|f| f.name == "main")
        .expect("main exists");
    assert_eq!(main.params.len(), 0);
    assert_eq!(main.ret, Some(IrType::Int));
    assert_eq!(main.body.len(), 3); // const 42, call, ret
    match &main.body[0] {
        Instr::IConst { dst, ty, n } => {
            assert_eq!(*dst, Value(0));
            assert_eq!(*ty, IrType::Int);
            assert_eq!(*n, 42);
        }
        _ => panic!("expected IConst 42"),
    }
    match &main.body[1] {
        Instr::Call { dst, callee, args } => {
            // In this module, `id` is the first function, so index 0
            assert_eq!(*callee, 0);
            assert_eq!(args.as_slice(), &[Value(0)]);
            assert_eq!(*dst, Some(Value(1)));
        }
        _ => panic!("expected Call to id"),
    }
    matches!(&main.body[2], Instr::Ret { val: Value(1) });
}

// Phase 15.1 ƒ?" purpose: lower U64 parameters and return types
#[test]
fn lowers_u64_identity() {
    let src = r#"
        pure function id_u64(x: U64) -> U64 { x }
    "#;
    let m = check(&parse(src).expect("parse ok")).expect("type-check+lower ok");
    assert_eq!(m.funcs.len(), 1);
    let f = &m.funcs[0];
    assert_eq!(f.name, "id_u64");
    assert_eq!(f.params, vec![IrType::U64]);
    assert_eq!(f.ret, Some(IrType::U64));
    assert_eq!(f.body.len(), 1);
    match &f.body[0] {
        Instr::Ret { val } => assert_eq!(*val, Value(0)),
        _ => panic!("expected Ret as only instr"),
    }
}

#[test]
fn lowers_u128_u256_helpers() {
    let src = r#"
        pure function make_u128() -> U128 { std::u128::from_limbs(1, 2) }
        pure function make_u256() -> U256 { std::u256::from_limbs(1, 2, 3, 4) }
    "#;
    let m = check(&parse(src).expect("parse ok")).expect("type-check+lower ok");
    assert_eq!(m.funcs.len(), 2);
    let f128 = &m.funcs[0];
    assert_eq!(f128.ret, Some(IrType::U128));
    assert!(f128.body.iter().any(|instr| matches!(instr, Instr::U128Init { .. })));
    let f256 = &m.funcs[1];
    assert_eq!(f256.ret, Some(IrType::U256));
    assert!(f256.body.iter().any(|instr| matches!(instr, Instr::U256Init { .. })));
}

#[test]
fn lowers_u128_u256_literals() {
    let src = r#"
        pure function lit_u128() -> U128 { 42 }
        pure function cast_u256() -> U256 { U256(7) }
    "#;
    let m = check(&parse(src).expect("parse ok")).expect("type-check+lower ok");
    let f128 = m
        .funcs
        .iter()
        .find(|f| f.name == "lit_u128")
        .expect("lit_u128 exists");
    assert_eq!(f128.ret, Some(IrType::U128));
    assert!(f128.body.iter().any(|instr| matches!(instr, Instr::U128Init { .. })));
    let f256 = m
        .funcs
        .iter()
        .find(|f| f.name == "cast_u256")
        .expect("cast_u256 exists");
    assert_eq!(f256.ret, Some(IrType::U256));
    assert!(f256.body.iter().any(|instr| matches!(instr, Instr::U256Init { .. })));
}
