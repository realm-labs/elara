//! Function prototype bytecode container.

use elara_core::Value;

use crate::Instr;

/// Index into a prototype constant pool.
pub type ConstantIndex = u32;

/// Index into a prototype upvalue list.
pub type UpvalueIndex = u16;

/// Register index inside one stack frame.
pub type Register = u16;

/// Compiled Lua function prototype.
#[derive(Clone, Debug, PartialEq)]
pub struct Proto {
    /// Instruction stream.
    pub code: Box<[Instr]>,
    /// Constant pool.
    pub constants: Box<[Value]>,
    /// Upvalue descriptors.
    pub upvalues: Box<[UpvalueDesc]>,
    /// Nested function prototypes.
    pub children: Box<[Proto]>,
    /// Maximum stack slots required by this function.
    pub max_stack: u16,
    /// Fixed parameter count.
    pub params: u8,
    /// Whether this function accepts varargs.
    pub is_vararg: bool,
    /// Debug metadata.
    pub debug: DebugInfo,
}

impl Proto {
    /// Creates a prototype.
    #[must_use]
    pub fn new(
        code: impl Into<Box<[Instr]>>,
        constants: impl Into<Box<[Value]>>,
        upvalues: impl Into<Box<[UpvalueDesc]>>,
        max_stack: u16,
        params: u8,
        is_vararg: bool,
        debug: DebugInfo,
    ) -> Self {
        Self {
            code: code.into(),
            constants: constants.into(),
            upvalues: upvalues.into(),
            children: Box::new([]),
            max_stack,
            params,
            is_vararg,
            debug,
        }
    }

    /// Replaces nested child prototypes.
    #[must_use]
    pub fn with_children(mut self, children: impl Into<Box<[Proto]>>) -> Self {
        self.children = children.into();
        self
    }

    /// Number of instructions in the prototype.
    #[must_use]
    pub fn len(&self) -> usize {
        self.code.len()
    }

    /// Returns true when the prototype has no instructions.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.code.is_empty()
    }
}

/// Upvalue descriptor placeholder.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UpvalueDesc {
    /// Optional source-level name.
    pub name: Option<Box<str>>,
    /// True when the upvalue captures a parent stack slot.
    pub in_stack: bool,
    /// Parent stack slot or upvalue index.
    pub index: UpvalueIndex,
}

impl UpvalueDesc {
    /// Creates an upvalue descriptor.
    #[must_use]
    pub fn new(name: Option<impl Into<Box<str>>>, in_stack: bool, index: UpvalueIndex) -> Self {
        Self {
            name: name.map(Into::into),
            in_stack,
            index,
        }
    }
}

/// Debug information placeholder.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct DebugInfo {
    /// Optional source name.
    pub source_name: Option<Box<str>>,
    /// One source line per instruction when available.
    pub line_info: Box<[u32]>,
}

impl DebugInfo {
    /// Creates debug info.
    #[must_use]
    pub fn new(source_name: Option<impl Into<Box<str>>>, line_info: impl Into<Box<[u32]>>) -> Self {
        Self {
            source_name: source_name.map(Into::into),
            line_info: line_info.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use elara_core::Value;

    use crate::{DebugInfo, Instr, Op, Proto, UpvalueDesc};

    #[test]
    fn op_proto_stores_code_constants_and_metadata() {
        let proto = Proto::new(
            [
                Instr::abc(Op::LoadK, 0, 0, 0),
                Instr::abc(Op::Return, 0, 1, 0),
            ],
            [Value::integer(42)],
            [UpvalueDesc::new(Some("env"), true, 0)],
            2,
            1,
            false,
            DebugInfo::new(Some("chunk"), [1, 1]),
        );

        assert_eq!(proto.len(), 2);
        assert!(!proto.is_empty());
        assert_eq!(proto.constants[0], Value::integer(42));
        assert_eq!(proto.upvalues[0].name.as_deref(), Some("env"));
        assert!(proto.children.is_empty());
        assert_eq!(proto.max_stack, 2);
        assert_eq!(proto.params, 1);
        assert!(!proto.is_vararg);
        assert_eq!(proto.debug.source_name.as_deref(), Some("chunk"));
        assert_eq!(&*proto.debug.line_info, &[1, 1]);
    }

    #[test]
    fn op_proto_empty_prototype_reports_empty() {
        let proto = Proto::new([], [], [], 0, 0, true, DebugInfo::new(None::<Box<str>>, []));

        assert!(proto.is_empty());
        assert_eq!(proto.len(), 0);
        assert!(proto.is_vararg);
    }
}
