//! Opcode and instruction encoding.

/// Number of bits reserved for an opcode.
pub const OP_BITS: u8 = 8;
/// Number of bits reserved for operand A.
pub const A_BITS: u8 = 16;
/// Number of bits reserved for operand B.
pub const B_BITS: u8 = 20;
/// Number of bits reserved for operand C.
pub const C_BITS: u8 = 20;

/// Largest encodable operand A.
pub const MAX_A: u16 = u16::MAX;
/// Largest encodable operand B.
pub const MAX_B: u32 = (1 << B_BITS) - 1;
/// Largest encodable operand C.
pub const MAX_C: u32 = (1 << C_BITS) - 1;

const OP_MASK: u64 = (1 << OP_BITS) - 1;
const A_SHIFT: u8 = OP_BITS;
const B_SHIFT: u8 = A_SHIFT + A_BITS;
const C_SHIFT: u8 = B_SHIFT + B_BITS;
const BC_BITS: u8 = B_BITS + C_BITS;
const MAX_BC: u64 = (1_u64 << BC_BITS) - 1;
const SBX_BIAS: i64 = (1_i64 << (BC_BITS - 1)) - 1;

/// Internal bytecode opcode.
#[repr(u8)]
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum Op {
    /// Copy one register to another.
    Move,
    /// Load `nil`.
    LoadNil,
    /// Load boolean.
    LoadBool,
    /// Load small integer immediate.
    LoadInt,
    /// Load floating-point immediate.
    LoadFloat,
    /// Load constant.
    LoadK,
    /// Read upvalue.
    GetUpvalue,
    /// Write upvalue.
    SetUpvalue,
    /// Read environment/global table.
    GetEnv,
    /// Write environment/global table.
    SetEnv,
    /// Declare current-version global.
    DeclGlobal,
    /// Create table.
    NewTable,
    /// Generic table read.
    GetTable,
    /// Generic table write.
    SetTable,
    /// Integer index read.
    GetIndex,
    /// Integer index write.
    SetIndex,
    /// Length operator.
    Len,
    /// Addition.
    Add,
    /// Subtraction.
    Sub,
    /// Multiplication.
    Mul,
    /// Division.
    Div,
    /// Floor division.
    IDiv,
    /// Modulo.
    Mod,
    /// Power.
    Pow,
    /// Unary minus.
    Unm,
    /// Bitwise and.
    BAnd,
    /// Bitwise or.
    BOr,
    /// Bitwise xor.
    BXor,
    /// Shift left.
    Shl,
    /// Shift right.
    Shr,
    /// Bitwise not.
    BNot,
    /// Relative jump.
    Jmp,
    /// Truth test.
    Test,
    /// Truth test with assignment.
    TestSet,
    /// Equality comparison.
    Eq,
    /// Less-than comparison.
    Lt,
    /// Less-or-equal comparison.
    Le,
    /// Numeric for-loop preparation.
    ForPrep,
    /// Numeric for-loop iteration.
    ForLoop,
    /// Generic for-loop preparation.
    TForPrep,
    /// Generic for-loop call.
    TForCall,
    /// Generic for-loop iteration.
    TForLoop,
    /// Function call.
    Call,
    /// Tail call.
    TailCall,
    /// Return from function.
    Return,
    /// Load varargs.
    Vararg,
    /// Load named vararg table.
    VarargTable,
    /// Create closure.
    Closure,
    /// Close upvalues.
    Close,
    /// Mark to-be-closed range.
    Tbc,
}

impl Op {
    /// Decodes an opcode byte.
    #[must_use]
    pub const fn from_byte(byte: u8) -> Option<Self> {
        match byte {
            0 => Some(Self::Move),
            1 => Some(Self::LoadNil),
            2 => Some(Self::LoadBool),
            3 => Some(Self::LoadInt),
            4 => Some(Self::LoadFloat),
            5 => Some(Self::LoadK),
            6 => Some(Self::GetUpvalue),
            7 => Some(Self::SetUpvalue),
            8 => Some(Self::GetEnv),
            9 => Some(Self::SetEnv),
            10 => Some(Self::DeclGlobal),
            11 => Some(Self::NewTable),
            12 => Some(Self::GetTable),
            13 => Some(Self::SetTable),
            14 => Some(Self::GetIndex),
            15 => Some(Self::SetIndex),
            16 => Some(Self::Len),
            17 => Some(Self::Add),
            18 => Some(Self::Sub),
            19 => Some(Self::Mul),
            20 => Some(Self::Div),
            21 => Some(Self::IDiv),
            22 => Some(Self::Mod),
            23 => Some(Self::Pow),
            24 => Some(Self::Unm),
            25 => Some(Self::BAnd),
            26 => Some(Self::BOr),
            27 => Some(Self::BXor),
            28 => Some(Self::Shl),
            29 => Some(Self::Shr),
            30 => Some(Self::BNot),
            31 => Some(Self::Jmp),
            32 => Some(Self::Test),
            33 => Some(Self::TestSet),
            34 => Some(Self::Eq),
            35 => Some(Self::Lt),
            36 => Some(Self::Le),
            37 => Some(Self::ForPrep),
            38 => Some(Self::ForLoop),
            39 => Some(Self::TForPrep),
            40 => Some(Self::TForCall),
            41 => Some(Self::TForLoop),
            42 => Some(Self::Call),
            43 => Some(Self::TailCall),
            44 => Some(Self::Return),
            45 => Some(Self::Vararg),
            46 => Some(Self::VarargTable),
            47 => Some(Self::Closure),
            48 => Some(Self::Close),
            49 => Some(Self::Tbc),
            _ => None,
        }
    }

    /// Stable opcode mnemonic.
    #[must_use]
    pub const fn mnemonic(self) -> &'static str {
        match self {
            Self::Move => "MOVE",
            Self::LoadNil => "LOAD_NIL",
            Self::LoadBool => "LOAD_BOOL",
            Self::LoadInt => "LOAD_INT",
            Self::LoadFloat => "LOAD_FLOAT",
            Self::LoadK => "LOAD_K",
            Self::GetUpvalue => "GET_UPVALUE",
            Self::SetUpvalue => "SET_UPVALUE",
            Self::GetEnv => "GET_ENV",
            Self::SetEnv => "SET_ENV",
            Self::DeclGlobal => "DECL_GLOBAL",
            Self::NewTable => "NEW_TABLE",
            Self::GetTable => "GET_TABLE",
            Self::SetTable => "SET_TABLE",
            Self::GetIndex => "GET_INDEX",
            Self::SetIndex => "SET_INDEX",
            Self::Len => "LEN",
            Self::Add => "ADD",
            Self::Sub => "SUB",
            Self::Mul => "MUL",
            Self::Div => "DIV",
            Self::IDiv => "IDIV",
            Self::Mod => "MOD",
            Self::Pow => "POW",
            Self::Unm => "UNM",
            Self::BAnd => "BAND",
            Self::BOr => "BOR",
            Self::BXor => "BXOR",
            Self::Shl => "SHL",
            Self::Shr => "SHR",
            Self::BNot => "BNOT",
            Self::Jmp => "JMP",
            Self::Test => "TEST",
            Self::TestSet => "TEST_SET",
            Self::Eq => "EQ",
            Self::Lt => "LT",
            Self::Le => "LE",
            Self::ForPrep => "FOR_PREP",
            Self::ForLoop => "FOR_LOOP",
            Self::TForPrep => "TFOR_PREP",
            Self::TForCall => "TFOR_CALL",
            Self::TForLoop => "TFOR_LOOP",
            Self::Call => "CALL",
            Self::TailCall => "TAIL_CALL",
            Self::Return => "RETURN",
            Self::Vararg => "VARARG",
            Self::VarargTable => "VARARG_TABLE",
            Self::Closure => "CLOSURE",
            Self::Close => "CLOSE",
            Self::Tbc => "TBC",
        }
    }
}

/// One packed bytecode instruction.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub struct Instr(u64);

impl Instr {
    /// Creates an ABC instruction.
    ///
    /// # Panics
    ///
    /// Panics if `b` or `c` exceeds the encodable operand range.
    #[must_use]
    pub const fn abc(op: Op, a: u16, b: u32, c: u32) -> Self {
        assert!(b <= MAX_B, "operand B out of range");
        assert!(c <= MAX_C, "operand C out of range");
        Self(
            (op as u64)
                | ((a as u64) << A_SHIFT)
                | ((b as u64) << B_SHIFT)
                | ((c as u64) << C_SHIFT),
        )
    }

    /// Creates an ABx instruction with a combined unsigned B/C operand.
    ///
    /// # Panics
    ///
    /// Panics if `bx` exceeds the encodable operand range.
    #[must_use]
    pub const fn abx(op: Op, a: u16, bx: u64) -> Self {
        assert!(bx <= MAX_BC, "operand Bx out of range");
        Self((op as u64) | ((a as u64) << A_SHIFT) | (bx << B_SHIFT))
    }

    /// Creates an AsBx instruction with a signed combined B/C operand.
    ///
    /// # Panics
    ///
    /// Panics if `sbx` exceeds the encodable signed operand range.
    #[must_use]
    pub const fn asbx(op: Op, a: u16, sbx: i64) -> Self {
        assert!(
            sbx >= -SBX_BIAS && sbx <= SBX_BIAS,
            "operand sBx out of range"
        );
        Self::abx(op, a, (sbx + SBX_BIAS) as u64)
    }

    /// Raw instruction word.
    #[must_use]
    pub const fn word(self) -> u64 {
        self.0
    }

    /// Decoded opcode.
    #[must_use]
    pub const fn op(self) -> Op {
        let byte = (self.0 & OP_MASK) as u8;
        match Op::from_byte(byte) {
            Some(op) => op,
            None => panic!("invalid encoded opcode"),
        }
    }

    /// Operand A.
    #[must_use]
    pub const fn a(self) -> u16 {
        ((self.0 >> A_SHIFT) & (u16::MAX as u64)) as u16
    }

    /// Operand B.
    #[must_use]
    pub const fn b(self) -> u32 {
        ((self.0 >> B_SHIFT) & (MAX_B as u64)) as u32
    }

    /// Operand C.
    #[must_use]
    pub const fn c(self) -> u32 {
        ((self.0 >> C_SHIFT) & (MAX_C as u64)) as u32
    }

    /// Combined unsigned B/C operand.
    #[must_use]
    pub const fn bx(self) -> u64 {
        (self.0 >> B_SHIFT) & MAX_BC
    }

    /// Combined signed B/C operand.
    #[must_use]
    pub const fn sbx(self) -> i64 {
        self.bx() as i64 - SBX_BIAS
    }
}

#[cfg(test)]
mod tests {
    use super::{Instr, MAX_B, MAX_C, Op};

    #[test]
    fn op_decodes_stable_opcode_bytes() {
        assert_eq!(Op::from_byte(0), Some(Op::Move));
        assert_eq!(Op::from_byte(44), Some(Op::Return));
        assert_eq!(Op::from_byte(49), Some(Op::Tbc));
        assert_eq!(Op::from_byte(50), None);
    }

    #[test]
    fn op_exposes_stable_mnemonics() {
        assert_eq!(Op::Move.mnemonic(), "MOVE");
        assert_eq!(Op::LoadK.mnemonic(), "LOAD_K");
        assert_eq!(Op::VarargTable.mnemonic(), "VARARG_TABLE");
    }

    #[test]
    fn op_instr_round_trips_abc_operands() {
        let instr = Instr::abc(Op::Add, 7, MAX_B, MAX_C);

        assert_eq!(instr.op(), Op::Add);
        assert_eq!(instr.a(), 7);
        assert_eq!(instr.b(), MAX_B);
        assert_eq!(instr.c(), MAX_C);
    }

    #[test]
    fn op_instr_round_trips_abx_operands() {
        let instr = Instr::abx(Op::LoadK, 3, 42);

        assert_eq!(instr.op(), Op::LoadK);
        assert_eq!(instr.a(), 3);
        assert_eq!(instr.bx(), 42);
    }

    #[test]
    fn op_instr_round_trips_signed_jump_operands() {
        let back = Instr::asbx(Op::Jmp, 0, -8);
        let forward = Instr::asbx(Op::Jmp, 0, 12);

        assert_eq!(back.sbx(), -8);
        assert_eq!(forward.sbx(), 12);
    }
}
