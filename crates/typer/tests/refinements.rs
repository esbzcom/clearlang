use clg_parser::parse;
use clg_typer::check;

#[test]
fn alias_resolves_in_params_and_returns() {
    let src = r#"
        type Nat = Int where n >= 0;
        pure function id(n: Nat) -> Nat { n }
        function main() -> Nat { id(1) }
    "#;
    check(&parse(src).expect("parse ok")).expect("typecheck ok");
}

#[test]
fn predicate_must_be_bool() {
    let src = r#"
        type Bad = Int where 1;
        pure function id(x: Bad) -> Int { x }
    "#;
    let err = check(&parse(src).expect("parse ok")).expect_err("predicate not bool");
    let s = format!("{err:#}");
    assert!(s.contains("T704"), "missing code T704: {s}");
}

#[test]
fn unsat_predicate_is_rejected() {
    let src = r#"
        type Impossible = Int where n >= 0 && n < 0;
        pure function id(x: Impossible) -> Int { x }
    "#;
    let err = check(&parse(src).expect("parse ok")).expect_err("predicate unsat");
    let s = format!("{err:#}");
    assert!(s.contains("T708"), "missing code T708: {s}");
}

#[test]
fn unsat_predicate_with_offset_is_rejected() {
    let src = r#"
        type Impossible = Int where n + 1 >= 0 && n + 1 < 0;
        pure function id(x: Impossible) -> Int { x }
    "#;
    let err = check(&parse(src).expect("parse ok")).expect_err("predicate unsat");
    let s = format!("{err:#}");
    assert!(s.contains("T708"), "missing code T708: {s}");
}

#[test]
fn cyclic_alias_is_rejected() {
    let src = r#"
        type A = B where a >= 0;
        type B = A where b >= 0;
        pure function f(x: A) -> A { x }
    "#;
    let err = check(&parse(src).expect("parse ok")).expect_err("cyclic alias");
    let s = format!("{err:#}");
    assert!(s.contains("T703"), "missing code T703: {s}");
}

#[test]
fn alias_cannot_shadow_resource() {
    let src = r#"
        resource File { fd: Int; drop {} }
        type File = Int where f >= 0;
        pure function f(fd: File) -> Int { fd }
    "#;
    let err = check(&parse(src).expect("parse ok")).expect_err("resource conflict");
    let s = format!("{err:#}");
    assert!(s.contains("T702"), "missing code T702: {s}");
}

#[test]
fn bounded_and_equality_refinements_typecheck() {
    let src = r#"
        type Nat = Int where n >= 0;
        type Small = Int where s >= 0 && s <= 10;
        type FortyTwo = Int where f == 42;
        pure function id(n: Nat) -> Nat { n }
        pure function clamp(x: Small) -> Small { x }
        pure function meaning() -> FortyTwo { 42 }
        function main() -> Int { id(1) + clamp(5) + meaning() }
    "#;
    check(&parse(src).expect("parse ok")).expect("typecheck ok");
}

#[test]
fn refinements_preserve_through_locals_calls_and_containers() {
    let src = r#"
        type Nat = Int where n >= 0;
        pure function add1(n: Nat) -> Nat { n + 1 }
        pure function wrap(n: Nat) -> Option<Nat> { Some(n) }
        pure function zero() -> Nat { 0 }
        pure function use(x: Int) -> Nat {
            let y = add1(x);
            match wrap(y) {
                Some(v) => v,
                None => zero()
            }
        }
        function main() -> Nat { use(1) }
    "#;
    check(&parse(src).expect("parse ok")).expect("typecheck ok");
}

#[test]
fn coalesce_and_try_preserve_refinements() {
    let src = r#"
        type Nat = Int where n >= 0;
        pure function zero() -> Nat { 0 }
        pure function pick(opt: Option<Nat>) -> Nat { opt ?? zero() }
        pure function bump(opt: Option<Nat>) -> Option<Nat> { Some(opt?) }
        pure function okify(res: Result<Nat, Nat>) -> Result<Nat, Nat> { Ok(res?) }
        function main() -> Int { 0 }
    "#;
    check(&parse(src).expect("parse ok")).expect("typecheck ok");
}

#[test]
fn alias_to_resource_collection_is_rejected() {
    let src = r#"
        resource R { v: Int; drop {} }
        type Bad = List<R> where xs > 0;
        pure function f(xs: Bad) -> Int { 0 }
    "#;
    let err = check(&parse(src).expect("parse ok")).expect_err("resource in collection");
    let s = format!("{err:#}");
    assert!(s.contains("T706"), "missing code T706: {s}");
}

#[test]
fn refined_value_cannot_flow_to_unrefined_param() {
    let src = r#"
        type Nat = Int where n >= 0;
        pure function takes_int(x: Int) -> Int { x }
        pure function use(n: Nat) -> Int { takes_int(n) }
    "#;
    let err = check(&parse(src).expect("parse ok")).expect_err("refinement loss");
    let s = format!("{err:#}");
    assert!(s.contains("T705"), "missing code T705: {s}");
}

#[test]
fn refined_value_cannot_return_as_base() {
    let src = r#"
        type Nat = Int where n >= 0;
        pure function bad(n: Nat) -> Int { n }
    "#;
    let err = check(&parse(src).expect("parse ok")).expect_err("refinement loss");
    let s = format!("{err:#}");
    assert!(s.contains("T705"), "missing code T705: {s}");
}

#[test]
fn refined_value_cannot_flow_into_unrefined_container() {
    let src = r#"
        type Nat = Int where n >= 0;
        pure function wrap(n: Nat) -> Option<Int> { Some(n) }
    "#;
    let err = check(&parse(src).expect("parse ok")).expect_err("refinement loss");
    let s = format!("{err:#}");
    assert!(s.contains("T705"), "missing code T705: {s}");
}

#[test]
fn refinement_drop_on_shadowing_is_rejected() {
    let src = r#"
        type Nat = Int where n >= 0;
        pure function step(n: Nat) -> Int {
            let n = n - 1;
            n
        }
    "#;
    let err = check(&parse(src).expect("parse ok")).expect_err("refinement loss");
    let s = format!("{err:#}");
    assert!(s.contains("T705"), "missing code T705: {s}");
}

#[test]
fn refined_alias_cannot_wrap_resource() {
    let src = r#"
        resource File { fd: Int; drop {} }
        type Bad = File where f == f;
        pure function f(x: Bad) -> Int { 0 }
    "#;
    let err = check(&parse(src).expect("parse ok")).expect_err("resource in refinement");
    let s = format!("{err:#}");
    assert!(s.contains("T706"), "missing code T706: {s}");
}

#[test]
fn impure_refinement_predicates_are_rejected() {
    let src = r#"
        mut function bump(x: Int) -> Int { x + 1 }
        type Bad = Int where bump(1) > 0;
        pure function f(x: Bad) -> Int { x }
    "#;
    let err = check(&parse(src).expect("parse ok")).expect_err("impure predicate");
    let s = format!("{err:#}");
    assert!(s.contains("T707"), "missing code T707: {s}");
}

#[test]
fn refinement_drop_in_if_is_rejected() {
    let src = r#"
        type Nat = Int where n >= 0;
        pure function f(n: Nat) -> Int {
            if n > 0 { n } else { 0 }
        }
    "#;
    let err = check(&parse(src).expect("parse ok")).expect_err("branch refinement drop");
    let s = format!("{err:#}");
    assert!(s.contains("T301"), "missing code T301: {s}");
}

#[test]
fn refinement_drop_in_option_coalesce_is_rejected() {
    let src = r#"
        type Nat = Int where n >= 0;
        pure function pick(opt: Option<Nat>) -> Int { opt ?? 0 }
    "#;
    let err = check(&parse(src).expect("parse ok")).expect_err("coalesce refinement drop");
    let s = format!("{err:#}");
    assert!(s.contains("T204"), "missing code T204: {s}");
}

#[test]
fn refinement_drop_in_try_is_rejected() {
    let src = r#"
        type Nat = Int where n >= 0;
        pure function unwrap(opt: Option<Nat>) -> Option<Int> { opt? }
    "#;
    let err = check(&parse(src).expect("parse ok")).expect_err("try refinement drop");
    let s = format!("{err:#}");
    assert!(s.contains("T603"), "missing code T603: {s}");
}
