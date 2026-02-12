use clg_ir::Instr;
use clg_parser::parse;
use clg_typer::check;

#[test]
fn lowers_non_capturing_lambda_to_closure_record() {
    let src = r#"
function make() -> function(Int) -> Int {
    (x: Int) => x + 1
}
"#;

    let module = check(&parse(src).expect("parse ok")).expect("type-check+lower ok");
    assert_eq!(module.funcs.len(), 1);
    let make = &module.funcs[0];
    assert_eq!(make.name, "make");

    assert!(
        make.body.iter().any(|instr| matches!(
            instr,
            Instr::Alloc {
                size: 8,
                align: 4,
                ..
            }
        )),
        "expected closure record allocation"
    );
    assert!(
        make.body
            .iter()
            .any(|instr| matches!(instr, Instr::Store { offset: 0, .. })),
        "expected closure code_id store"
    );
    assert!(
        make.body
            .iter()
            .any(|instr| matches!(instr, Instr::Store { offset: 4, .. })),
        "expected closure env_ptr store"
    );
}

#[test]
fn lowers_capturing_lambda_with_environment_allocation() {
    let src = r#"
function make(base: Int) -> function(Int) -> Int {
    (x: Int) => x + base
}
"#;

    let module = check(&parse(src).expect("parse ok")).expect("type-check+lower ok");
    assert_eq!(module.funcs.len(), 1);
    let make = &module.funcs[0];

    let allocs = make
        .body
        .iter()
        .filter(|instr| matches!(instr, Instr::Alloc { .. }))
        .count();
    assert!(
        allocs >= 2,
        "expected env allocation + closure record allocation"
    );
    assert!(
        make.body
            .iter()
            .any(|instr| matches!(instr, Instr::Store { offset: 4, .. })),
        "expected closure env_ptr store"
    );
}

#[test]
fn lowering_rejects_dynamic_closure_dispatch_for_now() {
    let src = r#"
io function apply(f: function(Int) -> Int, x: Int) -> Int {
    f(x)
}
"#;

    let err = check(&parse(src).expect("parse ok")).expect_err("lowering should fail");
    let msg = format!("{err:#}");
    assert!(msg.contains("T017"), "unexpected error: {msg}");
    assert!(
        msg.contains("dynamic closure calls"),
        "unexpected error: {msg}"
    );
}
