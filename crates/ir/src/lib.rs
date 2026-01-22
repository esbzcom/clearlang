#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlaceHolder;

// Phase 3.3 - Minimal IR shape (SSA-like, single block per function for now)

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IrType {
    Int,  // corresponds to ClearLang Type::Int (lowered to i32 for now)
    Bool, // corresponds to ClearLang Type::Bool (lowered to i32 0/1 for now)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Value(pub u32); // SSA value id

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VariantParts {
    pub tag: Value,
    pub payload_lo: Value,
    pub payload_hi: Value,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VariantKind {
    Option,
    Result,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinOpIR {
    Add,
    Sub,
    Mul,
    Div,
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
pub enum TrapCode {
    ContractViolation,
    AllocatorOom,
    InvalidUtf8,
    InvalidVariantTag,
    LimitsExceeded,
}

impl VariantKind {
    pub fn as_i32(self) -> i32 {
        match self {
            VariantKind::Option => 0,
            VariantKind::Result => 1,
        }
    }
}

impl TrapCode {
    pub fn as_i32(self) -> i32 {
        match self {
            TrapCode::ContractViolation => 1,
            TrapCode::AllocatorOom => 2,
            TrapCode::InvalidUtf8 => 3,
            TrapCode::InvalidVariantTag => 4,
            TrapCode::LimitsExceeded => 5,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GuardKind {
    Require,
    Ensure,
    LoopInvariant,
    LoopVariant,
    LoopVariantProgress,
}

impl GuardKind {
    pub fn as_i32(self) -> i32 {
        match self {
            GuardKind::Require => 0,
            GuardKind::Ensure => 1,
            GuardKind::LoopInvariant => 2,
            GuardKind::LoopVariant => 3,
            GuardKind::LoopVariantProgress => 4,
        }
    }
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
    // v = bin lhs op rhs (arithmetic/comparisons/bool)
    IBin {
        dst: Value,
        op: BinOpIR,
        lhs: Value,
        rhs: Value,
    },
    // guard: if cond == 0 trap with code; no result value
    Guard {
        cond: Value,
        trap: TrapCode,
        span: Option<(u32, u32)>,
        detail: GuardKind,
    },
    // v = if cond then then_v else else_v (expression form)
    ISelect {
        dst: Value,
        cond: Value,
        then_v: Value,
        else_v: Value,
    },
    // Variant constructor: allocate `{tag, payload_lo, payload_hi, reserved}` and return its pointer
    VariantInit {
        dst: Value,
        tag: Value,
        payload_lo: Value,
        payload_hi: Value,
    },
    // Variant destructors: load individual fields
    VariantLoadTag {
        dst: Value,
        variant: Value,
        kind: VariantKind,
    },
    VariantLoadPayloadLo {
        dst: Value,
        variant: Value,
    },
    VariantLoadPayloadHi {
        dst: Value,
        variant: Value,
    },
    // v? = call callee_idx(args) - callee is a function index in the module
    ReturnIf {
        cond: Value,
        ret: Value,
    },
    Call {
        dst: Option<Value>,
        callee: u32,
        args: Vec<Value>,
    },
    BlockBegin,
    BlockEnd,
    LoopBegin,
    LoopEnd,
    Br {
        depth: u32,
    },
    BrIf {
        cond: Value,
        depth: u32,
    },
    BrIfEqz {
        cond: Value,
        depth: u32,
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
