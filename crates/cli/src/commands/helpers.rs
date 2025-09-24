use std::path::PathBuf;

use clg_parser::ParserError as ParserErr;
use serde::Serialize;

#[derive(Serialize)]
pub struct JsonError {
    pub ok: bool,
    pub errors: Vec<JsonErrorItem>,
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

pub fn emit_parse_structured_json_errors(file: &PathBuf, errs: &[ParserErr]) {
    let items: Vec<JsonErrorItem> = errs
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
    let out = JsonError {
        ok: false,
        errors: items,
    };
    println!("{}", serde_json::to_string_pretty(&out).unwrap());
}

pub fn emit_type_json_error(file: &PathBuf, err_pretty: &str) {
    let (code, start, end, func_opt) = classify_type_error(err_pretty);
    emit_single_json_error(code, "type", err_pretty.trim(), file, start, end, func_opt);
}

pub fn emit_single_json_error(
    code: &'static str,
    stage: &'static str,
    message: &str,
    file: &PathBuf,
    start: usize,
    end: usize,
    function: Option<String>,
) {
    let item = JsonErrorItem {
        code,
        stage,
        message: message.to_string(),
        file: file.display().to_string(),
        start,
        end,
        function,
    };
    let out = JsonError {
        ok: false,
        errors: vec![item],
    };
    println!("{}", serde_json::to_string_pretty(&out).unwrap());
}

fn extract_span(s: &str) -> Option<(usize, usize)> {
    if let Some(idx) = s.find("at ") {
        let rest = &s[idx + 3..];
        let mut parts = rest.split("..");
        if let (Some(a), Some(brest)) = (parts.next(), parts.next()) {
            let mut bchars = brest.chars();
            let mut num = String::new();
            for ch in bchars.by_ref() {
                if ch.is_ascii_digit() {
                    num.push(ch);
                } else {
                    break;
                }
            }
            if let (Ok(st), Ok(en)) = (a.trim().parse::<usize>(), num.parse::<usize>()) {
                return Some((st, en));
            }
        }
    }
    None
}

fn extract_function_name(s: &str) -> Option<String> {
    if let Some(idx) = s.find("in function `") {
        let rest = &s[idx + "in function `".len()..];
        if let Some(end) = rest.find('`') {
            return Some(rest[..end].to_string());
        }
    }
    None
}

fn classify_type_error(s: &str) -> (&'static str, usize, usize, Option<String>) {
    let code = if s.contains("unknown function") {
        "T001"
    } else if s.contains("arity mismatch") {
        "T002"
    } else if s.contains("return type mismatch") {
        "T004"
    } else if s.contains("type mismatch") {
        "T003"
    } else if s.contains("must be Int") {
        "T005"
    } else if s.contains("unknown variable") {
        "T006"
    } else {
        "T000"
    };
    let span = extract_span(s).unwrap_or((0, 0));
    let func = extract_function_name(s);
    (code, span.0, span.1, func)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    #[test]
    fn extract_span_parses_basic_pattern() {
        let s = "some error at 12..34: details here";
        assert_eq!(extract_span(s), Some((12, 34)));
    }

    #[test]
    fn classify_type_error_maps_codes_and_spans() {
        let s1 = "at 5..8: unknown function `foo`";
        let (c1, st1, en1, f1) = super::classify_type_error(s1);
        assert_eq!(c1, "T001");
        assert_eq!((st1, en1), (5, 8));
        assert!(f1.is_none());
    }

    #[test]
    fn json_single_error_shape() {
        let item = JsonErrorItem {
            code: "T003",
            stage: "type",
            message: "type mismatch".to_string(),
            file: "file.clear".to_string(),
            start: 10,
            end: 12,
            function: Some("main".to_string()),
        };
        let out = JsonError {
            ok: false,
            errors: vec![item],
        };
        let s = serde_json::to_string_pretty(&out).unwrap();
        let v: Value = serde_json::from_str(&s).unwrap();
        assert_eq!(v["ok"], Value::Bool(false));
        assert!(v["errors"].is_array());
        assert_eq!(v["errors"].as_array().unwrap().len(), 1);
        let e = &v["errors"][0];
        assert_eq!(e["code"], Value::String("T003".into()));
        assert_eq!(e["stage"], Value::String("type".into()));
        assert_eq!(e["message"], Value::String("type mismatch".into()));
        assert_eq!(e["file"], Value::String("file.clear".into()));
        assert_eq!(e["start"], Value::Number(10.into()));
        assert_eq!(e["end"], Value::Number(12.into()));
        assert_eq!(e["function"], Value::String("main".into()));
    }

    #[test]
    fn json_multiple_errors_shape() {
        let items = vec![
            JsonErrorItem {
                code: "P001",
                stage: "parse",
                message: "unexpected token".into(),
                file: "a.clear".into(),
                start: 1,
                end: 2,
                function: None,
            },
            JsonErrorItem {
                code: "T006",
                stage: "type",
                message: "unknown variable `x`".into(),
                file: "b.clear".into(),
                start: 5,
                end: 6,
                function: Some("foo".into()),
            },
        ];
        let out = JsonError {
            ok: false,
            errors: items,
        };
        let s = serde_json::to_string(&out).unwrap();
        let v: Value = serde_json::from_str(&s).unwrap();
        assert_eq!(v["ok"], Value::Bool(false));
        let arr = v["errors"].as_array().unwrap();
        assert_eq!(arr.len(), 2);
        assert_eq!(arr[0]["code"], Value::String("P001".into()));
        assert_eq!(arr[1]["code"], Value::String("T006".into()));
        assert_eq!(arr[1]["function"], Value::String("foo".into()));
    }
}
