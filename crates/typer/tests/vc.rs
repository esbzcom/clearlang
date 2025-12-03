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
fn generates_vc_for_if_let_sugar() {
    let src = r#"
        pure function default_or_zero(opt: Option<Int>) -> Int
            ensure { result >= 0 }
        {
            if let Some(v) = opt { v } else { 0 }
        }
    "#;
    let ast = parse(src).expect("parse ok");
    type_check_only(&ast).expect("type-check ok");
    let vcs = generate_vcs(&ast);
    assert_eq!(vcs.len(), 1);
    let vc = &vcs[0];
    assert_eq!(vc.function, "default_or_zero");
    assert_eq!(vc.pre.ast, "true");
    assert!(vc.post.ast.contains(">="));
    assert!(vc.vc_smt2.contains("=>"));
    assert!(vc.vc_smt2.contains("cl.variant.tag"));
    assert!(!vc.vc_smt2.contains("unsupported"));
}

#[test]
fn generates_vc_for_try_sugar() {
    let src = r#"
        pure function bump_when_positive(opt: Option<Int>) -> Option<Int>
            ensure { result == result }
        {
            if opt? > 0 { Some(opt? + 1) } else { None }
        }
    "#;
    let ast = parse(src).expect("parse ok");
    type_check_only(&ast).expect("type-check ok");
    let vcs = generate_vcs(&ast);
    assert_eq!(vcs.len(), 1);
    let vc = &vcs[0];
    assert_eq!(vc.function, "bump_when_positive");
    assert!(vc.post.ast.contains("=="));
    assert!(vc.vc_smt2.contains("=>"));
    assert!(vc.vc_smt2.contains("cl.variant.payload_lo"));
    assert!(!vc.vc_smt2.contains("unsupported"));
}
#[test]
fn generates_vc_for_option_coalesce() {
    let src = r#"
        pure function pick(opt: Option<Int>) -> Int
            ensure { result >= 0 }
        { opt ?? 7 }
    "#;
    let ast = parse(src).expect("parse ok");
    type_check_only(&ast).expect("type-check ok");
    let vcs = generate_vcs(&ast);
    assert_eq!(vcs.len(), 1);
    let vc = &vcs[0];
    assert!(vc.vc_smt2.contains("cl.variant.tag"));
    assert!(!vc.vc_smt2.contains("unsupported"));
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

#[test]
fn generates_vcs_for_loop_invariant_and_variant() {
    let src = r#"
        pure function countdown(n: Int) -> Int {
            while n > 0 invariant { n >= 0 } variant { n } {
                n;
            }
            n
        }
    "#;
    let ast = parse(src).expect("parse ok");
    type_check_only(&ast).expect("type-check ok");
    let vcs = generate_vcs(&ast);
    assert_eq!(vcs.len(), 3);
    let ids: Vec<&str> = vcs.iter().map(|vc| vc.vc_id.as_str()).collect();
    assert!(ids.iter().any(|id| id.contains("invariant")));
    assert!(ids.iter().any(|id| id.contains("variant_nonneg")));
    assert!(ids.iter().any(|id| id.contains("variant_decrease")));
    assert!(vcs.iter().any(|vc| vc.post.ast.contains(">= 0")));
    assert!(vcs
        .iter()
        .any(|vc| vc.vc_smt2.contains("declare-const cl.loop.variant.next.0")));
}

#[test]
fn generates_vc_for_refined_alias_params_and_returns() {
    let src = r#"
        type Nat = Int where n >= 0;
        pure function inc(a: Nat) -> Nat { a + 1 }
    "#;
    let ast = parse(src).expect("parse ok");
    type_check_only(&ast).expect("type-check ok");
    let vcs = generate_vcs(&ast);
    assert_eq!(vcs.len(), 1);
    let vc = &vcs[0];
    assert_eq!(vc.function, "inc");
    assert_eq!(vc.vc_id, "vc:0");
    assert_eq!(vc.pre.ast, "a >= 0");
    assert_eq!(vc.post.ast, "result >= 0");
}

#[test]
fn propagates_refinement_through_let_bindings() {
    let src = r#"
        type Nat = Int where n >= 0;
        pure function copy(a: Nat) -> Nat {
            let y = a;
            y
        }
    "#;
    let ast = parse(src).expect("parse ok");
    type_check_only(&ast).expect("type-check ok");
    let vcs = generate_vcs(&ast);
    assert_eq!(vcs.len(), 1);
    let vc = &vcs[0];
    assert!(vc.pre.ast.contains("a >= 0"));
    assert!(vc.pre.ast.contains("y >= 0"));
    assert!(vc.post.ast.contains("result >= 0"));
}

#[test]
fn propagates_refinement_into_option_match_binders() {
    let src = r#"
        type Nat = Int where n >= 0;
        pure function unwrap(opt: Option<Nat>) -> Int
            ensure { result >= 0 }
        {
            match opt {
                Some(v) => v,
                None => 0
            }
        }
    "#;
    let ast = parse(src).expect("parse ok");
    type_check_only(&ast).expect("type-check ok");
    let vcs = generate_vcs(&ast);
    assert_eq!(vcs.len(), 1);
    let vc = &vcs[0];
    assert!(vc.pre.ast.contains("v >= 0"));
    assert!(vc.vc_smt2.contains("=>"));
}
