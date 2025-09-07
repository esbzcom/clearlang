#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span { pub start: usize, pub end: usize }

#[derive(Debug, Clone)]
pub struct Program {
    pub funcs: Vec<Func>,
}

#[derive(Debug, Clone)]
pub struct Func {
    pub effect: Effect,
    pub name: String,
    pub params: Vec<Param>,
    pub ret: Type,
    pub body: Expr, // single-expression body for Phase 2
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Effect { Pure, Mut, Io, None }

#[derive(Debug, Clone)]
pub struct Param {
    pub name: String,
    pub ty: Type,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Type { Int, Bool, Str }

#[derive(Debug, Clone)]
pub enum Expr {
    Int(i64, Span),
    Bool(bool, Span),
    Str(String, Span),
    Var(String, Span),
    Bin { op: BinOp, lhs: Box<Expr>, rhs: Box<Expr>, span: Span },
    Call { callee: String, args: Vec<Expr>, span: Span },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinOp { Add, Sub, Mul, Div }
