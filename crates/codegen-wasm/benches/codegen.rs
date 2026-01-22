use clg_codegen_wasm::{emit_from_ir_with_opts, CodegenOpts};
use clg_ir::{Function as IrFunction, Instr, IrType, Module as IrModule, Value};
use clg_parser::parse;
use clg_typer::check;
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn build_string_ir(count: usize) -> IrModule {
    let mut body: Vec<Instr> = Vec::with_capacity(count + 2);
    for i in 0..count {
        body.push(Instr::IStringConst {
            dst: Value(i as u32),
            s: format!("string_{i}"),
        });
    }
    let ret_value = Value(count as u32);
    body.push(Instr::IConst {
        dst: ret_value,
        ty: IrType::Int,
        n: 0,
    });
    body.push(Instr::Ret { val: ret_value });
    IrModule {
        funcs: vec![IrFunction {
            name: "main".to_string(),
            params: Vec::new(),
            ret: Some(IrType::Int),
            body,
        }],
    }
}

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
                    export_aliases: Vec::new(),
                },
            )
            .unwrap()
        })
    });

    let no_strings = build_string_ir(0);
    c.bench_function("codegen::emit_wasm_strings_0", |b| {
        b.iter(|| {
            emit_from_ir_with_opts(
                black_box(&no_strings),
                CodegenOpts {
                    debug_names: false,
                    proof_section: None,
                    export_aliases: Vec::new(),
                },
            )
            .unwrap()
        })
    });

    let many_strings = build_string_ir(1024);
    c.bench_function("codegen::emit_wasm_strings_1024", |b| {
        b.iter(|| {
            emit_from_ir_with_opts(
                black_box(&many_strings),
                CodegenOpts {
                    debug_names: false,
                    proof_section: None,
                    export_aliases: Vec::new(),
                },
            )
            .unwrap()
        })
    });
}

criterion_group!(benches, bench_codegen);
criterion_main!(benches);
