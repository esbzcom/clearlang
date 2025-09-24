pub mod intrinsics {
    pub mod strings;
}
pub mod ir;
pub mod trivial;

pub use ir::{emit_from_ir, emit_from_ir_with_opts, CodegenOpts};
pub use trivial::emit_trivial_main;
