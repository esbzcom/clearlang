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
    assert_eq!(vc.refinements.len(), 2);
    let param = &vc.refinements[0];
    assert_eq!(param.alias, "Nat");
    assert_eq!(param.binder, "n");
    assert_eq!(param.substitution.ast, "a");
    assert_eq!(param.predicate.ast, "a >= 0");
    assert!(matches!(
        param.attachment.kind,
        RefinementAttachmentKind::Param
    ));
    match &param.attachment.detail {
        RefinementAttachmentDetail::Param { param } => assert_eq!(param, "a"),
        _ => panic!("expected param attachment"),
    }
    let ret = &vc.refinements[1];
    assert_eq!(ret.predicate.ast, "result >= 0");
    assert!(matches!(
        ret.attachment.kind,
        RefinementAttachmentKind::Return
    ));
    match &ret.attachment.detail {
        RefinementAttachmentDetail::Return { result } => assert_eq!(result, "result"),
        _ => panic!("expected return attachment"),
    }
}

#[test]
fn generates_vc_for_inline_refined_params_and_returns() {
    let src = r#"
        pure function inc(a: Int where a >= 0) -> Int where result >= 0 { a + 1 }
    "#;
    let ast = parse(src).expect("parse ok");
    type_check_only(&ast).expect("type-check ok");
    let vcs = generate_vcs(&ast);
    assert_eq!(vcs.len(), 1);
    let vc = &vcs[0];
    assert_eq!(vc.pre.ast, "a >= 0");
    assert_eq!(vc.post.ast, "result >= 0");
    assert_eq!(vc.refinements.len(), 2);
}

#[test]
fn generates_vc_for_generic_refined_alias_instantiation() {
    let src = r#"
        type Stable<T> = T where v == v;
        pure function keep(x: Stable<Int>) -> Stable<Int> { x }
    "#;
    let ast = parse(src).expect("parse ok");
    type_check_only(&ast).expect("type-check ok");
    let vcs = generate_vcs(&ast);
    assert_eq!(vcs.len(), 1);
    let vc = &vcs[0];
    assert_eq!(vc.pre.ast, "x == x");
    assert_eq!(vc.post.ast, "result == result");
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
    assert!(!vc.pre.ast.contains("y >= 0"));
    assert!(vc.post.ast.contains("result >= 0"));
    assert_eq!(
        vc.pre.ast.matches("a >= 0").count(),
        2,
        "expected let refinement to inline into parameter-level predicate"
    );
    let has_let = vc
        .refinements
        .iter()
        .any(|premise| match &premise.attachment.detail {
            RefinementAttachmentDetail::Flow(detail) => {
                matches!(detail.flow_kind, RefinementFlowKind::Let)
                    && detail.name.as_deref() == Some("y")
            }
            _ => false,
        });
    assert!(has_let, "expected let flow refinement premise for y");
}

#[test]
fn propagates_refinement_into_option_match_binders() {
    let src = r#"
        type Nat = Int where n >= 0;
        pure function unwrap(opt: Option<Nat>) -> Int
            ensure { result >= 0 }
        {
            match opt {
                Some(v) => v + 0,
                None => 0
            }
        }
    "#;
    let ast = parse(src).expect("parse ok");
    type_check_only(&ast).expect("type-check ok");
    let vcs = generate_vcs(&ast);
    assert_eq!(vcs.len(), 1);
    let vc = &vcs[0];
    assert!(
        !vc.pre.ast.contains("v >= 0"),
        "match-arm binder refinements should not leak unconstrained local symbols into VC preconditions"
    );
    assert!(vc.vc_smt2.contains("=>"));
}

#[test]
fn vc_precondition_order_matches_runtime_guards() {
    let src = r#"
        type Nat = Int where n >= 0;
        pure function bump(n: Nat) -> Nat
            require { n > 1 }
        {
            let y = n;
            y
        }
    "#;
    let ast = parse(src).expect("parse ok");
    type_check_only(&ast).expect("type-check ok");
    let vcs = generate_vcs(&ast);
    assert_eq!(vcs.len(), 1);
    let parts: Vec<&str> = vcs[0].pre.ast.split("&&").map(|s| s.trim()).collect();
    assert_eq!(parts, vec!["n > 1", "n >= 0", "n >= 0"]);
}

#[test]
fn vc_smt_declares_parameters_with_sorts_and_closes_let_locals() {
    let src = r#"
        type Nat = Int where n >= 0;
        pure function copy_if(flag: Bool, a: Nat) -> Nat {
            let y = a;
            if flag { y } else { a }
        }
    "#;
    let ast = parse(src).expect("parse ok");
    type_check_only(&ast).expect("type-check ok");
    let vcs = generate_vcs(&ast);
    assert_eq!(vcs.len(), 1);
    let vc = &vcs[0];
    assert!(vc.vc_smt2.contains("(declare-const flag Bool)"));
    assert!(vc.vc_smt2.contains("(declare-const a Int)"));
    assert!(
        !vc.vc_smt2.contains("(>= y 0)"),
        "solver prelude should not contain unconstrained local refinement symbols"
    );
}

#[test]
fn refined_return_ensure_appends_after_explicit_ensures() {
    let src = r#"
        type Nat = Int where n >= 0;
        pure function inc(n: Nat) -> Nat
            ensure { result > 0 }
        { n + 1 }
    "#;
    let ast = parse(src).expect("parse ok");
    type_check_only(&ast).expect("type-check ok");
    let vcs = generate_vcs(&ast);
    assert_eq!(vcs.len(), 2);
    assert_eq!(vcs[0].post.ast, "result > 0");
    assert_eq!(vcs[1].post.ast, "result >= 0");
}

#[test]
fn emits_call_arg_refinement_premise() {
    let src = r#"
        type Nat = Int where n >= 0;
        pure function takes(n: Nat) -> Nat { n }
        pure function caller(x: Int) -> Nat { takes(x + 1) }
    "#;
    let ast = parse(src).expect("parse ok");
    type_check_only(&ast).expect("type-check ok");
    let vcs = generate_vcs(&ast);
    let vc = vcs
        .iter()
        .find(|vc| vc.function == "caller" && vc.vc_id == "vc:0")
        .expect("caller vc");
    assert!(vc.pre.ast.contains("x + 1 >= 0"));
    let call = vc
        .refinements
        .iter()
        .find(|premise| match &premise.attachment.detail {
            RefinementAttachmentDetail::Flow(detail) => {
                matches!(detail.flow_kind, RefinementFlowKind::CallArg)
                    && detail.callee.as_deref() == Some("takes")
                    && detail.arg_index == Some(0)
            }
            _ => false,
        });
    let call = call.expect("call arg refinement premise");
    assert_eq!(call.substitution.ast, "x + 1");
    assert_eq!(call.predicate.ast, "x + 1 >= 0");
    assert!(vc.vc_smt2.contains("define-fun cl.ref.premise.0.sub"));
    assert!(vc.vc_smt2.contains("define-fun cl.ref.premise.0.pred"));
}

#[test]
fn declares_bitwise_and_builtin_helpers() {
    let src = r#"
        pure function check(a: Bytes, b: Bytes, x: U64, y: U64) -> Bool
            ensure { result == std::bytes::eq_ct(a, b) }
            ensure { (x & y) == x }
        {
            std::bytes::eq_ct(a, b)
        }
    "#;
    let ast = parse(src).expect("parse ok");
    let output = check_with_vcs(&ast).expect("type-check ok");
    assert!(
        output
            .vcs
            .iter()
            .any(|vc| vc.vc_smt2.contains("bvand")),
        "expected concrete bvand encoding in SMT output"
    );
    assert!(
        output
            .vcs
            .iter()
            .any(|vc| vc.vc_smt2.contains("declare-fun clg.u64.to_int ((_ BitVec 64)) Int")),
        "expected U64 bitvector bridge declaration in SMT prelude"
    );
    assert!(
        output
            .vcs
            .iter()
            .any(|vc| vc.vc_smt2.contains("declare-fun |std::bytes::eq_ct|")),
        "expected builtin declarations for std::bytes::eq_ct"
    );
    let vc = output
        .vcs
        .iter()
        .find(|vc| vc.vc_id == "vc:0")
        .expect("primary ensure vc");
    assert!(
        vc.assumptions
            .iter()
            .all(|a| !matches!(a.category, AssumptionCategory::Unsigned)),
        "U64-covered paths should not emit unsigned.int_model boundary"
    );
    assert!(
        vc.assumptions
            .iter()
            .all(|a| !matches!(a.category, AssumptionCategory::Bitwise)),
        "U64-covered bitwise operators should not emit bitwise.uninterpreted boundary"
    );
    let crypto = vc
        .assumptions
        .iter()
        .find(|a| matches!(a.category, AssumptionCategory::Crypto))
        .expect("expected crypto assumption boundary");
    assert!(
        crypto
            .symbols
            .iter()
            .any(|symbol| symbol == "std::bytes::eq_ct"),
        "expected std::bytes::eq_ct boundary in crypto assumption"
    );
    let primitive = vc
        .assumptions
        .iter()
        .find(|a| matches!(a.category, AssumptionCategory::Primitive))
        .expect("expected primitive assumption boundary");
    assert!(
        primitive
            .symbols
            .iter()
            .any(|symbol| symbol == "std::bytes::eq_ct"),
        "expected std::bytes::eq_ct boundary in primitive assumption"
    );
}

#[test]
fn labels_u64_rotate_and_byte_intrinsics_as_bitwise_assumptions() {
    let src = r#"
        pure function rotate_and_pack(x: U64, y: U64) -> U64
            ensure { result == std::u64::rotl(x, y) }
            ensure { std::u64::to_bytes_le(result) == std::u64::to_bytes_be(result) }
        {
            let z = std::u64::rotr(x, y);
            std::u64::from_bytes_le(std::u64::to_bytes_le(z))
        }
    "#;
    let ast = parse(src).expect("parse ok");
    let output = check_with_vcs(&ast).expect("type-check ok");
    let vc = output
        .vcs
        .iter()
        .find(|vc| vc.vc_id == "vc:0")
        .expect("primary ensure vc");

    let bitwise = vc
        .assumptions
        .iter()
        .find(|a| matches!(a.category, AssumptionCategory::Bitwise))
        .expect("expected bitwise assumption boundary");

    assert!(
        bitwise.symbols.iter().any(|s| s == "std::u64::to_bytes_le"),
        "expected std::u64::to_bytes_le in bitwise symbols"
    );
    assert!(
        bitwise
            .symbols
            .iter()
            .any(|s| s == "std::u64::from_bytes_le"),
        "expected std::u64::from_bytes_le in bitwise symbols"
    );
}

#[test]
fn prefixed_literals_match_decimal_vc_outputs_for_u64_paths() {
    let decimal_src = r#"
        pure function mask(x: U64) -> U64
            ensure { result == (x & 255) }
        { x & 170 }
    "#;
    let prefixed_src = r#"
        pure function mask(x: U64) -> U64
            ensure { result == (x & 0xFF) }
        { x & 0b1010_1010 }
    "#;

    let decimal_ast = parse(decimal_src).expect("decimal parse ok");
    let prefixed_ast = parse(prefixed_src).expect("prefixed parse ok");
    let decimal = check_with_vcs(&decimal_ast).expect("decimal type-check ok");
    let prefixed = check_with_vcs(&prefixed_ast).expect("prefixed type-check ok");

    assert_eq!(decimal.vcs.len(), prefixed.vcs.len());

    let decimal_shapes: Vec<_> = decimal
        .vcs
        .iter()
        .map(|vc| {
            let assumptions: Vec<_> = vc
                .assumptions
                .iter()
                .map(|a| {
                    (
                        a.id,
                        a.category.as_str(),
                        a.status,
                        a.message,
                        a.symbols.clone(),
                    )
                })
                .collect();
            (
                vc.vc_id.clone(),
                vc.pre.ast.clone(),
                vc.post.ast.clone(),
                vc.vc_smt2.clone(),
                assumptions,
            )
        })
        .collect();

    let prefixed_shapes: Vec<_> = prefixed
        .vcs
        .iter()
        .map(|vc| {
            let assumptions: Vec<_> = vc
                .assumptions
                .iter()
                .map(|a| {
                    (
                        a.id,
                        a.category.as_str(),
                        a.status,
                        a.message,
                        a.symbols.clone(),
                    )
                })
                .collect();
            (
                vc.vc_id.clone(),
                vc.pre.ast.clone(),
                vc.post.ast.clone(),
                vc.vc_smt2.clone(),
                assumptions,
            )
        })
        .collect();

    assert_eq!(
        decimal_shapes, prefixed_shapes,
        "prefixed literals should preserve VC/assumption determinism vs decimal equivalents"
    );
}

#[test]
fn labels_external_dependencies_as_assumed_boundaries() {
    let src = r#"
        pure function rely(x: Int) -> Int
            ensure { result == ext::dep(x) }
        { ext::dep(x) }
    "#;
    let ast = parse(src).expect("parse ok");
    let std_types = StdTypeMap::new();
    let external = vec![ExternalBuiltinSig {
        name: "ext::dep".to_string(),
        params: vec![Param {
            kind: ParamKind::Borrow,
            name: "x".to_string(),
            ty: Type::Int,
        }],
        ret: Type::Int,
        effect: Effect::Pure,
        route: clg_typer::BuiltinRoute::PackageImport,
    }];
    let output = check_with_vcs_with_std_and_external(&ast, &std_types, &external)
        .expect("type-check with external dependency");
    let vc = output
        .vcs
        .iter()
        .find(|vc| vc.vc_id == "vc:0")
        .expect("primary vc");
    let external_boundary = vc
        .assumptions
        .iter()
        .find(|a| matches!(a.category, AssumptionCategory::External))
        .expect("expected external dependency assumption boundary");
    assert!(
        external_boundary
            .symbols
            .iter()
            .any(|symbol| symbol == "ext::dep"),
        "expected ext::dep symbol in external boundary"
    );
}

#[test]
fn list_readonly_vcs_carry_no_assumptions() {
    let src = r#"
        pure function empty_list() -> List<Int> { std::list::new() }

        pure function list_readonly(l: List<Int>) -> Bool
            require { std::list::len(l) >= 0 }
            ensure { std::list::is_empty(empty_list()) }
        {
            match std::list::get(l, 0) {
                Some(_) => false,
                None => std::list::is_empty(l)
            }
        }
    "#;
    let ast = parse(src).expect("parse ok");
    let output = check_with_vcs(&ast).expect("type-check ok");
    let vc = output
        .vcs
        .iter()
        .find(|vc| vc.function == "list_readonly" && vc.vc_id == "vc:0")
        .expect("list_readonly vc:0");

    assert!(
        vc.assumptions.is_empty(),
        "read-only list reasoning must not emit assumption boundaries"
    );
    assert!(
        vc.pre.ast.contains("std::list::len(l) >= 0"),
        "VC precondition should retain deterministic list-length guard"
    );
    assert!(
        vc.vc_smt2
            .contains("(forall ((l Int) (i Int)) (=> (and (<= 0 i) (< i (|std::list::len| l))) (= (cl.variant.tag (|std::list::get| l i)) 1)))"),
        "list read-only VC must encode in-range index safety axiom for std::list::get"
    );
    assert!(
        vc.vc_smt2
            .contains("(forall ((l Int) (i Int)) (=> (or (< i 0) (>= i (|std::list::len| l))) (= (cl.variant.tag (|std::list::get| l i)) 0)))"),
        "list read-only VC must encode out-of-range index safety axiom for std::list::get"
    );
    assert!(
        vc.vc_smt2
            .contains("(forall ((l_before Int) (l_after Int) (i Int)) (=> (and (= l_before l_after) (<= 0 i) (< i (|std::list::len| l_before))) (= (|std::list::get| l_before i) (|std::list::get| l_after i))))"),
        "list read-only VC must encode unchanged-state get value-preservation axiom"
    );
}

#[test]
fn list_append_pop_vcs_carry_no_assumptions() {
    let src = r#"
        pure function list_append_pop(l: List<Int>, x: Int) -> Bool
            ensure { std::list::len(std::list::push(l, x)) == std::list::len(l) + 1 }
            ensure {
                if std::list::len(l) > 0 {
                    match std::list::pop(l) {
                        Some(_) => true,
                        None => false
                    }
                } else {
                    match std::list::pop(l) {
                        Some(_) => false,
                        None => true
                    }
                }
            }
        {
            true
        }
    "#;
    let ast = parse(src).expect("parse ok");
    let output = check_with_vcs(&ast).expect("type-check ok");
    let vc = output
        .vcs
        .iter()
        .find(|vc| vc.function == "list_append_pop" && vc.vc_id == "vc:0")
        .expect("list_append_pop vc:0");

    assert!(
        vc.assumptions.is_empty(),
        "append/pop list reasoning must not emit assumption boundaries"
    );
    assert!(
        vc.vc_smt2
            .contains("(assert (forall ((l Int) (x Int)) (= (|std::list::len| (|std::list::push| l x)) (+ (|std::list::len| l) 1))))"),
        "list append/pop VC must encode push length-delta axiom"
    );
    assert!(
        vc.vc_smt2
            .contains("(assert (forall ((l Int)) (=> (> (|std::list::len| l) 0) (= (cl.variant.tag (|std::list::pop| l)) 1))))"),
        "list append/pop VC must encode pop Some axiom for non-empty lists"
    );
    assert!(
        vc.vc_smt2
            .contains("(assert (forall ((l Int)) (=> (<= (|std::list::len| l) 0) (= (cl.variant.tag (|std::list::pop| l)) 0))))"),
        "list append/pop VC must encode pop None axiom for empty lists"
    );
}

#[test]
fn list_indexed_mutation_vcs_carry_no_assumptions() {
    let src = r#"
        pure function list_indexed_mutation(l: List<Int>, x: Int, i: Int) -> Bool
            require { 0 <= i && i < std::list::len(l) }
            ensure { std::list::len(std::list::insert(l, x, i)) == std::list::len(l) + 1 }
            ensure { std::list::len(std::list::remove(l, i)) == std::list::len(l) - 1 }
            ensure { std::list::len(std::list::remove_take(l, i)[0]) == std::list::len(l) - 1 }
        {
            match std::list::remove_take(l, i)[1] {
                Some(v) => std::list::get(l, i) == Some(v),
                None => false
            }
        }
    "#;
    let ast = parse(src).expect("parse ok");
    let output = check_with_vcs(&ast).expect("type-check ok");
    let vc = output
        .vcs
        .iter()
        .find(|vc| vc.function == "list_indexed_mutation" && vc.vc_id == "vc:0")
        .expect("list_indexed_mutation vc:0");

    assert!(
        vc.assumptions.is_empty(),
        "indexed list mutation reasoning must not emit assumption boundaries"
    );
    assert!(
        vc.vc_smt2
            .contains("(assert (forall ((l Int) (x Int) (i Int)) (=> (and (<= 0 i) (<= i (|std::list::len| l))) (= (|std::list::len| (|std::list::insert| l x i)) (+ (|std::list::len| l) 1)))))"),
        "list indexed-mutation VC must encode insert length-delta axiom"
    );
    assert!(
        vc.vc_smt2
            .contains("(assert (forall ((l Int) (i Int)) (=> (and (<= 0 i) (< i (|std::list::len| l))) (= (|std::list::len| (|std::list::remove| l i)) (- (|std::list::len| l) 1)))))"),
        "list indexed-mutation VC must encode remove length-delta axiom"
    );
    assert!(
        vc.vc_smt2
            .contains("(declare-fun cl.list.remove_take.list (Int) Int)"),
        "list indexed-mutation VC must declare remove_take list projection helper"
    );
    assert!(
        vc.vc_smt2
            .contains("(assert (forall ((l Int) (i Int)) (=> (and (<= 0 i) (< i (|std::list::len| l))) (= (|std::list::len| (cl.list.remove_take.list (|std::list::remove_take| l i))) (- (|std::list::len| l) 1)))))"),
        "list indexed-mutation VC must encode in-range remove_take length axiom"
    );
}

#[test]
fn list_checked_mutation_vcs_carry_no_assumptions() {
    let src = r#"
        pure function list_checked_mutation(l: List<Int>, x: Int, i: Int) -> Bool
            ensure { result == result }
        {
            match std::list::insert_checked(l, x, i) {
                Ok(updated) => std::list::len(updated) >= std::list::len(l),
                Err(_err_insert) => match std::list::remove_checked(l, i) {
                    Ok(updated2) => std::list::len(updated2) <= std::list::len(l),
                    Err(_err_remove) => true
                }
            }
        }
    "#;
    let ast = parse(src).expect("parse ok");
    let output = check_with_vcs(&ast).expect("type-check ok");
    let vc = output
        .vcs
        .iter()
        .find(|vc| vc.function == "list_checked_mutation" && vc.vc_id == "vc:0")
        .expect("list_checked_mutation vc:0");

    assert!(
        vc.assumptions.is_empty(),
        "checked list mutation reasoning must not emit assumption boundaries"
    );
    assert!(
        vc.vc_smt2
            .contains("(assert (forall ((l Int) (x Int) (i Int)) (=> (and (<= 0 i) (<= i (|std::list::len| l))) (= (cl.variant.tag (|std::list::insert_checked| l x i)) 1))))"),
        "list checked-mutation VC must encode insert_checked Ok-tag axiom"
    );
    assert!(
        vc.vc_smt2
            .contains("(assert (forall ((l Int) (i Int)) (=> (and (<= 0 i) (< i (|std::list::len| l))) (= (cl.variant.tag (|std::list::remove_checked| l i)) 1))))"),
        "list checked-mutation VC must encode remove_checked Ok-tag axiom"
    );
    assert!(
        vc.vc_smt2
            .contains("(assert (forall ((l Int) (x Int) (i Int)) (=> (and (<= 0 i) (<= i (|std::list::len| l))) (= (cl.variant.payload_lo (|std::list::insert_checked| l x i)) (|std::list::insert| l x i)))))"),
        "list checked-mutation VC must encode insert_checked payload-to-insert axiom"
    );
}

#[test]
fn map_readonly_vcs_carry_no_assumptions() {
    let src = r#"
        pure function empty_map() -> Map<Int, Int> {
            std::map::new()
        }

        pure function map_readonly(m: Map<Int, Int>, k: Int) -> Bool
            ensure { result == result }
        {
            if std::map::contains(m, k) {
                match std::map::get(m, k) {
                    Some(v) => v == v,
                    None => false
                }
            } else {
                match std::map::get(m, k) {
                    Some(_) => false,
                    None => std::map::len(m) >= 0 && (std::map::is_empty(empty_map()) || std::map::is_empty(empty_map()) == false)
                }
            }
        }
    "#;
    let ast = parse(src).expect("parse ok");
    let output = check_with_vcs(&ast).expect("type-check ok");
    let vc = output
        .vcs
        .iter()
        .find(|vc| vc.function == "map_readonly" && vc.vc_id == "vc:0")
        .expect("map_readonly vc:0");

    assert!(
        vc.assumptions.is_empty(),
        "read-only map reasoning must not emit assumption boundaries"
    );
    assert!(
        vc.vc_smt2
            .contains("(assert (= (|std::map::len| (|std::map::new|)) 0))"),
        "map read-only VC must encode new->len axiom"
    );
    assert!(
        vc.vc_smt2
            .contains("(assert (|std::map::is_empty| (|std::map::new|)))"),
        "map read-only VC must encode new->is_empty axiom"
    );
    assert!(
        vc.vc_smt2
            .contains("(assert (forall ((m Int) (k Int)) (=> (|std::map::contains| m k) (= (cl.variant.tag (|std::map::get| m k)) 1))))"),
        "map read-only VC must encode contains=>get(Some) axiom"
    );
    assert!(
        vc.vc_smt2
            .contains("(assert (forall ((m Int) (k Int)) (=> (not (|std::map::contains| m k)) (= (cl.variant.tag (|std::map::get| m k)) 0))))"),
        "map read-only VC must encode !contains=>get(None) axiom"
    );
}

#[test]
fn map_membership_overwrite_vcs_carry_no_assumptions() {
    let src = r#"
        pure function map_membership_overwrite(m: Map<Int, Int>, k: Int, j: Int, v: Int) -> Bool
            ensure { result == result }
        {
            let after_insert = std::map::insert(m, k, v);
            let after_take = std::map::insert_take(m, k, v);
            let after_take_map = after_take[0];
            if k == j {
                match std::map::get(after_insert, j) {
                    Some(got) => got == v,
                    None => false
                }
            } else {
                std::map::contains(after_insert, j) == std::map::contains(m, j)
                    && std::map::contains(after_take_map, j) == std::map::contains(m, j)
            }
        }
    "#;
    let ast = parse(src).expect("parse ok");
    let output = check_with_vcs(&ast).expect("type-check ok");
    let vc = output
        .vcs
        .iter()
        .find(|vc| vc.function == "map_membership_overwrite" && vc.vc_id == "vc:0")
        .expect("map_membership_overwrite vc:0");

    assert!(
        vc.assumptions.is_empty(),
        "map membership/overwrite reasoning must not emit assumption boundaries"
    );
    assert!(
        vc.vc_smt2
            .contains("(assert (forall ((m Int) (k Int) (v Int)) (|std::map::contains| (|std::map::insert| m k v) k)))"),
        "map overwrite VC must encode insert=>contains(new key) axiom"
    );
    assert!(
        vc.vc_smt2
            .contains("(assert (forall ((m Int) (k Int) (v Int)) (= (|std::map::len| (|std::map::insert| m k v)) (ite (|std::map::contains| m k) (|std::map::len| m) (+ (|std::map::len| m) 1)))))"),
        "map overwrite VC must encode insert length delta axiom"
    );
    assert!(
        vc.vc_smt2
            .contains("(assert (forall ((m Int) (k Int) (v Int)) (= (cl.map.insert_take.map (|std::map::insert_take| m k v)) (|std::map::insert| m k v))))"),
        "map overwrite VC must encode insert_take map projection axiom"
    );
    assert!(
        vc.vc_smt2
            .contains("(assert (forall ((m Int) (k Int) (v Int)) (=> (|std::map::contains| m k) (= (cl.variant.tag (cl.map.insert_take.prev (|std::map::insert_take| m k v))) 1))))"),
        "map overwrite VC must encode insert_take replaced-value Some-tag axiom"
    );
}

#[test]
fn map_mutation_take_vcs_carry_no_assumptions() {
    let src = r#"
        pure function map_mutation_take(m: Map<Int, Int>, k: Int, j: Int) -> Bool
            ensure { result == result }
        {
            let after_remove = std::map::remove(m, k);
            let after_take = std::map::remove_take(m, k);
            let after_take_map = after_take[0];
            if k == j {
                std::map::contains(after_remove, j) == false
                    && std::map::contains(after_take_map, j) == false
            } else {
                std::map::contains(after_remove, j) == std::map::contains(m, j)
                    && std::map::contains(after_take_map, j) == std::map::contains(m, j)
            }
        }
    "#;
    let ast = parse(src).expect("parse ok");
    let output = check_with_vcs(&ast).expect("type-check ok");
    let vc = output
        .vcs
        .iter()
        .find(|vc| vc.function == "map_mutation_take" && vc.vc_id == "vc:0")
        .expect("map_mutation_take vc:0");

    assert!(
        vc.assumptions.is_empty(),
        "map mutation/take reasoning must not emit assumption boundaries"
    );
    assert!(
        vc.vc_smt2
            .contains("(assert (forall ((m Int) (k Int)) (not (|std::map::contains| (|std::map::remove| m k) k))))"),
        "map mutation/take VC must encode remove clears-key membership axiom"
    );
    assert!(
        vc.vc_smt2
            .contains("(assert (forall ((m Int) (k Int)) (= (|std::map::len| (|std::map::remove| m k)) (ite (|std::map::contains| m k) (- (|std::map::len| m) 1) (|std::map::len| m)))))"),
        "map mutation/take VC must encode remove length axiom"
    );
    assert!(
        vc.vc_smt2
            .contains("(assert (forall ((m Int) (k Int)) (= (cl.map.remove_take.map (|std::map::remove_take| m k)) (|std::map::remove| m k))))"),
        "map mutation/take VC must encode remove_take map projection axiom"
    );
    assert!(
        vc.vc_smt2
            .contains("(assert (forall ((m Int) (k Int)) (=> (|std::map::contains| m k) (= (cl.variant.tag (cl.map.remove_take.prev (|std::map::remove_take| m k))) 1))))"),
        "map mutation/take VC must encode remove_take Some-tag axiom when key is present"
    );
}

#[test]
fn set_subset_membership_vcs_carry_no_assumptions() {
    let src = r#"
        pure function subset_membership(a: Set<Int>, b: Set<Int>, x: Int) -> Bool
            require { std::set::subset(a, b) && std::set::contains(a, x) }
            ensure { result == std::set::contains(b, x) }
        {
            std::set::contains(b, x)
        }
    "#;
    let ast = parse(src).expect("parse ok");
    let output = check_with_vcs(&ast).expect("type-check ok");
    let vc = output
        .vcs
        .iter()
        .find(|vc| vc.function == "subset_membership" && vc.vc_id == "vc:0")
        .expect("subset_membership vc:0");

    assert!(
        vc.assumptions.is_empty(),
        "set membership/subset reasoning must not emit assumption boundaries"
    );
    assert!(vc.pre.ast.contains("std::set::subset(a, b)"));
    assert!(vc.pre.ast.contains("std::set::contains(a, x)"));
    assert!(vc.vc_smt2.contains("|std::set::subset|"));
    assert!(vc.vc_smt2.contains("|std::set::contains|"));
    assert!(
        vc.vc_smt2
            .contains("(forall ((a Int) (b Int)) (= (|std::set::subset| a b)"),
        "subset VCs must include finite-set subset axiom"
    );
}

#[test]
fn set_algebra_vcs_carry_no_assumptions() {
    let src = r#"
        pure function set_algebra(a: Set<Int>, b: Set<Int>) -> Bool
            ensure { std::set::subset(std::set::intersect(a, b), std::set::union(a, b)) }
            ensure { result == std::set::subset(std::set::diff(a, b), a) }
        {
            std::set::subset(std::set::diff(a, b), a)
        }
    "#;
    let ast = parse(src).expect("parse ok");
    let output = check_with_vcs(&ast).expect("type-check ok");
    let vc = output
        .vcs
        .iter()
        .find(|vc| vc.function == "set_algebra" && vc.vc_id == "vc:0")
        .expect("set_algebra vc:0");
    assert!(vc.assumptions.is_empty(), "set algebra VC should be assumption-free");
    assert!(
        vc.vc_smt2.contains(
            "(forall ((a Int) (b Int) (x Int)) (= (|std::set::contains| (|std::set::union| a b) x)"
        ),
        "set algebra VCs must include union membership axiom"
    );
    assert!(
        vc.vc_smt2.contains(
            "(forall ((a Int) (b Int) (x Int)) (= (|std::set::contains| (|std::set::intersect| a b) x)"
        ),
        "set algebra VCs must include intersect membership axiom"
    );
    assert!(
        vc.vc_smt2.contains(
            "(forall ((a Int) (b Int) (x Int)) (= (|std::set::contains| (|std::set::diff| a b) x)"
        ),
        "set algebra VCs must include diff membership axiom"
    );
}

#[test]
fn set_cardinality_vcs_carry_no_assumptions() {
    let src = r#"
        pure function set_cardinality(a: Set<Int>, b: Set<Int>) -> Bool
            ensure { std::set::len(std::set::intersect(a, b)) <= std::set::len(a) }
            ensure { std::set::len(std::set::union(a, b)) >= std::set::len(a) }
            ensure { std::set::len(std::set::diff(a, b)) <= std::set::len(a) }
        {
            true
        }
    "#;
    let ast = parse(src).expect("parse ok");
    let output = check_with_vcs(&ast).expect("type-check ok");
    let vc = output
        .vcs
        .iter()
        .find(|vc| vc.function == "set_cardinality" && vc.vc_id == "vc:0")
        .expect("set_cardinality vc:0");
    assert!(
        vc.assumptions.is_empty(),
        "set cardinality VC should be assumption-free"
    );
    assert!(
        vc.vc_smt2
            .contains("(forall ((a Int) (b Int)) (<= (|std::set::len| (|std::set::intersect| a b)) (|std::set::len| a)))"),
        "set cardinality VCs must include intersect cardinality axiom"
    );
    assert!(
        vc.vc_smt2
            .contains("(forall ((a Int) (b Int)) (>= (|std::set::len| (|std::set::union| a b)) (|std::set::len| a)))"),
        "set cardinality VCs must include union cardinality axiom"
    );
    assert!(
        vc.vc_smt2
            .contains("(forall ((a Int) (b Int)) (<= (|std::set::len| (|std::set::diff| a b)) (|std::set::len| a)))"),
        "set cardinality VCs must include diff cardinality axiom"
    );
}
