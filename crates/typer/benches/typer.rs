use clg_parser::parse;
use clg_typer::type_check_only;
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn bench_type_check(c: &mut Criterion) {
    let src = include_str!("../../../clearlang-tests/03_nested_calls.clear");
    let ast = parse(src).expect("parse ok");
    c.bench_function("typer::type_check_only", |b| {
        b.iter(|| type_check_only(black_box(&ast)).expect("typecheck ok"))
    });
}

criterion_group!(benches, bench_type_check);
criterion_main!(benches);
