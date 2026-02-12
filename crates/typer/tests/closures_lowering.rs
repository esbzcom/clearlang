use clg_ir::Instr;
use clg_parser::parse;
use clg_typer::check;

fn find_func<'a>(module: &'a clg_ir::Module, name: &str) -> &'a clg_ir::Function {
    module
        .funcs
        .iter()
        .find(|func| func.name == name)
        .unwrap_or_else(|| panic!("missing function `{name}`"))
}

#[test]
fn lowers_non_capturing_lambda_to_closure_record() {
    let src = r#"
function make() -> function(Int) -> Int {
    (x: Int) => x + 1
}
"#;

    let module = check(&parse(src).expect("parse ok")).expect("type-check+lower ok");
    let make = find_func(&module, "make");
    assert!(
        module
            .funcs
            .iter()
            .any(|func| func.name.starts_with("__clg_lambda_")),
        "expected synthetic lambda body function"
    );

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
    let make = find_func(&module, "make");

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
fn lowers_dynamic_closure_dispatch_with_signature_wrapper() {
    let src = r#"
function make() -> function(Int) -> Int {
    (x: Int) => x + 1
}

io function apply(f: function(Int) -> Int, x: Int) -> Int {
    f(x)
}

io function run(x: Int) -> Int {
    apply(make(), x)
}
"#;

    let module = check(&parse(src).expect("parse ok")).expect("type-check+lower ok");
    let apply = find_func(&module, "apply");
    let (dispatcher_idx, dispatch_args) = apply
        .body
        .iter()
        .find_map(|instr| match instr {
            Instr::Call { callee, args, .. } => Some((*callee as usize, args.len())),
            _ => None,
        })
        .expect("apply should call a dispatcher wrapper");
    assert_eq!(
        dispatch_args, 3,
        "dispatcher call should pass code_id, env_ptr, and one user argument"
    );

    let dispatcher = module
        .funcs
        .get(dispatcher_idx)
        .expect("dispatcher index should resolve");
    assert!(
        dispatcher.name.starts_with("__clg_dispatch_"),
        "expected dispatcher function call target, got `{}`",
        dispatcher.name
    );
    let lambda_idx = dispatcher
        .body
        .iter()
        .find_map(|instr| match instr {
            Instr::Call { callee, .. } => Some(*callee as usize),
            _ => None,
        })
        .expect("dispatcher should contain lambda case call");
    let lambda = module
        .funcs
        .get(lambda_idx)
        .expect("lambda callee index should resolve");
    assert!(
        lambda.name.starts_with("__clg_lambda_"),
        "dispatcher should call synthetic lambda function"
    );
}
