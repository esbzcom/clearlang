use clg_codegen_wasm::emit_from_ir;
use clg_parser::parse;
use clg_typer::check;
use wasmparser::{Parser, Payload};

mod common;

fn align4(n: u32) -> u32 {
    (n + 3) & !3
}

fn align16(n: u32) -> u32 {
    (n + 15) & !15
}

#[test]
fn memory_pages_and_heap_ptr_fit_string_data() {
    // Two large, distinct literals
    let a = "a".repeat(40_000);
    let b = "b".repeat(40_000);
    let src = format!(
        "function main() -> Int {{ std::str::len(std::str::concat({a:?}, {b:?})) }}",
        a = a,
        b = b,
    );
    let ast = parse(&src).expect("parse ok");
    let ir = check(&ast).expect("type-check+lower ok");
    let wasm = emit_from_ir(&ir).expect("codegen ok");

    // Compute expected size of data area (headers + bytes, aligned)
    let total = align4(4 + 40_000) + align4(4 + 40_000);
    let heap_start_expect = align16(total);
    let min_pages_expect = heap_start_expect.div_ceil(65536).max(1);

    let mut min_pages_found: Option<u64> = None;
    let mut heap_init_found: Option<i32> = None;

    for payload in Parser::new(0).parse_all(&wasm) {
        match payload.expect("payload") {
            Payload::MemorySection(rdr) => {
                for mem in rdr {
                    let mem = mem.expect("memory");
                    min_pages_found = Some(mem.initial);
                }
            }
            Payload::GlobalSection(rdr) => {
                if heap_init_found.is_none() {
                    if let Some(glob_res) = rdr.into_iter().next() {
                        let glob = glob_res.expect("global");
                        let mut ops = glob.init_expr.get_operators_reader();
                        use wasmparser::Operator;
                        if let Ok(Operator::I32Const { value }) = ops.read() {
                            heap_init_found = Some(value);
                        }
                    }
                }
            }
            _ => {}
        }
    }

    let found_pages = min_pages_found.expect("memory section present");
    assert_eq!(found_pages as u32, min_pages_expect, "memory min pages");
    let heap_init = heap_init_found.expect("heap_ptr global present");
    assert_eq!(heap_init as u32, heap_start_expect, "heap_ptr init value");
}
