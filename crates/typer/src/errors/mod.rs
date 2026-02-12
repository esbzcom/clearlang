mod collections;
mod control;
mod core;
mod effects;
mod generics;
mod match_errors;
mod refinements;
mod resources;
mod traits;
mod types;
mod unsigned;

use clg_ast::Type;

#[derive(Debug, Clone)]
pub struct TyperError {
    pub code: &'static str,
    pub message: String,
    pub start: usize,
    pub end: usize,
}

impl TyperError {
    pub fn new(code: &'static str, message: String, start: usize, end: usize) -> Self {
        TyperError {
            code,
            message,
            start,
            end,
        }
    }
}

impl std::fmt::Display for TyperError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.code, self.message)
    }
}

impl std::error::Error for TyperError {}

pub(super) fn render_type(ty: &Type) -> String {
    match ty {
        Type::Int => "Int".to_string(),
        Type::U8 => "U8".to_string(),
        Type::U64 => "U64".to_string(),
        Type::U128 => "U128".to_string(),
        Type::U256 => "U256".to_string(),
        Type::Bool => "Bool".to_string(),
        Type::String => "String".to_string(),
        Type::Bytes => "Bytes".to_string(),
        Type::Named { name, args } => {
            if args.is_empty() {
                name.to_string()
            } else {
                let rendered = args.iter().map(render_type).collect::<Vec<_>>().join(", ");
                format!("{}<{}>", name, rendered)
            }
        }
        Type::Option(inner) => format!("Option<{}>", render_type(inner)),
        Type::Result(ok, err) => format!("Result<{}, {}>", render_type(ok), render_type(err)),
        Type::List(inner) => format!("List<{}>", render_type(inner)),
        Type::Set(inner) => format!("Set<{}>", render_type(inner)),
        Type::Map(key, val) => format!("Map<{}, {}>", render_type(key), render_type(val)),
        Type::Array(inner, Some(len)) => format!("[{}; {}]", render_type(inner), len),
        Type::Array(inner, None) => format!("Array<{}>", render_type(inner)),
        Type::Slice(inner) => format!("Slice<{}>", render_type(inner)),
        Type::Tuple(elements) => {
            let rendered = elements
                .iter()
                .map(|elem| render_type(elem))
                .collect::<Vec<_>>()
                .join(", ");
            format!("({})", rendered)
        }
        Type::Fn { params, ret } => {
            let rendered = params
                .iter()
                .map(render_type)
                .collect::<Vec<_>>()
                .join(", ");
            format!("function({}) -> {}", rendered, render_type(ret))
        }
    }
}
