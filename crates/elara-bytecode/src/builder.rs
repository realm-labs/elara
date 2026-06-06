//! Bytecode prototype builder.

use elara_core::Value;

use crate::{ConstantIndex, DebugInfo, Instr, Op, Proto, StringIndex, UpvalueDesc, UpvalueIndex};

/// Incremental builder for a function prototype.
#[derive(Debug, Default)]
pub struct ProtoBuilder {
    code: Vec<Instr>,
    constants: Vec<Value>,
    string_constants: Vec<Box<[u8]>>,
    upvalues: Vec<UpvalueDesc>,
    children: Vec<Proto>,
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

    /// Adds a string constant and returns its pool index.
    pub fn add_string_constant(&mut self, value: impl AsRef<[u8]>) -> StringIndex {
        let index = StringIndex::try_from(self.string_constants.len())
            .expect("string constant pool index must fit in u32");
        self.string_constants.push(value.as_ref().into());
        index
    }

    /// Adds an upvalue descriptor and returns its index.
    pub fn add_upvalue(&mut self, upvalue: UpvalueDesc) -> UpvalueIndex {
        let index =
            UpvalueIndex::try_from(self.upvalues.len()).expect("upvalue index must fit in u16");
        self.upvalues.push(upvalue);
        index
    }

    /// Adds a child prototype and returns its index.
    pub fn add_child(&mut self, proto: Proto) -> u32 {
        let index = u32::try_from(self.children.len()).expect("child proto index must fit in u32");
        self.children.push(proto);
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

    /// Number of emitted instructions.
    #[must_use]
    pub fn code_len(&self) -> usize {
        self.code.len()
    }

    /// Replaces an emitted AsBx instruction.
    ///
    /// # Panics
    ///
    /// Panics when `offset` does not refer to an emitted instruction.
    pub fn patch_asbx(&mut self, offset: usize, op: Op, a: u16, sbx: i64) {
        self.code[offset] = Instr::asbx(op, a, sbx);
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
        .with_children(self.children)
        .with_string_constants(self.string_constants)
    }
}
