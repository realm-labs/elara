//! Bytecode prototype builder.

use elara_core::Value;

use crate::{ConstantIndex, DebugInfo, Instr, Op, Proto, UpvalueDesc, UpvalueIndex};

/// Incremental builder for a function prototype.
#[derive(Debug, Default)]
pub struct ProtoBuilder {
    code: Vec<Instr>,
    constants: Vec<Value>,
    upvalues: Vec<UpvalueDesc>,
    line_info: Vec<u32>,
    source_name: Option<Box<str>>,
    max_stack: u16,
    params: u8,
    is_vararg: bool,
}

impl ProtoBuilder {
    /// Creates an empty builder.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets prototype stack and parameter metadata.
    #[must_use]
    pub const fn with_signature(mut self, max_stack: u16, params: u8, is_vararg: bool) -> Self {
        self.max_stack = max_stack;
        self.params = params;
        self.is_vararg = is_vararg;
        self
    }

    /// Sets the optional debug source name.
    #[must_use]
    pub fn with_source_name(mut self, source_name: impl Into<Box<str>>) -> Self {
        self.source_name = Some(source_name.into());
        self
    }

    /// Adds a constant and returns its pool index.
    pub fn add_constant(&mut self, value: Value) -> ConstantIndex {
        let index = ConstantIndex::try_from(self.constants.len())
            .expect("constant pool index must fit in u32");
        self.constants.push(value);
        index
    }

    /// Adds an upvalue descriptor and returns its index.
    pub fn add_upvalue(&mut self, upvalue: UpvalueDesc) -> UpvalueIndex {
        let index =
            UpvalueIndex::try_from(self.upvalues.len()).expect("upvalue index must fit in u16");
        self.upvalues.push(upvalue);
        index
    }

    /// Emits an already-encoded instruction.
    pub fn emit(&mut self, instr: Instr) -> usize {
        self.emit_line(instr, 0)
    }

    /// Emits an instruction with a source line.
    pub fn emit_line(&mut self, instr: Instr, line: u32) -> usize {
        let offset = self.code.len();
        self.code.push(instr);
        self.line_info.push(line);
        offset
    }

    /// Emits an ABC instruction.
    pub fn emit_abc(&mut self, op: Op, a: u16, b: u32, c: u32) -> usize {
        self.emit(Instr::abc(op, a, b, c))
    }

    /// Emits an ABx instruction.
    pub fn emit_abx(&mut self, op: Op, a: u16, bx: u64) -> usize {
        self.emit(Instr::abx(op, a, bx))
    }

    /// Emits an AsBx instruction.
    pub fn emit_asbx(&mut self, op: Op, a: u16, sbx: i64) -> usize {
        self.emit(Instr::asbx(op, a, sbx))
    }

    /// Finishes the prototype.
    #[must_use]
    pub fn finish(self) -> Proto {
        Proto::new(
            self.code,
            self.constants,
            self.upvalues,
            self.max_stack,
            self.params,
            self.is_vararg,
            DebugInfo {
                source_name: self.source_name,
                line_info: self.line_info.into_boxed_slice(),
            },
        )
    }
}
