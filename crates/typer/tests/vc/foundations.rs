use clg_ast::{Effect, Param, ParamKind, Type};
use clg_parser::parse;
use clg_typer::{
    check_with_vcs, check_with_vcs_with_std_and_external, generate_vcs, type_check_only,
    AssumptionCategory, ExternalBuiltinSig, RefinementAttachmentDetail, RefinementAttachmentKind,
    RefinementFlowKind, StdTypeMap,
};

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
fn generates_linear_branch_vc_for_resource_collection_flow() {
    let src = r#"
        resource File { drop {} }

        pure function choose(flag: Bool, consume files: List<File>) -> List<File> {
            if flag {
                std::list::remove_take(files, 0)[0]
            } else {
                std::list::remove_take(files, 0)[0]
            }
        }
    "#;
    let ast = parse(src).expect("parse ok");
    type_check_only(&ast).expect("type-check ok");
    let vcs = generate_vcs(&ast);
    let vc = vcs
        .iter()
        .find(|vc| vc.function == "choose" && vc.vc_id == "linear:branch:0")
        .expect("linear branch vc");
    assert!(vc.post.ast.contains("files"));
    assert!(vc
        .vc_smt2
        .contains("declare-const cl.linear.branch.0.0.then"));
    assert!(vc
        .vc_smt2
        .contains("declare-const cl.linear.branch.0.0.else"));
}

#[test]
fn generates_linear_loop_vc_for_resource_collection_flow() {
    let src = r#"
        resource File { drop {} }

        pure function loop_step(consume files: List<File>, n: Int) -> List<File> {
            while n > 0 invariant { n >= 0 } variant { n } {
                let out = std::list::remove_take(files, 0);
                let files = out[0];
            }
            files
        }
    "#;
    let ast = parse(src).expect("parse ok");
    type_check_only(&ast).expect("type-check ok");
    let vcs = generate_vcs(&ast);
    assert!(
        vcs.iter()
            .any(|vc| vc.function == "loop_step" && vc.vc_id == "linear:loop:0"),
        "expected linear loop vc"
    );
    let vc = vcs
        .iter()
        .find(|vc| vc.function == "loop_step" && vc.vc_id == "linear:loop:0")
        .expect("linear loop vc");
    assert!(vc.post.ast.contains("files"));
    assert!(vc
        .vc_smt2
        .contains("declare-const cl.linear.loop.0.0.before"));
    assert!(vc
        .vc_smt2
        .contains("declare-const cl.linear.loop.0.0.after"));
    assert!(
        vcs.iter()
            .any(|vc| vc.function == "loop_step" && vc.vc_id == "loop:0:invariant"),
        "expected existing loop invariant vc"
    );
}

#[test]
fn mut_linear_collection_flow_emits_linear_vc_without_mut_pre() {
    let src = r#"
        resource File { drop {} }

        mut function choose(flag: Bool, consume files: List<File>) -> List<File> {
            if flag {
                std::list::remove_take(files, 0)[0]
            } else {
                std::list::remove_take(files, 0)[0]
            }
        }
    "#;
    let ast = parse(src).expect("parse ok");
    type_check_only(&ast).expect("type-check ok");
    let vcs = generate_vcs(&ast);
    assert!(
        vcs.iter()
            .any(|vc| vc.function == "choose" && vc.vc_id == "linear:branch:0"),
        "expected linear branch vc for mut function"
    );
    assert!(
        !vcs.iter().any(|vc| vc.vc_id.starts_with("mut_pre:")),
        "did not expect mut_pre vcs for pure linear ownership APIs"
    );
}

#[test]
fn linear_branch_vc_tracks_helper_rebound_owner() {
    let src = r#"
        resource File { drop {} }

        pure function id(consume files: List<File>) -> List<File> { files }

        pure function choose(flag: Bool, consume files: List<File>) -> List<File> {
            let l = id(files);
            if flag {
                std::list::remove_take(l, 0)[0]
            } else {
                std::list::remove_take(l, 0)[0]
            }
        }
    "#;
    let ast = parse(src).expect("parse ok");
    type_check_only(&ast).expect("type-check ok");
    let vcs = generate_vcs(&ast);
    let vc = vcs
        .iter()
        .find(|vc| vc.function == "choose" && vc.vc_id == "linear:branch:0")
        .expect("linear branch vc for helper rebound owner");
    assert!(
        vc.post.ast.contains("l"),
        "expected rebound owner variable in linear branch vc"
    );
}

#[test]
fn linear_branch_vc_tracks_inline_owner_expression() {
    let src = r#"
        resource File { drop {} }

        pure function id(consume files: List<File>) -> List<File> { files }

        pure function choose(flag: Bool, consume files: List<File>) -> List<File> {
            if flag {
                std::list::remove_take(id(files), 0)[0]
            } else {
                std::list::remove_take(id(files), 0)[0]
            }
        }
    "#;
    let ast = parse(src).expect("parse ok");
    type_check_only(&ast).expect("type-check ok");
    let vcs = generate_vcs(&ast);
    let vc = vcs
        .iter()
        .find(|vc| vc.function == "choose" && vc.vc_id == "linear:branch:0")
        .expect("linear branch vc for inline owner expression");
    assert!(
        vc.post.ast.contains("files"),
        "expected underlying tracked owner in linear branch vc"
    );
}

#[test]
fn linear_loop_vc_tracks_inline_owner_expression() {
    let src = r#"
        resource File { drop {} }

        pure function id(consume files: List<File>) -> List<File> { files }

        pure function loop_step(consume files: List<File>, n: Int) -> List<File> {
            while n > 0 invariant { n >= 0 } variant { n } {
                let out = std::list::remove_take(id(files), 0);
                let files = out[0];
            }
            files
        }
    "#;
    let ast = parse(src).expect("parse ok");
    type_check_only(&ast).expect("type-check ok");
    let vcs = generate_vcs(&ast);
    let vc = vcs
        .iter()
        .find(|vc| vc.function == "loop_step" && vc.vc_id == "linear:loop:0")
        .expect("linear loop vc for inline owner expression");
    assert!(
        vc.post.ast.contains("files"),
        "expected underlying tracked owner in linear loop vc"
    );
}

#[test]
fn linear_branch_vc_inline_owner_respects_shadowing() {
    let src = r#"
        resource File { drop {} }

        pure function id(consume x: List<File>) -> List<File> { x }

        pure function choose(
            flag: Bool,
            consume files: List<File>,
            consume other: List<File>
        ) -> List<File> {
            if flag {
                std::list::remove_take(id({ let files = other; files }), 0)[0]
            } else {
                std::list::remove_take(id({ let files = other; files }), 0)[0]
            }
        }
    "#;
    let ast = parse(src).expect("parse ok");
    type_check_only(&ast).expect("type-check ok");
    let vcs = generate_vcs(&ast);
    let vc = vcs
        .iter()
        .find(|vc| vc.function == "choose" && vc.vc_id == "linear:branch:0")
        .expect("linear branch vc for shadowed inline owner expression");
    assert!(
        vc.post.ast.contains("other"),
        "expected shadowed owner variable in linear branch vc"
    );
    assert!(
        !vc.post.ast.contains("files"),
        "did not expect outer shadowed owner variable in linear branch vc"
    );
}

#[test]
fn linear_loop_vc_inline_owner_respects_shadowing() {
    let src = r#"
        resource File { drop {} }

        pure function id(consume x: List<File>) -> List<File> { x }

        pure function loop_step(
            consume files: List<File>,
            consume other: List<File>,
            n: Int
        ) -> (List<File>, List<File>) {
            while n > 0 invariant { n >= 0 } variant { n } {
                let out = std::list::remove_take(id({ let files = other; files }), 0);
                let other = out[0];
            }
            (files, other)
        }
    "#;
    let ast = parse(src).expect("parse ok");
    // This shape currently trips T804 in type-checking due loop-join ownership analysis,
    // but VC generation should still track the lexical owner (`other`) in the inline
    // ownership-API expression.
    let vcs = generate_vcs(&ast);
    let vc = vcs
        .iter()
        .find(|vc| vc.function == "loop_step" && vc.vc_id == "linear:loop:0")
        .expect("linear loop vc for shadowed inline owner expression");
    assert!(
        vc.post.ast.contains("other"),
        "expected shadowed owner variable in linear loop vc"
    );
    assert!(
        !vc.post.ast.contains("files"),
        "did not expect outer shadowed owner variable in linear loop vc"
    );
}

