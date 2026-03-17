# Developer Workflow

## Requirements

- Rust toolchain with `rustfmt` and `clippy` components.
- `wasm-tools` in PATH for validation (`cargo install wasm-tools --locked`).

## Quick Commands

This repo provides a small `xtask` helper with common targets. Use the alias:

```bash
cargo xtask fmt
cargo xtask clippy
cargo xtask test
cargo xtask validate
cargo xtask emit-vcs
cargo xtask std-core-artifact
cargo xtask std-surface-drift-check
cargo xtask ci
```

If the alias is not available, run:

```bash
cargo run -p xtask -- <command>
```

## Notes

- For CLI stage logs, pass `-v`/`-vv` or set `CLG_LOG=debug`/`trace`.
- `validate` and `ci` expect `wasm-tools` to be installed.
- Microbenchmarks: `cargo bench -p clg-parser`, `cargo bench -p clg-typer`, `cargo bench -p clg-codegen-wasm`.
