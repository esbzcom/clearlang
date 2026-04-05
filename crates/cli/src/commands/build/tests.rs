#[cfg(test)]
mod tests {
    use super::*;
    use clg_ast::{Param, ParamKind};

    include!("tests/core_contracts.rs");
    include!("tests/determinism_release.rs");
    include!("tests/solver_backend.rs");
}
