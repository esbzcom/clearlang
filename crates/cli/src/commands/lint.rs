use std::fs;
use std::path::{Path, PathBuf};

use anyhow::Result;

use crate::commands::helpers::{make_single_json_error, CommandError};
use crate::logging::{Logger, StageTimings};

const LINT_ERROR_CODE: &str = "C133";

#[derive(Clone, Debug)]
struct LintWarning {
    rule: &'static str,
    message: &'static str,
    file: PathBuf,
    line: usize,
    column: usize,
}

pub fn run(path: PathBuf, deny_warnings: bool, json_errors: bool, logger: Logger) -> Result<()> {
    let mut timings = StageTimings::new();
    let files = {
        let _stage = timings.start(logger, "lint_discover");
        discover_clear_files(path.as_path(), json_errors)?
    };

    let mut warnings = Vec::new();
    {
        let _stage = timings.start(logger, "lint_scan");
        for file in &files {
            let source = fs::read_to_string(file).map_err(|err| {
                lint_error(
                    format!("reading `{}`: {err}", file.display()),
                    file,
                    json_errors,
                )
            })?;
            warnings.extend(scan_file(file, source.as_str()));
        }
    }

    warnings.sort_by(|a, b| {
        a.file
            .as_os_str()
            .cmp(b.file.as_os_str())
            .then_with(|| a.line.cmp(&b.line))
            .then_with(|| a.column.cmp(&b.column))
            .then_with(|| a.rule.cmp(b.rule))
    });

    logger.summary(&timings);

    if deny_warnings && !warnings.is_empty() {
        if !json_errors {
            emit_warnings(&warnings);
        }
        let first = &warnings[0];
        return Err(lint_error(
            format!(
                "lint failed: {} warning(s) found (--deny-warnings); first warning: [{}] {}:{}:{} {}",
                warnings.len(),
                first.rule,
                first.file.display(),
                first.line,
                first.column,
                first.message
            ),
            first.file.as_path(),
            json_errors,
        )
        .into());
    }

    emit_warnings(&warnings);
    println!(
        "lint ok: {} file(s), {} warning(s)",
        files.len(),
        warnings.len()
    );
    Ok(())
}

fn emit_warnings(warnings: &[LintWarning]) {
    for warning in warnings {
        eprintln!(
            "lint warning [{}] {}:{}:{} {}",
            warning.rule,
            warning.file.display(),
            warning.line,
            warning.column,
            warning.message
        );
    }
}

fn scan_file(file: &Path, source: &str) -> Vec<LintWarning> {
    let mut warnings = Vec::new();

    if let Some((line, column)) = first_crlf_position(source) {
        warnings.push(LintWarning {
            rule: "lint.crlf_line_endings",
            message: "CRLF line endings detected; run `clg fmt` to normalize",
            file: file.to_path_buf(),
            line,
            column,
        });
    }

    for (idx, line) in source.lines().enumerate() {
        if let Some(column) = trailing_whitespace_column(line) {
            warnings.push(LintWarning {
                rule: "lint.trailing_whitespace",
                message: "trailing whitespace detected",
                file: file.to_path_buf(),
                line: idx + 1,
                column,
            });
        }
    }

    if !source.is_empty() && !source.ends_with('\n') {
        let line = source.lines().count().max(1);
        let column = source
            .lines()
            .last()
            .map(|last| last.chars().count() + 1)
            .unwrap_or(1);
        warnings.push(LintWarning {
            rule: "lint.missing_final_newline",
            message: "missing trailing newline at end-of-file",
            file: file.to_path_buf(),
            line,
            column,
        });
    }

    warnings
}

fn trailing_whitespace_column(line: &str) -> Option<usize> {
    let trimmed = line.trim_end_matches([' ', '\t']);
    if trimmed.len() == line.len() {
        return None;
    }
    Some(trimmed.chars().count() + 1)
}

fn first_crlf_position(source: &str) -> Option<(usize, usize)> {
    source.find("\r\n").map(|idx| {
        let prefix = &source[..idx];
        let line = prefix.bytes().filter(|b| *b == b'\n').count() + 1;
        let col_prefix = prefix
            .rfind('\n')
            .map(|last_nl| &prefix[(last_nl + 1)..])
            .unwrap_or(prefix);
        let column = col_prefix.chars().count() + 1;
        (line, column)
    })
}

fn discover_clear_files(path: &Path, json_errors: bool) -> Result<Vec<PathBuf>> {
    if path.is_file() {
        if is_clear_source(path) {
            return Ok(vec![path.to_path_buf()]);
        }
        return Err(lint_error(
            format!("expected `.clear` source file, found `{}`", path.display()),
            path,
            json_errors,
        )
        .into());
    }
    if !path.is_dir() {
        return Err(lint_error(
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
        return Err(lint_error(
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
            lint_error(
                format!("reading `{}`: {err}", dir.display()),
                dir,
                json_errors,
            )
        })?
        .collect::<std::io::Result<Vec<_>>>()
        .map_err(|err| {
            lint_error(
                format!("reading `{}`: {err}", dir.display()),
                dir,
                json_errors,
            )
        })?;
    entries.sort_by(|a, b| a.path().as_os_str().cmp(b.path().as_os_str()));

    for entry in entries {
        let path = entry.path();
        let ty = entry.file_type().map_err(|err| {
            lint_error(
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

fn lint_error(message: String, file: &Path, json_errors: bool) -> CommandError {
    if json_errors {
        CommandError::json(make_single_json_error(
            LINT_ERROR_CODE,
            "lint",
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
    use super::{first_crlf_position, scan_file, trailing_whitespace_column};
    use std::path::Path;

    #[test]
    fn trailing_whitespace_column_reports_first_trailing_char() {
        assert_eq!(trailing_whitespace_column("abc  "), Some(4));
        assert_eq!(trailing_whitespace_column("abc\t"), Some(4));
        assert_eq!(trailing_whitespace_column("abc"), None);
    }

    #[test]
    fn first_crlf_position_reports_line_and_column() {
        assert_eq!(first_crlf_position("a\r\nb"), Some((1, 2)));
        assert_eq!(first_crlf_position("a\nbb\r\nc"), Some((2, 3)));
        assert_eq!(first_crlf_position("a\nb"), None);
    }

    #[test]
    fn scan_file_emits_expected_rules() {
        let file = Path::new("x.clear");
        let warnings = scan_file(file, "a  \r\nb");
        let rules: Vec<&str> = warnings.iter().map(|w| w.rule).collect();
        assert!(rules.contains(&"lint.trailing_whitespace"));
        assert!(rules.contains(&"lint.crlf_line_endings"));
        assert!(rules.contains(&"lint.missing_final_newline"));
    }
}
