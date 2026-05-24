use clg_parser::parse;
use clg_typer::check;

#[test]
fn typer_accepts_u64_wrapping_aliases() {
    let src = r#"
        pure function calc(a: U64, b: U64) -> U64 {
            std::u64::add_wrapping(std::u64::sub_wrapping(a, b), std::u64::mul_wrapping(a, b))
        }
    "#;
    check(&parse(src).expect("parse ok")).expect("type-check ok");
}

#[test]
fn typer_accepts_u64_saturating_aliases() {
    let src = r#"
        pure function calc(a: U64, b: U64) -> U64 {
            std::u64::add_saturating(std::u64::sub_saturating(a, b), std::u64::mul_saturating(a, b))
        }
    "#;
    check(&parse(src).expect("parse ok")).expect("type-check ok");
}
