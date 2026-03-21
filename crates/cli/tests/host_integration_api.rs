use std::path::PathBuf;

use clg_cli::logging::Logger;

#[test]
fn host_integration_can_reference_run_command_api() {
    let _api: fn(PathBuf, String, bool, Logger) -> anyhow::Result<()> = clg_cli::commands::run::run;
}
