pub mod intrinsics {
    pub mod crypto;
    pub mod env;
    pub mod runtime;
    pub mod strings;
    pub mod u64;
    pub mod wasi;
}
pub mod ir;
pub mod trivial;

pub use ir::{
    emit_from_ir, emit_from_ir_with_opts, CodegenOpts, ExportAlias, ExternalImport, StdCoreLinkMode,
};
pub use trivial::emit_trivial_main;
