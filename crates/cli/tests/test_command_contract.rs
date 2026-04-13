use assert_cmd::prelude::*;
use serde_json::Value;
use std::fs;
use std::path::Path;
use std::process::Command;
use tempfile::tempdir;

fn write_test_file(path: &Path, body: &str) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("create parent");
    }
    fs::write(path, body).expect("write test source");
}

fn parse_json_lines(lines: &str) -> Vec<Value> {
    lines
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| serde_json::from_str::<Value>(line).expect("json line"))
        .collect()
}

#[cfg(unix)]
fn create_dir_link(link: &Path, target: &Path) {
    std::os::unix::fs::symlink(target, link).expect("create symlink dir");
}

#[cfg(windows)]
fn create_dir_link(link: &Path, target: &Path) {
    let link_str = link.to_string_lossy().into_owned();
    let target_str = target.to_string_lossy().into_owned();
    let output = Command::new("cmd")
        .args(["/C", "mklink", "/J", link_str.as_str(), target_str.as_str()])
        .output()
        .expect("create junction");
    assert!(
        output.status.success(),
        "mklink /J failed: stdout=`{}` stderr=`{}`",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}


include!("test_command_contract/discovery_and_plan.rs");
include!("test_command_contract/execution_and_reporting.rs");
include!("test_command_contract/runtime_failures.rs");
include!("test_command_contract/mocks_and_bindings.rs");
