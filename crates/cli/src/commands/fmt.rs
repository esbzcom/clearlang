use std::fs;
use std::path::{Path, PathBuf};

use anyhow::Result;

use crate::commands::helpers::{make_single_json_error, CommandError};
use crate::logging::{Logger, StageTimings};

const FORMAT_ERROR_CODE: &str = "C132";

pub fn run(path: PathBuf, check: bool, json_errors: bool, logger: Logger) -> Result<()> {
    let mut timings = StageTimings::new();
    let files = {
        let _stage = timings.start(logger, "fmt_discover");
        discover_clear_files(path.as_path(), json_errors)?
    };

    let mut changed = Vec::new();
    {
        let _stage = timings.start(logger, "fmt_apply");
        for file in &files {
            let source = fs::read_to_string(file).map_err(|err| {
                fmt_error(
                    format!("reading `{}`: {err}", file.display()),
                    file,
                    json_errors,
                )
            })?;
            let formatted = format_source(source.as_str());
            if source != formatted {
                if !check {
                    fs::write(file, formatted.as_bytes()).map_err(|err| {
                        fmt_error(
                            format!("writing `{}`: {err}", file.display()),
                            file,
                            json_errors,
                        )
                    })?;
                }
                changed.push(file.clone());
            }
        }
    }

    logger.summary(&timings);

    if check {
        if changed.is_empty() {
            println!("fmt ok: {} file(s) already formatted", files.len());
            return Ok(());
        }
        let first = &changed[0];
        return Err(fmt_error(
            format!(
                "format check failed: {} file(s) require formatting (first: `{}`)",
                changed.len(),
                first.display()
            ),
            first.as_path(),
            json_errors,
        )
        .into());
    }

    println!(
        "fmt ok: {} file(s) scanned, {} file(s) rewritten",
        files.len(),
        changed.len()
    );
    Ok(())
}

fn discover_clear_files(path: &Path, json_errors: bool) -> Result<Vec<PathBuf>> {
    if path.is_file() {
        if is_clear_source(path) {
            return Ok(vec![path.to_path_buf()]);
        }
        return Err(fmt_error(
            format!("expected `.clear` source file, found `{}`", path.display()),
            path,
            json_errors,
        )
        .into());
    }
    if !path.is_dir() {
        return Err(fmt_error(
            format!("path does not exist: `{}`", path.display()),
            path,
            json_errors,
        )
        .into());
    }

    let mut files = Vec::new();
    collect_clear_files(path, &mut files, json_errors)?;
    files.sort_by(|a, b| a.as_os_str().cmp(b.as_os_str()));
    if files.is_empty() {
        return Err(fmt_error(
            format!("no `.clear` files found under `{}`", path.display()),
            path,
            json_errors,
        )
        .into());
    }
    Ok(files)
}

fn collect_clear_files(dir: &Path, out: &mut Vec<PathBuf>, json_errors: bool) -> Result<()> {
    let mut entries = fs::read_dir(dir)
        .map_err(|err| {
            fmt_error(
                format!("reading `{}`: {err}", dir.display()),
                dir,
                json_errors,
            )
        })?
        .collect::<std::io::Result<Vec<_>>>()
        .map_err(|err| {
            fmt_error(
                format!("reading `{}`: {err}", dir.display()),
                dir,
                json_errors,
            )
        })?;
    entries.sort_by(|a, b| a.path().as_os_str().cmp(b.path().as_os_str()));

    for entry in entries {
        let path = entry.path();
        let ty = entry.file_type().map_err(|err| {
            fmt_error(
                format!("reading `{}`: {err}", path.display()),
                &path,
                json_errors,
            )
        })?;
        if ty.is_dir() {
            collect_clear_files(path.as_path(), out, json_errors)?;
        } else if ty.is_file() && is_clear_source(path.as_path()) {
            out.push(path);
        }
    }
    Ok(())
}

fn is_clear_source(path: &Path) -> bool {
    path.extension().and_then(|ext| ext.to_str()) == Some("clear")
}

fn format_source(source: &str) -> String {
    let normalized = source.replace("\r\n", "\n").replace('\r', "\n");
    let mut out = String::new();
    for line in normalized.lines() {
        out.push_str(line.trim_end_matches([' ', '\t']));
        out.push('\n');
    }
    if out.is_empty() {
        out.push('\n');
    }
    out
}

fn fmt_error(message: String, file: &Path, json_errors: bool) -> CommandError {
    if json_errors {
        CommandError::json(make_single_json_error(
            FORMAT_ERROR_CODE,
            "fmt",
            message,
            file,
            0,
            0,
            None,
        ))
    } else {
        CommandError::stderr(message)
    }
}

#[cfg(test)]
mod tests {
    use super::format_source;

    #[test]
    fn format_source_normalizes_line_endings_and_trailing_whitespace() {
        let input = "function main() -> Int { 0 }\r\n\t\r\nx  \t\r";
        let formatted = format_source(input);
        assert_eq!(formatted, "function main() -> Int { 0 }\n\nx\n");
    }

    #[test]
    fn format_source_preserves_existing_blank_lines() {
        let input = "a\n\n\n";
        let formatted = format_source(input);
        assert_eq!(formatted, "a\n\n\n");
    }

    #[test]
    fn format_source_empty_input_produces_single_newline() {
        let formatted = format_source("");
        assert_eq!(formatted, "\n");
    }
}
