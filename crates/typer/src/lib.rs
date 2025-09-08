mod errors;
mod builtins;
mod check;
mod lower;

pub use errors::TyperError;
pub use check::check;

// Re-export internal modules for tests if needed
