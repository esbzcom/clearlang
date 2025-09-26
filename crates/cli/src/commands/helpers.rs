use std::path::Path;

use clg_parser::ParserError as ParserErr;
use serde::Serialize;

#[derive(Serialize)]
pub struct JsonError {
    pub ok: bool,
    pub errors: Vec<JsonErrorItem>,
}

impl JsonError {
    pub fn single(item: JsonErrorItem) -> Self {
        JsonError {
            ok: false,
            errors: vec![item],
        }
    }

    pub fn to_pretty_string(&self) -> String {
        serde_json::to_string_pretty(self).expect("serialize json error")
    }
}

#[derive(Serialize)]
pub struct JsonErrorItem {
    pub code: &'static str,
    pub stage: &'static str,
    pub message: String,
    pub file: String,
    pub start: usize,
    pub end: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub function: Option<String>,
}

pub fn make_parse_json_error(file: &Path, errs: &[ParserErr]) -> JsonError {
    let items = errs
        .iter()
        .map(|e| JsonErrorItem {
            code: e.code,
            stage: "parse",
            message: e.message.clone(),
            file: file.display().to_string(),
            start: e.start,
            end: e.end,
            function: None,
        })
        .collect();
    JsonError {
        ok: false,
        errors: items,
    }
}

pub fn make_single_json_error(
    code: &'static str,
    stage: &'static str,
    message: impl Into<String>,
    file: &Path,
    start: usize,
    end: usize,
    function: Option<String>,
) -> JsonError {
    JsonError::single(JsonErrorItem {
        code,
        stage,
        message: message.into(),
        file: file.display().to_string(),
        start,
        end,
        function,
    })
}

pub fn extract_function_name(s: &str) -> Option<String> {
    if let Some(idx) = s.find("in function `") {
        let rest = &s[idx + "in function `".len()..];
        if let Some(end) = rest.find('`') {
            return Some(rest[..end].to_string());
        }
    }
    None
}

#[derive(Debug)]
pub struct CommandError {
    exit_code: i32,
    stdout: Option<String>,
    stderr: Option<String>,
}

impl CommandError {
    pub fn json(json: JsonError) -> Self {
        CommandError {
            exit_code: 1,
            stdout: Some(json.to_pretty_string()),
            stderr: None,
        }
    }

    pub fn exit_code(&self) -> i32 {
        self.exit_code
    }

    pub fn emit(&self) {
        if let Some(ref out) = self.stdout {
            println!("{out}");
        }
        if let Some(ref err) = self.stderr {
            eprintln!("{err}");
        }
    }
}

impl std::fmt::Display for CommandError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(err) = &self.stderr {
            write!(f, "{err}")
        } else if let Some(out) = &self.stdout {
            write!(f, "{out}")
        } else {
            write!(f, "command failed with exit code {}", self.exit_code)
        }
    }
}

impl std::error::Error for CommandError {}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;
    use std::path::PathBuf;

    #[test]
    fn extract_function_name_finds_function_context() {
        let msg = "type error in function `foo` at 1..3";
        assert_eq!(extract_function_name(msg).as_deref(), Some("foo"));
    }

    #[test]
    fn json_single_error_shape() {
        let file = PathBuf::from("file.clear");
        let json = make_single_json_error(
            "T003",
            "type",
            "type mismatch",
            &file,
            10,
            12,
            Some("main".to_string()),
        );
        let v: Value = serde_json::from_str(&json.to_pretty_string()).unwrap();
        assert_eq!(v["ok"], Value::Bool(false));
        let errors = v["errors"].as_array().unwrap();
        assert_eq!(errors.len(), 1);
        let e0 = &errors[0];
        assert_eq!(e0["code"], Value::String("T003".into()));
        assert_eq!(e0["stage"], Value::String("type".into()));
        assert_eq!(e0["message"], Value::String("type mismatch".into()));
        assert_eq!(e0["file"], Value::String("file.clear".into()));
        assert_eq!(e0["start"], Value::Number(10.into()));
        assert_eq!(e0["end"], Value::Number(12.into()));
        assert_eq!(e0["function"], Value::String("main".into()));
    }

    #[test]
    fn json_multiple_errors_shape() {
        let file = PathBuf::from("a.clear");
        let errs = vec![
            ParserErr {
                code: "P001",
                message: "unexpected token".into(),
                start: 1,
                end: 2,
            },
            ParserErr {
                code: "P001",
                message: "another error".into(),
                start: 3,
                end: 4,
            },
        ];
        let json = make_parse_json_error(&file, &errs);
        let v: Value = serde_json::from_str(&json.to_pretty_string()).unwrap();
        assert_eq!(v["ok"], Value::Bool(false));
        let arr = v["errors"].as_array().unwrap();
        assert_eq!(arr.len(), 2);
        assert_eq!(arr[0]["code"], Value::String("P001".into()));
        assert_eq!(arr[0]["stage"], Value::String("parse".into()));
    }
}
