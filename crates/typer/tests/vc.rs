use clg_parser::parse;
use clg_typer::{check_with_vcs, generate_vcs, type_check_only};

#[test]
fn generates_vc_for_contracts() {
    let src = r#"
        pure function inc(x: Int) -> Int
            require { 0 <= x }
            ensure { result > x }
        { x + 1 }
    "#;
    let ast = parse(src).expect("parse ok");
    let output = check_with_vcs(&ast).expect("type-check ok");
    assert_eq!(output.vcs.len(), 1);
    let vc = &output.vcs[0];
    assert_eq!(vc.function, "inc");
    assert_eq!(vc.vc_id, "vc:0");
    assert!(vc.pre.ast.contains("<="));
    assert!(vc.pre.smt2.contains("<="));
    assert!(vc.post.ast.contains(">"));
    assert!(vc.vc_smt2.contains("=>"));
    assert!(vc.vc_smt2.contains("(+ x 1)"));
}

#[test]
fn generates_multiple_vcs_for_multiple_ensures() {
    let src = r#"
        pure function stats(x: Int) -> Int
            require { x >= 0 }
            require { x < 100 }
            ensure { result >= x }
            ensure { result != x }
        { x + 1 }
    "#;
    let ast = parse(src).expect("parse ok");
    let output = check_with_vcs(&ast).expect("type-check ok");
    assert_eq!(output.vcs.len(), 2);
    let first = &output.vcs[0];
    let second = &output.vcs[1];
    assert_eq!(first.function, "stats");
    assert_eq!(second.function, "stats");
    assert_eq!(first.vc_id, "vc:0");
    assert_eq!(second.vc_id, "vc:1");
    assert_eq!(first.pre.ast, second.pre.ast);
    assert!(first.pre.ast.contains("&&"));
    assert!(first.post.ast.contains(">="));
    assert!(second.post.ast.contains("!="));
    assert!(first.vc_smt2.contains("=>"));
    assert!(second.vc_smt2.contains("=>"));
}
#[test]
fn generates_mut_pre_vc_for_mut_calls() {
    let src = r#"
        mut function push(l: List<Int>) -> List<Int>
            require { std::list::can_mut(l) }
        {
            std::list::push_mut(l, 1)
        }
    "#;
    let ast = parse(src).expect("parse ok");
    type_check_only(&ast).expect("type-check ok");
    let vcs = generate_vcs(&ast);
    assert_eq!(vcs.len(), 1);
    let vc = &vcs[0];
    assert_eq!(vc.function, "push");
    assert!(vc.vc_id.starts_with("mut_pre:std::list::push_mut:"));
    assert!(vc.pre.ast.contains("std::list::can_mut"));
    assert!(vc.post.ast.contains("std::list::can_mut"));
    assert!(vc.vc_smt2.contains("std::list::can_mut"));
}
