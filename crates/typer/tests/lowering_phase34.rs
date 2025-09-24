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
        Instr::IBin { dst, op, lhs, rhs } => {
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
