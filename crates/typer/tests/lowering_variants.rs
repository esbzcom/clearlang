use clg_ir::{Instr, Value};
use clg_parser::parse;
use clg_typer::check;

#[test]
fn lowers_option_some_to_variant_init() {
    let src = r#"
        pure function make() -> Option<Int> { Some(1) }
    "#;
    let ir = check(&parse(src).expect("parse ok")).expect("type-check+lower ok");
    let func = ir
        .funcs
        .iter()
        .find(|f| f.name == "make")
        .expect("make function present");

    // expect: const 1, const tag, const zero, VariantInit, Ret
    assert_eq!(func.body.len(), 5);
    match &func.body[3] {
        Instr::VariantInit {
            dst,
            tag,
            payload_lo,
            payload_hi,
        } => {
            assert_eq!(*dst, Value(3));
            assert_eq!(*payload_lo, Value(0));
            assert_eq!(*tag, Value(1));
            assert_eq!(*payload_hi, Value(2));
        }
        other => panic!("expected VariantInit, found {other:?}"),
    }
}

#[test]
fn lowers_option_none_to_zeroed_variant() {
    let src = r#"
        pure function nothing() -> Option<Int> { None }
    "#;
    let ir = check(&parse(src).expect("parse ok")).expect("type-check+lower ok");
    let func = ir
        .funcs
        .iter()
        .find(|f| f.name == "nothing")
        .expect("function present");

    // expect: const tag(0), const zero, VariantInit, Ret
    assert_eq!(func.body.len(), 4);
    match &func.body[2] {
        Instr::VariantInit {
            tag,
            payload_lo,
            payload_hi,
            ..
        } => {
            assert_eq!(*tag, Value(0));
            assert_eq!(*payload_lo, Value(1));
            assert_eq!(*payload_hi, Value(1));
        }
        other => panic!("expected VariantInit, found {other:?}"),
    }
}
#[test]
fn lowers_option_try_to_return_if() {
    let src = r#"
        pure function inc(opt: Option<Int>) -> Option<Int> { Some(opt? + 1) }
    "#;
    let ir = check(&parse(src).expect("parse ok")).expect("type-check+lower ok");
    let func = ir
        .funcs
        .iter()
        .find(|f| f.name == "inc")
        .expect("inc function present");

    assert!(
        func.body
            .iter()
            .any(|instr| matches!(instr, Instr::ReturnIf { .. })),
        "lowering must emit ReturnIf for opt? propagation"
    );
}

#[test]
fn lowers_result_try_to_return_if() {
    let src = r#"
        pure function add(res: Result<Int, Int>) -> Result<Int, Int> { Ok(res? + 1) }
    "#;
    let ir = check(&parse(src).expect("parse ok")).expect("type-check+lower ok");
    let func = ir
        .funcs
        .iter()
        .find(|f| f.name == "add")
        .expect("add function present");

    assert!(
        func.body
            .iter()
            .any(|instr| matches!(instr, Instr::ReturnIf { .. })),
        "lowering must emit ReturnIf for res? propagation"
    );
}
