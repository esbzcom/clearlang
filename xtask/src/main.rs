use std::env;
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

fn main() -> Result<(), String> {
    let mut args = env::args().skip(1);
    let cmd = args.next().unwrap_or_else(|| "help".to_string());
    let root = repo_root();

    match cmd.as_str() {
        "fmt" => cargo_cmd(&root, &["fmt", "--all"])?,
        "clippy" => cargo_cmd(
            &root,
            &[
                "clippy",
                "--workspace",
                "--all-targets",
                "--",
                "-D",
                "warnings",
            ],
        )?,
        "test" => cargo_cmd(&root, &["test", "--workspace"])?,
        "validate" => validate_samples(&root)?,
        "emit-vcs" => emit_vcs_sample(&root)?,
        "ci" => {
            cargo_cmd(&root, &["fmt", "--all", "--", "--check"])?;
            cargo_cmd(
                &root,
                &[
                    "clippy",
                    "--workspace",
                    "--all-targets",
                    "--",
                    "-D",
                    "warnings",
                ],
            )?;
            cargo_cmd(&root, &["test", "--workspace"])?;
            cargo_cmd(&root, &["build", "--release", "-p", "clg-cli"])?;
            validate_samples(&root)?;
        }
        "help" | "-h" | "--help" => {
            print_help();
        }
        other => return Err(format!("unknown xtask command: {other}")),
    }

    Ok(())
}

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("xtask in repo root/xtask")
        .to_path_buf()
}

fn cargo_cmd(root: &Path, args: &[&str]) -> Result<(), String> {
    run(Command::new("cargo").args(args).current_dir(root))
}

fn validate_samples(root: &Path) -> Result<(), String> {
    let tmp = root.join("tmp").join("ci");
    fs::create_dir_all(&tmp).map_err(|e| format!("create tmp dir: {e}"))?;
    let samples = [
        "01_hello.clear",
        "02_arith.clear",
        "03_nested_calls.clear",
        "04_multiline_call.clear",
        "05_trailing_param_comma.clear",
        "08_main_const.clear",
        "18_return_simple.clear",
    ];
    for sample in samples {
        let out = tmp.join(sample.replace(".clear", ".wasm"));
        let sample_path = root.join("clearlang-tests").join(sample);
        run(Command::new("cargo")
            .arg("run")
            .arg("-p")
            .arg("clg-cli")
            .arg("--")
            .arg("build")
            .arg(&sample_path)
            .arg("-o")
            .arg(&out)
            .current_dir(root))?;
        run(Command::new("wasm-tools")
            .arg("validate")
            .arg(&out)
            .current_dir(root))?;
    }
    run(Command::new("cargo")
        .arg("run")
        .arg("-p")
        .arg("clg-cli")
        .arg("--")
        .arg("emit-hello")
        .arg("-o")
        .arg(tmp.join("emit_hello.wasm"))
        .current_dir(root))?;
    run(Command::new("wasm-tools")
        .arg("validate")
        .arg(tmp.join("emit_hello.wasm"))
        .current_dir(root))?;
    Ok(())
}

fn emit_vcs_sample(root: &Path) -> Result<(), String> {
    let tmp = root.join("tmp").join("xtask");
    fs::create_dir_all(&tmp).map_err(|e| format!("create tmp dir: {e}"))?;
    let src = r#"
        pure function inc(x: Int) -> Int
            require { x >= 0 }
            ensure { result > x }
        { x + 1 }
        function main() -> Int { inc(1) }
    "#;
    let src_path = tmp.join("contract.clear");
    let wasm_path = tmp.join("contract.wasm");
    let vcs_path = tmp.join("contract.vc.json");
    fs::write(&src_path, src).map_err(|e| format!("write source: {e}"))?;
    run(Command::new("cargo")
        .arg("run")
        .arg("-p")
        .arg("clg-cli")
        .arg("--")
        .arg("build")
        .arg(&src_path)
        .arg("-o")
        .arg(&wasm_path)
        .arg("--emit-vcs")
        .arg(&vcs_path)
        .current_dir(root))?;
    Ok(())
}

fn run(cmd: &mut Command) -> Result<(), String> {
    let program = cmd.get_program().to_os_string();
    let args = cmd
        .get_args()
        .map(|arg| arg.to_os_string())
        .collect::<Vec<_>>();
    let display = display_cmd(&program, &args);
    let status = cmd
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()
        .map_err(|e| format!("failed to run {display}: {e}"))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!(
            "command {display} failed with status {:?}",
            status.code()
        ))
    }
}

fn display_cmd(program: &OsStr, args: &[std::ffi::OsString]) -> String {
    let mut line = program.to_string_lossy().to_string();
    for arg in args {
        line.push(' ');
        line.push_str(&arg.to_string_lossy());
    }
    line
}

fn print_help() {
    println!("xtask commands:");
    println!("  fmt        - cargo fmt --all");
    println!("  clippy     - cargo clippy --workspace --all-targets -- -D warnings");
    println!("  test       - cargo test --workspace");
    println!("  validate   - build samples and run wasm-tools validate");
    println!("  emit-vcs   - build a small contract sample with --emit-vcs");
    println!("  ci         - fmt + clippy + test + validate");
}
