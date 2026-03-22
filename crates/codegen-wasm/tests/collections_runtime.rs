#![allow(clippy::identity_op)]

use clg_codegen_wasm::emit_from_ir;
use clg_parser::parse;
use clg_typer::check;
use wasmtime::Val;

mod common;

fn align_up(value: i32, align: i32) -> i32 {
    if align <= 1 {
        return value;
    }
    let adj = align - 1;
    (value + adj) / align * align
}

fn write_i32(memory: &wasmtime::Memory, store: &mut wasmtime::Store<()>, addr: i32, value: i32) {
    memory
        .write(store, addr as usize, &value.to_le_bytes())
        .expect("write i32");
}

fn read_i32(memory: &wasmtime::Memory, store: &mut wasmtime::Store<()>, addr: i32) -> i32 {
    let mut buf = [0u8; 4];
    memory
        .read(store, addr as usize, &mut buf)
        .expect("read i32");
    i32::from_le_bytes(buf)
}

fn read_option_i32(
    memory: &wasmtime::Memory,
    store: &mut wasmtime::Store<()>,
    ptr: i32,
) -> (i32, i32) {
    let tag = read_i32(memory, store, ptr);
    let payload = read_i32(memory, store, ptr + 4);
    (tag, payload)
}

fn get_global_i32(
    instance: &wasmtime::Instance,
    store: &mut wasmtime::Store<()>,
    name: &str,
) -> i32 {
    let g = instance
        .get_global(&mut *store, name)
        .expect("global present");
    g.get(&mut *store).i32().expect("i32 global")
}

fn set_global_i32(
    instance: &wasmtime::Instance,
    store: &mut wasmtime::Store<()>,
    name: &str,
    value: i32,
) {
    let g = instance
        .get_global(&mut *store, name)
        .expect("global present");
    g.set(&mut *store, Val::I32(value)).expect("set global");
}

fn alloc_list(
    memory: &wasmtime::Memory,
    store: &mut wasmtime::Store<()>,
    heap_ptr: i32,
    elems: &[i32],
) -> (i32, i32) {
    let list_ptr = align_up(heap_ptr, 4);
    let len = elems.len() as i32;
    let cap = if len > 0 { len } else { 1 };
    let data_ptr = list_ptr + 16;

    write_i32(memory, store, list_ptr + 0, len);
    write_i32(memory, store, list_ptr + 4, cap);
    write_i32(memory, store, list_ptr + 8, 0);
    write_i32(memory, store, list_ptr + 12, data_ptr);

    for (idx, value) in elems.iter().enumerate() {
        let offset = data_ptr + (idx as i32) * 4;
        write_i32(memory, store, offset, *value);
    }

    let end = data_ptr + cap * 4;
    let new_heap = align_up(end, 4);
    (list_ptr, new_heap)
}

fn alloc_map(
    memory: &wasmtime::Memory,
    store: &mut wasmtime::Store<()>,
    heap_ptr: i32,
    entries: &[(i32, i32)],
) -> (i32, i32) {
    let map_ptr = align_up(heap_ptr, 4);
    let len = entries.len() as i32;
    let cap = if len > 0 { len } else { 1 };
    let entry_size = 8;
    let data_ptr = map_ptr + 16;

    write_i32(memory, store, map_ptr + 0, len);
    write_i32(memory, store, map_ptr + 4, cap);
    write_i32(memory, store, map_ptr + 8, 0);
    write_i32(memory, store, map_ptr + 12, data_ptr);

    for (idx, (key, val)) in entries.iter().enumerate() {
        let base = data_ptr + (idx as i32) * entry_size;
        write_i32(memory, store, base, *key);
        write_i32(memory, store, base + 4, *val);
    }

    let end = data_ptr + cap * entry_size;
    let new_heap = align_up(end, 4);
    (map_ptr, new_heap)
}

fn assert_pair_unordered(a: i32, b: i32, x: i32, y: i32) {
    assert!(
        (a == x && b == y) || (a == y && b == x),
        "expected pair {{ {}, {} }} got {{ {}, {} }}",
        x,
        y,
        a,
        b
    );
}

#[path = "collections_runtime/collection_guards.rs"]
mod collection_guards;
#[path = "collections_runtime/list_ops.rs"]
mod list_ops;
#[path = "collections_runtime/map_ops.rs"]
mod map_ops;
#[path = "collections_runtime/set_ops.rs"]
mod set_ops;
#[path = "collections_runtime/structural_eq.rs"]
mod structural_eq;
