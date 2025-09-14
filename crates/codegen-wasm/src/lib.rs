use anyhow::Result;

pub mod intrinsics { pub mod strings; }
pub mod ir;
pub mod trivial;

pub use ir::{emit_from_ir_with_opts, emit_from_ir, CodegenOpts};
pub use trivial::emit_trivial_main;

