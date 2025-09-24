#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlaceHolder;

// Phase 3.3 — Minimal IR shape (SSA‑like, single block per function for now)

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IrType {
    Int,  // corresponds to ClearLang Type::Int (lowered to i32 for now)
    Bool, // corresponds to ClearLang Type::Bool (lowered to i32 0/1 for now)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Value(pub u32); // SSA value id

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinOpIR {
    Add,
    Sub,
    Mul,
    Div,
}

#[derive(Debug, Clone)]
pub enum Instr {
    // v = const n
    IConst {
        dst: Value,
        ty: IrType,
        n: i64,
    },
    // v = address of string literal (allocated in data segment at codegen)
    IStringConst {
        dst: Value,
        s: String,
    },
    // v = bin lhs op rhs
    IBin {
        dst: Value,
        op: BinOpIR,
        lhs: Value,
        rhs: Value,
    },
    // v = if cond then then_v else else_v (expression form)
    ISelect {
        dst: Value,
        cond: Value,
        then_v: Value,
        else_v: Value,
    },
    // v? = call callee_idx(args) — callee is a function index in the module
    Call {
        dst: Option<Value>,
        callee: u32,
        args: Vec<Value>,
    },
    // return v
    Ret {
        val: Value,
    },
}

#[derive(Debug, Clone)]
pub struct Function {
    pub name: String,
    pub params: Vec<IrType>,
    pub ret: Option<IrType>,
    pub body: Vec<Instr>, // single block for Phase 3.x
}

#[derive(Debug, Clone, Default)]
pub struct Module {
    pub funcs: Vec<Function>,
}
