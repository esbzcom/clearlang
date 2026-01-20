use clg_codegen_wasm::{emit_from_ir_with_opts, CodegenOpts};
use clg_parser::parse;
use clg_typer::check;
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn bench_codegen(c: &mut Criterion) {
    let src = include_str!("../../../clearlang-tests/17_hello_str.clear");
    let ast = parse(src).expect("parse ok");
    let ir = check(&ast).expect("typecheck ok");
    c.bench_function("codegen::emit_wasm", |b| {
        b.iter(|| {
            emit_from_ir_with_opts(
                black_box(&ir),
                CodegenOpts {
                    debug_names: false,
                    proof_section: None,
                },
            )
            .unwrap()
        })
    });
}

criterion_group!(benches, bench_codegen);
criterion_main!(benches);
