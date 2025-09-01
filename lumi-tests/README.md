Lumi sample programs (Phase 1/2)

How to try them with the CLI parser:

- Example: `cargo run -p lumi-cli -- parse lumi-tests/01_hello.lumi`
- Explore others by changing the filename accordingly.

Quick Start

- Parse AST (01_hello.lumi): `cargo run -p lumi-cli -- parse lumi-tests/01_hello.lumi`
- Emit hello Wasm (Phase 1 demo): `cargo run -p lumi-cli -- emit-hello -o tmp/hello.wasm`
- Run hello with Wasmtime: `wasmtime --invoke main tmp/hello.wasm`
  - Validate: `wasm-tools validate tmp/hello.wasm`

Notes
- The parser currently supports: functions, Int/Bool types, integer and boolean literals, variables, function calls, and binary ops `+ - * /` with standard precedence. Trailing commas are allowed in parameter lists, but not in call argument lists.
- File `06_trailing_call_comma.lumi` is an intentional parse error example.

Test Files
- `01_hello.lumi`: define `add2`, call it in `main`.
- `02_arith.lumi`: arithmetic with grouping and precedence.
- `03_nested_calls.lumi`: nested function calls (`add(mul(...))`).
- `04_multiline_call.lumi`: multi-line call formatting with parentheses.
- `05_trailing_param_comma.lumi`: trailing comma in parameter list (allowed).
- `06_trailing_call_comma.lumi`: trailing comma in call args (should fail to parse).
- `07_bools.lumi`: boolean literals and pure functions returning Bool.
- `08_main_const.lumi`: minimal program that can be compiled to Wasm (constant main).

Build From Source (const-eval subset)

- Build and run a few samples (Int-returning only):
  - `cargo run -p lumi-cli -- build lumi-tests/01_hello.lumi -o tmp/hello_prog.wasm`
  - `cargo run -p lumi-cli -- build lumi-tests/02_arith.lumi -o tmp/arith.wasm`
  - `cargo run -p lumi-cli -- build lumi-tests/03_nested_calls.lumi -o tmp/nested.wasm`
  - Run: `wasmtime --invoke main tmp/arith.wasm`
  - Validate: `wasm-tools validate tmp/arith.wasm`
  - Note: `07_bools.lumi` is not supported by const-eval codegen yet (non-Int return).

Safety Notes

- Compile time: parser + (upcoming) typer/contracts reject unsafe programs.
- Load time: always validate generated Wasm (`wasm-tools validate tmp/*.wasm`).
- Runtime: contract checks (when enabled) trap on violation; Wasmtime sandboxing + resource limits recommended.

Test End-to-End (parser → codegen → run)

- Run integration test: `cargo test -p lumi-codegen-wasm --test full_pipeline`

Run With Node.js (hello example)

```
node -e "const fs=require('fs');(async()=>{const m=await WebAssembly.instantiate(fs.readFileSync('tmp/hello.wasm')); console.log(m.instance.exports.main());})();"
```

- Expected output: `42`

Validate/Inspect (hello example)

- Validate: `wasm-tools validate tmp/hello.wasm`
- Inspect: `wasm-tools print tmp/hello.wasm` or `wasm-objdump -x tmp/hello.wasm`
- Wasmer (optional): `wasmer run --invoke main tmp/hello.wasm` (prints `42`).
