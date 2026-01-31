#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

#[derive(Debug, Clone)]
pub struct RefinedAlias {
    pub name: String,
    pub name_span: Span,
    pub type_params: Vec<String>,
    pub base: Type,
    pub binder: Option<String>,
    pub predicate: Expr,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct Program {
    pub refined_aliases: Vec<RefinedAlias>,
    pub resources: Vec<Resource>,
    pub structs: Vec<StructDecl>,
    pub enums: Vec<EnumDecl>,
    pub funcs: Vec<Func>,
}

#[derive(Debug, Clone)]
pub struct StructDecl {
    pub name: String,
    pub name_span: Span,
    pub fields: Vec<StructField>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct StructField {
    pub name: String,
    pub ty: Type,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct EnumDecl {
    pub name: String,
    pub name_span: Span,
    pub variants: Vec<EnumVariant>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct EnumVariant {
    pub name: String,
    pub name_span: Span,
    pub fields: Vec<Type>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct Resource {
    pub name: String,
    pub name_span: Span,
    pub fields: Vec<ResourceField>,
    pub drop_block: Block,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct ResourceField {
    pub name: String,
    pub ty: Type,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct Block {
    pub statements: Vec<Stmt>,
    pub tail: Option<Box<Expr>>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub enum Stmt {
    Let {
        name: String,
        expr: Box<Expr>,
        span: Span,
    },
    Expr {
        expr: Box<Expr>,
        span: Span,
    },
    While {
        cond: Box<Expr>,
        invariant: Box<Expr>,
        variant: Option<Box<Expr>>,
        body: Box<Block>,
        span: Span,
    },
}

#[derive(Debug, Clone)]
pub struct Func {
    pub effect: Effect,
    pub effect_span: Option<Span>,
    pub name: String,
    pub params: Vec<Param>,
    pub ret: Type,
    pub requires: Vec<Contract>,
    pub ensures: Vec<Contract>,
    pub body: Expr, // single-expression body for Phase 2
}

#[derive(Debug, Clone)]
pub struct Contract {
    pub span: Span,
    pub expr: Expr,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Effect {
    Pure,
    Mut,
    Io,
    None,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParamKind {
    Borrow,
    Consume,
}

#[derive(Debug, Clone)]
pub struct Param {
    pub kind: ParamKind,
    pub name: String,
    pub ty: Type,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Type {
    Int,
    U8,
    U64,
    U128,
    U256,
    Bool,
    String,
    Bytes,
    Resource(String),
    Option(Box<Type>),
    Result(Box<Type>, Box<Type>),
    List(Box<Type>),
    Set(Box<Type>),
    Map(Box<Type>, Box<Type>),
    Array(Box<Type>, u32),
    Tuple(Vec<Type>),
}

#[derive(Debug, Clone)]
pub enum Expr {
    Int(i64, Span),
    Bool(bool, Span),
    String(String, Span),
    Var(String, Span),
    ArrayLit {
        elems: Vec<Expr>,
        span: Span,
    },
    TupleLit {
        elems: Vec<Expr>,
        span: Span,
    },
    StructLit {
        name: String,
        fields: Vec<StructFieldInit>,
        span: Span,
    },
    FieldAccess {
        base: Box<Expr>,
        field: String,
        span: Span,
    },
    Block {
        block: Box<Block>,
    },
    Bin {
        op: BinOp,
        lhs: Box<Expr>,
        rhs: Box<Expr>,
        span: Span,
    },
    Call {
        callee: String,
        args: Vec<Expr>,
        span: Span,
    },
    Return {
        expr: Box<Expr>,
        span: Span,
    },
    Unary {
        op: UnaryOp,
        expr: Box<Expr>,
        span: Span,
    },
    Match {
        scrutinee: Box<Expr>,
        arms: Vec<MatchArm>,
        span: Span,
    },
    If {
        cond: Box<Expr>,
        then_br: Box<Expr>,
        else_br: Box<Expr>,
        span: Span,
    },
    Index {
        base: Box<Expr>,
        index: Box<Expr>,
        span: Span,
    },
    Try {
        expr: Box<Expr>,
        span: Span,
    },
}

#[derive(Debug, Clone)]
pub struct StructFieldInit {
    pub name: String,
    pub expr: Expr,
    pub span: Span,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    Shl,
    Shr,
    BitAnd,
    BitXor,
    BitOr,
    Lt,
    Le,
    Gt,
    Ge,
    Eq,
    Neq,
    And,
    Or,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnaryOp {
    Not,
}

#[derive(Debug, Clone)]
pub enum MatchPat {
    Some(String),
    None,
    Ok(String),
    Err(String),
}

#[derive(Debug, Clone)]
pub struct MatchArm {
    pub pat: MatchPat,
    pub expr: Expr,
}
