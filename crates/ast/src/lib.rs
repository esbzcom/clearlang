#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

#[derive(Debug, Clone)]
pub struct RefinedAlias {
    pub is_exported: bool,
    pub name: String,
    pub name_span: Span,
    pub type_params: Vec<String>,
    pub base: Type,
    pub binder: Option<String>,
    pub predicate: Expr,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct ModuleDecl {
    pub path: Vec<String>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct ImportItem {
    pub name: String,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub enum ImportKind {
    Module { alias: Option<String> },
    Items { items: Vec<ImportItem> },
}

#[derive(Debug, Clone)]
pub struct ImportDecl {
    pub path: Vec<String>,
    pub path_span: Span,
    pub kind: ImportKind,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct Program {
    pub module: Option<ModuleDecl>,
    pub imports: Vec<ImportDecl>,
    pub contracts: Vec<ContractDecl>,
    pub refined_aliases: Vec<RefinedAlias>,
    pub resources: Vec<Resource>,
    pub structs: Vec<StructDecl>,
    pub enums: Vec<EnumDecl>,
    pub traits: Vec<TraitDecl>,
    pub impls: Vec<ImplDecl>,
    pub funcs: Vec<Func>,
}

#[derive(Debug, Clone)]
pub struct ContractDecl {
    pub name: String,
    pub name_span: Span,
    pub version: u32,
    pub fields: Vec<StructField>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct TypeParam {
    pub name: String,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct TraitBound {
    pub param: String,
    pub trait_name: String,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct StructDecl {
    pub is_exported: bool,
    pub is_resource: bool,
    pub name: String,
    pub name_span: Span,
    pub type_params: Vec<TypeParam>,
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
    pub is_exported: bool,
    pub is_resource: bool,
    pub name: String,
    pub name_span: Span,
    pub type_params: Vec<TypeParam>,
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
    pub is_exported: bool,
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
    pub is_exported: bool,
    pub effect: Effect,
    pub effect_span: Option<Span>,
    pub name: String,
    pub type_params: Vec<TypeParam>,
    pub params: Vec<Param>,
    pub ret: Type,
    pub where_bounds: Vec<TraitBound>,
    pub requires: Vec<Contract>,
    pub ensures: Vec<Contract>,
    pub body: Expr, // single-expression body for Phase 2
}

#[derive(Debug, Clone)]
pub struct TraitMethod {
    pub effect: Effect,
    pub effect_span: Option<Span>,
    pub name: String,
    pub params: Vec<Param>,
    pub ret: Type,
    pub default_body: Option<Expr>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct TraitDecl {
    pub is_exported: bool,
    pub name: String,
    pub name_span: Span,
    pub type_params: Vec<TypeParam>,
    pub methods: Vec<TraitMethod>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct ImplDecl {
    pub trait_name: String,
    pub trait_name_span: Span,
    pub type_params: Vec<TypeParam>,
    pub for_type: Type,
    pub where_bounds: Vec<TraitBound>,
    pub methods: Vec<Func>,
    pub span: Span,
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

#[derive(Debug, Clone)]
pub struct LambdaParam {
    pub name: String,
    pub ty: Type,
    pub span: Span,
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
    Named { name: String, args: Vec<Type> },
    Option(Box<Type>),
    Result(Box<Type>, Box<Type>),
    List(Box<Type>),
    Set(Box<Type>),
    Map(Box<Type>, Box<Type>),
    Array(Box<Type>, Option<u32>),
    Slice(Box<Type>),
    Tuple(Vec<Type>),
    Fn { params: Vec<Type>, ret: Box<Type> },
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
        type_args: Vec<Type>,
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
    Lambda {
        params: Vec<LambdaParam>,
        body: Box<Expr>,
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
    Wildcard,
    EnumVariant {
        enum_name: String,
        variant: String,
        binders: Vec<String>,
    },
}

#[derive(Debug, Clone)]
pub struct MatchArm {
    pub pat: MatchPat,
    pub expr: Expr,
}
