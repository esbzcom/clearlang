use clg_ir::{Instr, Value, VariantKind};
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

#[test]
fn lowers_if_let_some_to_select() {
    let src = r#"
        pure function pick(opt: Option<Int>) -> Int {
            if let Some(v) = opt { v } else { 0 }
        }
    "#;
    let ir = check(&parse(src).expect("parse ok")).expect("type-check+lower ok");
    let func = ir
        .funcs
        .iter()
        .find(|f| f.name == "pick")
        .expect("pick function present");

    assert!(
        func.body
            .iter()
            .any(|instr| matches!(instr, Instr::ISelect { .. })),
        "lowering must emit ISelect for if let Some"
    );
    assert!(
        !func
            .body
            .iter()
            .any(|instr| matches!(instr, Instr::ReturnIf { .. })),
        "if let should not emit ReturnIf"
    );
}

#[test]
fn lowers_option_coalesce_to_select() {
    let src = r#"
        pure function coalesce(opt: Option<Int>) -> Int { opt ?? 7 }
    "#;
    let ir = check(&parse(src).expect("parse ok")).expect("type-check+lower ok");
    let func = ir
        .funcs
        .iter()
        .find(|f| f.name == "coalesce")
        .expect("coalesce function present");

    assert!(
        func.body
            .iter()
            .any(|instr| matches!(instr, Instr::ISelect { .. })),
        "lowering must emit ISelect for ??"
    );
}

#[test]
fn lowers_if_let_err_to_select() {
    let src = r#"
        pure function pick(res: Result<Int, Int>) -> Int {
            if let Err(e) = res { e } else { 0 }
        }
    "#;
    let ir = check(&parse(src).expect("parse ok")).expect("type-check+lower ok");
    let func = ir
        .funcs
        .iter()
        .find(|f| f.name == "pick")
        .expect("pick function present");

    assert!(
        func.body
            .iter()
            .any(|instr| matches!(instr, Instr::ISelect { .. })),
        "lowering must emit ISelect for if let Err"
    );
}
#[test]
fn option_try_lowering_preserves_payload_and_propagation() {
    let src = r#"
        pure function inc(opt: Option<Int>) -> Option<Int> { Some(opt? + 1) }
    "#;
    let ir = check(&parse(src).expect("parse ok")).expect("type-check+lower ok");
    let func = ir
        .funcs
        .iter()
        .find(|f| f.name == "inc")
        .expect("function present");

    assert_eq!(
        func.body.len(),
        12,
        "expected canonical variant lowering shape"
    );

    match &func.body[0] {
        Instr::VariantLoadTag { variant, kind, .. } => {
            assert_eq!(*variant, Value(0));
            assert_eq!(*kind, VariantKind::Option);
        }
        other => panic!("expected VariantLoadTag at [0], found {other:?}"),
    }
    match &func.body[1] {
        Instr::VariantLoadPayloadLo { variant, .. } => assert_eq!(*variant, Value(0)),
        other => panic!("expected VariantLoadPayloadLo at [1], found {other:?}"),
    }
    match &func.body[2] {
        Instr::VariantLoadPayloadHi { variant, .. } => assert_eq!(*variant, Value(0)),
        other => panic!("expected VariantLoadPayloadHi at [2], found {other:?}"),
    }
    match &func.body[4] {
        Instr::IBin { lhs, rhs, .. } => {
            assert_eq!(*lhs, Value(1));
            assert_eq!(*rhs, Value(4));
        }
        other => panic!("expected equality compare before ReturnIf, found {other:?}"),
    }
    match &func.body[5] {
        Instr::ReturnIf { cond, ret } => {
            assert_eq!(*cond, Value(5));
            assert_eq!(*ret, Value(0));
        }
        other => panic!("expected ReturnIf wiring propagation, found {other:?}"),
    }
    match &func.body[7] {
        Instr::IBin { dst, lhs, rhs, .. } => {
            assert_eq!(*dst, Value(7));
            assert_eq!(*lhs, Value(2));
            assert_eq!(*rhs, Value(6));
        }
        other => panic!("expected addition for payload, found {other:?}"),
    }
    match &func.body[10] {
        Instr::VariantInit {
            dst,
            tag,
            payload_lo,
            payload_hi,
        } => {
            assert_eq!(*dst, Value(10));
            assert_eq!(*tag, Value(8));
            assert_eq!(*payload_lo, Value(7));
            assert_eq!(*payload_hi, Value(9));
        }
        other => panic!("expected VariantInit at [10], found {other:?}"),
    }
    match &func.body[11] {
        Instr::Ret { val } => assert_eq!(*val, Value(10)),
        other => panic!("expected Ret returning constructor value, found {other:?}"),
    }
}

#[test]
fn result_try_lowering_tracks_ok_flow() {
    let src = r#"
        pure function add(res: Result<Int, Int>) -> Result<Int, Int> { Ok(res? + 1) }
    "#;
    let ir = check(&parse(src).expect("parse ok")).expect("type-check+lower ok");
    let func = ir
        .funcs
        .iter()
        .find(|f| f.name == "add")
        .expect("function present");

    assert_eq!(
        func.body.len(),
        12,
        "expected canonical variant lowering shape"
    );

    match &func.body[0] {
        Instr::VariantLoadTag { variant, kind, .. } => {
            assert_eq!(*variant, Value(0));
            assert_eq!(*kind, VariantKind::Result);
        }
        other => panic!("expected VariantLoadTag at [0], found {other:?}"),
    }
    match &func.body[5] {
        Instr::ReturnIf { cond, ret } => {
            assert_eq!(*cond, Value(5));
            assert_eq!(*ret, Value(0));
        }
        other => panic!("expected ReturnIf propagation for Err, found {other:?}"),
    }
    match &func.body[7] {
        Instr::IBin { dst, lhs, rhs, .. } => {
            assert_eq!(*dst, Value(7));
            assert_eq!(*lhs, Value(2));
            assert_eq!(*rhs, Value(6));
        }
        other => panic!("expected addition for Ok payload, found {other:?}"),
    }
    match &func.body[10] {
        Instr::VariantInit {
            dst,
            tag,
            payload_lo,
            payload_hi,
        } => {
            assert_eq!(*dst, Value(10));
            assert_eq!(*tag, Value(8));
            assert_eq!(*payload_lo, Value(7));
            assert_eq!(*payload_hi, Value(9));
        }
        other => panic!("expected VariantInit at [10], found {other:?}"),
    }
    match &func.body[11] {
        Instr::Ret { val } => assert_eq!(*val, Value(10)),
        other => panic!("expected Ret returning constructor value, found {other:?}"),
    }
}

#[test]
fn lowers_enum_variant_constructor_with_tuple_payload() {
    let src = r#"
        enum Pair {
            Pair(Int, Int)
        }
        pure function make() -> Pair { Pair::Pair(1, 2) }
    "#;
    let ir = check(&parse(src).expect("parse ok")).expect("type-check+lower ok");
    let func = ir
        .funcs
        .iter()
        .find(|f| f.name == "make")
        .expect("make function present");

    assert!(
        func.body
            .iter()
            .any(|instr| matches!(instr, Instr::Alloc { .. })),
        "expected heap allocation for tuple payload"
    );
    assert!(
        func.body
            .iter()
            .any(|instr| matches!(instr, Instr::Store { .. })),
        "expected stores for tuple payload fields"
    );
    assert!(
        func.body
            .iter()
            .any(|instr| matches!(instr, Instr::VariantInit { .. })),
        "expected VariantInit for enum constructor"
    );
}

#[test]
fn lowers_enum_match_to_tag_check_and_select() {
    let src = r#"
        enum Color {
            Red,
            Green
        }
        pure function pick(c: Color) -> Int {
            match c {
                Color::Red => 1,
                Color::Green => 2
            }
        }
    "#;
    let ir = check(&parse(src).expect("parse ok")).expect("type-check+lower ok");
    let func = ir
        .funcs
        .iter()
        .find(|f| f.name == "pick")
        .expect("pick function present");

    assert!(
        func.body.iter().any(|instr| matches!(
            instr,
            Instr::VariantLoadTag {
                kind: VariantKind::Enum { max_tag: 2 },
                ..
            }
        )),
        "expected VariantLoadTag with enum max_tag"
    );
    assert!(
        func.body
            .iter()
            .any(|instr| matches!(instr, Instr::BrIfEqz { .. })),
        "expected branch on tag check for enum match lowering"
    );
    assert!(
        func.body
            .iter()
            .any(|instr| matches!(instr, Instr::Store { .. })),
        "expected Store into match result slot"
    );
    assert!(
        func.body
            .iter()
            .any(|instr| matches!(instr, Instr::Load { .. })),
        "expected Load from match result slot"
    );
}
