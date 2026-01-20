use clg_parser::parse;
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn bench_parse(c: &mut Criterion) {
    let src = include_str!("../../../clearlang-tests/03_nested_calls.clear");
    c.bench_function("parser::parse_nested_calls", |b| {
        b.iter(|| parse(black_box(src)).expect("parse ok"))
    });
}

criterion_group!(benches, bench_parse);
criterion_main!(benches);
