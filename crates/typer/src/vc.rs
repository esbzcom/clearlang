mod generate;
mod linear;
mod loops;
mod refinements;
mod smt;
mod source;

pub use generate::{generate_vcs, generate_vcs_with_dependencies, AssumptionDependencies};

use clg_ast::{Expr, Span, Type};
use smt::SmtEncoder;
use source::expr_to_source;

#[derive(Debug, Clone)]
pub struct ContractExpr {
    pub ast: String,
    pub smt2: String,
    pub span: Option<Span>,
}

#[derive(Debug, Clone)]
pub struct ExprSnapshot {
    pub ast: String,
    pub smt2: String,
}

#[derive(Debug, Clone)]
pub enum RefinementAttachmentKind {
    Param,
    Return,
    Flow,
}

#[derive(Debug, Clone)]
pub enum RefinementAttachmentDetail {
    Param { param: String },
    Return { result: String },
    Flow(RefinementFlowDetail),
}

#[derive(Debug, Clone)]
pub struct RefinementAttachment {
    pub kind: RefinementAttachmentKind,
    pub detail: RefinementAttachmentDetail,
}

#[derive(Debug, Clone)]
pub enum RefinementFlowKind {
    Let,
    CallArg,
    MatchBinder,
}

impl RefinementFlowKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            RefinementFlowKind::Let => "let",
            RefinementFlowKind::CallArg => "call_arg",
            RefinementFlowKind::MatchBinder => "match_binder",
        }
    }
}

#[derive(Debug, Clone)]
pub struct RefinementFlowDetail {
    pub flow_kind: RefinementFlowKind,
    pub name: Option<String>,
    pub callee: Option<String>,
    pub arg_index: Option<usize>,
    pub variant: Option<String>,
    pub arm: Option<usize>,
}

impl RefinementFlowDetail {
    fn new(flow_kind: RefinementFlowKind) -> Self {
        Self {
            flow_kind,
            name: None,
            callee: None,
            arg_index: None,
            variant: None,
            arm: None,
        }
    }
}

impl RefinementAttachment {
    fn param(name: String) -> Self {
        Self {
            kind: RefinementAttachmentKind::Param,
            detail: RefinementAttachmentDetail::Param { param: name },
        }
    }

    fn result(name: String) -> Self {
        Self {
            kind: RefinementAttachmentKind::Return,
            detail: RefinementAttachmentDetail::Return { result: name },
        }
    }

    fn flow(detail: RefinementFlowDetail) -> Self {
        Self {
            kind: RefinementAttachmentKind::Flow,
            detail: RefinementAttachmentDetail::Flow(detail),
        }
    }
}

#[derive(Debug, Clone)]
pub struct RefinementPremise {
    pub alias: String,
    pub binder: String,
    pub substitution: ExprSnapshot,
    pub predicate: ExprSnapshot,
    pub attachment: RefinementAttachment,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssumptionCategory {
    Unsigned,
    Bitwise,
    Crypto,
    Primitive,
    External,
}

impl AssumptionCategory {
    pub fn as_str(&self) -> &'static str {
        match self {
            AssumptionCategory::Unsigned => "unsigned",
            AssumptionCategory::Bitwise => "bitwise",
            AssumptionCategory::Crypto => "crypto",
            AssumptionCategory::Primitive => "primitive",
            AssumptionCategory::External => "external",
        }
    }
}

#[derive(Debug, Clone)]
pub struct AssumptionBoundary {
    pub id: &'static str,
    pub category: AssumptionCategory,
    pub status: &'static str,
    pub message: &'static str,
    pub symbols: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct VerificationCondition {
    pub function: String,
    pub vc_id: String,
    pub pre: ContractExpr,
    pub post: ContractExpr,
    pub vc_smt2: String,
    pub status: &'static str,
    pub refinements: Vec<RefinementPremise>,
    pub assumptions: Vec<AssumptionBoundary>,
}

#[derive(Debug)]
struct LoopObligation<'a> {
    invariant: &'a Expr,
    variant: Option<&'a Expr>,
}

#[derive(Clone)]
struct RefinementObligation {
    alias: String,
    binder: String,
    substitution: Expr,
    substitution_type: Type,
    predicate: Expr,
    attachment: RefinementAttachment,
}

fn snapshot_expr(expr: &Expr) -> ExprSnapshot {
    let ast = expr_to_source(expr, 0);
    let mut encoder = SmtEncoder::default();
    let smt2 = encoder.encode(expr);
    ExprSnapshot { ast, smt2 }
}
