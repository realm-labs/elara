//! Internal bytecode dump/load format.

use elara_core::{Value, ValueTag};

use crate::{DebugInfo, Instr, LocalVarDesc, Proto, UpvalueDesc, VerifyError, verify_proto};

const MAGIC: &[u8; 8] = b"ELBC\r\n\x1a\n";
const FORMAT_VERSION: u16 = 1;

const CONSTANT_NIL: u8 = 0;
const CONSTANT_FALSE: u8 = 1;
const CONSTANT_TRUE: u8 = 2;
const CONSTANT_INTEGER: u8 = 3;
const CONSTANT_FLOAT: u8 = 4;

/// Serializes a prototype tree to Elara's internal bytecode format.
pub fn dump_proto(proto: &Proto) -> Result<Vec<u8>, DumpError> {
    let mut writer = Writer { bytes: Vec::new() };
    writer.bytes.extend_from_slice(MAGIC);
    writer.u16(FORMAT_VERSION);
    write_proto(&mut writer, proto)?;
    Ok(writer.bytes)
}

/// Loads a prototype tree from Elara's internal bytecode format.
pub fn load_proto(bytes: &[u8]) -> Result<Proto, LoadError> {
    let mut reader = Reader { bytes, offset: 0 };
    reader.expect(MAGIC)?;
    let version = reader.u16()?;
    if version != FORMAT_VERSION {
        return Err(LoadError::UnsupportedVersion { version });
    }
    let proto = read_proto(&mut reader)?;
    if !reader.is_empty() {
        return Err(LoadError::TrailingBytes {
            offset: reader.offset,
            trailing: reader.bytes.len() - reader.offset,
        });
    }
    verify_proto_tree(&proto).map_err(LoadError::Verification)?;
    Ok(proto)
}

/// Bytecode dump failure.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DumpError {
    /// A constant cannot be represented in the internal dump format.
    UnsupportedConstant {
        /// Unsupported constant tag.
        tag: ValueTag,
    },
    /// A collection length exceeds the current dump format limit.
    LengthOutOfRange {
        /// Field name.
        field: &'static str,
        /// Actual length.
        len: usize,
    },
}

/// Bytecode load failure.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum LoadError {
    /// Input ended before a complete value could be read.
    UnexpectedEof {
        /// Byte offset where the read started.
        offset: usize,
        /// Number of bytes required.
        needed: usize,
        /// Number of bytes still available.
        available: usize,
    },
    /// Header magic does not match Elara's internal bytecode format.
    BadMagic,
    /// Header format version is not supported by this crate.
    UnsupportedVersion {
        /// Encoded format version.
        version: u16,
    },
    /// Input contains bytes after the prototype tree.
    TrailingBytes {
        /// Offset of the first trailing byte.
        offset: usize,
        /// Number of trailing bytes.
        trailing: usize,
    },
    /// A serialized opcode byte is not known to this build.
    InvalidOpcode {
        /// Encoded instruction word.
        word: u64,
    },
    /// A serialized constant tag is not known to this build.
    InvalidConstantTag {
        /// Encoded constant tag.
        tag: u8,
    },
    /// A loaded prototype failed bytecode verification.
    Verification(Vec<VerifyError>),
}

fn write_proto(writer: &mut Writer, proto: &Proto) -> Result<(), DumpError> {
    writer.u16(proto.max_stack);
    writer.u8(proto.params);
    writer.u8(u8::from(proto.is_vararg));
    writer.instrs(&proto.code)?;
    writer.constants(&proto.constants)?;
    writer.byte_strings("string_constants", &proto.string_constants)?;
    writer.upvalues(&proto.upvalues)?;
    writer.protos(&proto.children)?;
    writer.debug_info(&proto.debug)
}

fn read_proto(reader: &mut Reader<'_>) -> Result<Proto, LoadError> {
    let max_stack = reader.u16()?;
    let params = reader.u8()?;
    let is_vararg = reader.u8()? != 0;
    let code = reader.instrs()?;
    let constants = reader.constants()?;
    let string_constants = reader.byte_strings()?;
    let upvalues = reader.upvalues()?;
    let children = reader.protos()?;
    let debug = reader.debug_info()?;
    Ok(Proto::new(
        code, constants, upvalues, max_stack, params, is_vararg, debug,
    )
    .with_string_constants(string_constants)
    .with_children(children))
}

fn verify_proto_tree(proto: &Proto) -> Result<(), Vec<VerifyError>> {
    verify_proto(proto)?;
    for child in &proto.children {
        verify_proto_tree(child)?;
    }
    Ok(())
}

struct Writer {
    bytes: Vec<u8>,
}

impl Writer {
    fn u8(&mut self, value: u8) {
        self.bytes.push(value);
    }

    fn u16(&mut self, value: u16) {
        self.bytes.extend_from_slice(&value.to_le_bytes());
    }

    fn u32(&mut self, value: u32) {
        self.bytes.extend_from_slice(&value.to_le_bytes());
    }

    fn u64(&mut self, value: u64) {
        self.bytes.extend_from_slice(&value.to_le_bytes());
    }

    fn i64(&mut self, value: i64) {
        self.bytes.extend_from_slice(&value.to_le_bytes());
    }

    fn f64(&mut self, value: f64) {
        self.u64(value.to_bits());
    }

    fn len(&mut self, field: &'static str, len: usize) -> Result<(), DumpError> {
        let len = u32::try_from(len).map_err(|_| DumpError::LengthOutOfRange { field, len })?;
        self.u32(len);
        Ok(())
    }

    fn bytes(&mut self, field: &'static str, value: &[u8]) -> Result<(), DumpError> {
        self.len(field, value.len())?;
        self.bytes.extend_from_slice(value);
        Ok(())
    }

    fn optional_str(&mut self, field: &'static str, value: Option<&str>) -> Result<(), DumpError> {
        if let Some(value) = value {
            self.u8(1);
            self.bytes(field, value.as_bytes())
        } else {
            self.u8(0);
            Ok(())
        }
    }

    fn instrs(&mut self, instrs: &[Instr]) -> Result<(), DumpError> {
        self.len("code", instrs.len())?;
        for instr in instrs {
            self.u64(instr.word());
        }
        Ok(())
    }

    fn constants(&mut self, constants: &[Value]) -> Result<(), DumpError> {
        self.len("constants", constants.len())?;
        for constant in constants {
            match constant.tag() {
                ValueTag::Nil => self.u8(CONSTANT_NIL),
                ValueTag::Bool => self.u8(if constant.as_bool().unwrap_or(false) {
                    CONSTANT_TRUE
                } else {
                    CONSTANT_FALSE
                }),
                ValueTag::Integer => {
                    self.u8(CONSTANT_INTEGER);
                    self.i64(
                        constant
                            .as_integer()
                            .expect("integer tag has integer payload"),
                    );
                }
                ValueTag::Float => {
                    self.u8(CONSTANT_FLOAT);
                    self.f64(constant.as_float().expect("float tag has float payload"));
                }
                tag => return Err(DumpError::UnsupportedConstant { tag }),
            }
        }
        Ok(())
    }

    fn byte_strings(
        &mut self,
        field: &'static str,
        strings: &[Box<[u8]>],
    ) -> Result<(), DumpError> {
        self.len(field, strings.len())?;
        for string in strings {
            self.bytes(field, string)?;
        }
        Ok(())
    }

    fn upvalues(&mut self, upvalues: &[UpvalueDesc]) -> Result<(), DumpError> {
        self.len("upvalues", upvalues.len())?;
        for upvalue in upvalues {
            self.optional_str("upvalue_name", upvalue.name.as_deref())?;
            self.u8(u8::from(upvalue.in_stack));
            self.u16(upvalue.index);
        }
        Ok(())
    }

    fn protos(&mut self, protos: &[Proto]) -> Result<(), DumpError> {
        self.len("children", protos.len())?;
        for proto in protos {
            write_proto(self, proto)?;
        }
        Ok(())
    }

    fn debug_info(&mut self, debug: &DebugInfo) -> Result<(), DumpError> {
        self.optional_str("source_name", debug.source_name.as_deref())?;
        self.len("line_info", debug.line_info.len())?;
        for line in &debug.line_info {
            self.u32(*line);
        }
        self.len("local_vars", debug.local_vars.len())?;
        for local in &debug.local_vars {
            self.bytes("local_name", local.name.as_bytes())?;
            self.u16(local.register);
            self.u32(local.start_pc);
            self.u32(local.end_pc);
        }
        Ok(())
    }
}

struct Reader<'a> {
    bytes: &'a [u8],
    offset: usize,
}

impl Reader<'_> {
    fn is_empty(&self) -> bool {
        self.offset == self.bytes.len()
    }

    fn expect(&mut self, expected: &[u8]) -> Result<(), LoadError> {
        let actual = self.take(expected.len())?;
        if actual == expected {
            Ok(())
        } else {
            Err(LoadError::BadMagic)
        }
    }

    fn take(&mut self, len: usize) -> Result<&[u8], LoadError> {
        let available = self.bytes.len().saturating_sub(self.offset);
        if available < len {
            return Err(LoadError::UnexpectedEof {
                offset: self.offset,
                needed: len,
                available,
            });
        }
        let start = self.offset;
        self.offset += len;
        Ok(&self.bytes[start..self.offset])
    }

    fn u8(&mut self) -> Result<u8, LoadError> {
        Ok(self.take(1)?[0])
    }

    fn u16(&mut self) -> Result<u16, LoadError> {
        let bytes = self.take(2)?;
        Ok(u16::from_le_bytes([bytes[0], bytes[1]]))
    }

    fn u32(&mut self) -> Result<u32, LoadError> {
        let bytes = self.take(4)?;
        Ok(u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
    }

    fn u64(&mut self) -> Result<u64, LoadError> {
        let bytes = self.take(8)?;
        Ok(u64::from_le_bytes([
            bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
        ]))
    }

    fn i64(&mut self) -> Result<i64, LoadError> {
        let bytes = self.take(8)?;
        Ok(i64::from_le_bytes([
            bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
        ]))
    }

    fn f64(&mut self) -> Result<f64, LoadError> {
        Ok(f64::from_bits(self.u64()?))
    }

    fn len(&mut self) -> Result<usize, LoadError> {
        Ok(self.u32()? as usize)
    }

    fn bytes(&mut self) -> Result<Box<[u8]>, LoadError> {
        let len = self.len()?;
        Ok(self.take(len)?.into())
    }

    fn optional_str(&mut self) -> Result<Option<Box<str>>, LoadError> {
        if self.u8()? == 0 {
            return Ok(None);
        }
        let bytes = self.bytes()?;
        let string = String::from_utf8(bytes.into_vec()).map_err(|_| LoadError::BadMagic)?;
        Ok(Some(string.into_boxed_str()))
    }

    fn instrs(&mut self) -> Result<Box<[Instr]>, LoadError> {
        let len = self.len()?;
        let mut instrs = Vec::with_capacity(len);
        for _ in 0..len {
            let word = self.u64()?;
            let instr = Instr::from_word(word).ok_or(LoadError::InvalidOpcode { word })?;
            instrs.push(instr);
        }
        Ok(instrs.into_boxed_slice())
    }

    fn constants(&mut self) -> Result<Box<[Value]>, LoadError> {
        let len = self.len()?;
        let mut constants = Vec::with_capacity(len);
        for _ in 0..len {
            constants.push(match self.u8()? {
                CONSTANT_NIL => Value::nil(),
                CONSTANT_FALSE => Value::boolean(false),
                CONSTANT_TRUE => Value::boolean(true),
                CONSTANT_INTEGER => Value::integer(self.i64()?),
                CONSTANT_FLOAT => Value::float(self.f64()?),
                tag => return Err(LoadError::InvalidConstantTag { tag }),
            });
        }
        Ok(constants.into_boxed_slice())
    }

    fn byte_strings(&mut self) -> Result<Box<[Box<[u8]>]>, LoadError> {
        let len = self.len()?;
        let mut strings = Vec::with_capacity(len);
        for _ in 0..len {
            strings.push(self.bytes()?);
        }
        Ok(strings.into_boxed_slice())
    }

    fn upvalues(&mut self) -> Result<Box<[UpvalueDesc]>, LoadError> {
        let len = self.len()?;
        let mut upvalues = Vec::with_capacity(len);
        for _ in 0..len {
            upvalues.push(UpvalueDesc {
                name: self.optional_str()?,
                in_stack: self.u8()? != 0,
                index: self.u16()?,
            });
        }
        Ok(upvalues.into_boxed_slice())
    }

    fn protos(&mut self) -> Result<Box<[Proto]>, LoadError> {
        let len = self.len()?;
        let mut protos = Vec::with_capacity(len);
        for _ in 0..len {
            protos.push(read_proto(self)?);
        }
        Ok(protos.into_boxed_slice())
    }

    fn debug_info(&mut self) -> Result<DebugInfo, LoadError> {
        let source_name = self.optional_str()?;
        let line_len = self.len()?;
        let mut line_info = Vec::with_capacity(line_len);
        for _ in 0..line_len {
            line_info.push(self.u32()?);
        }
        let local_len = self.len()?;
        let mut local_vars = Vec::with_capacity(local_len);
        for _ in 0..local_len {
            let name =
                String::from_utf8(self.bytes()?.into_vec()).map_err(|_| LoadError::BadMagic)?;
            local_vars.push(LocalVarDesc {
                name: name.into_boxed_str(),
                register: self.u16()?,
                start_pc: self.u32()?,
                end_pc: self.u32()?,
            });
        }
        Ok(DebugInfo {
            source_name,
            line_info: line_info.into_boxed_slice(),
            local_vars: local_vars.into_boxed_slice(),
        })
    }
}

#[cfg(test)]
mod tests {
    use elara_core::Value;

    use crate::{
        DebugInfo, DumpError, Instr, LoadError, Op, Proto, ProtoBuilder, UpvalueDesc, dump_proto,
        load_proto,
    };

    #[test]
    fn dump_load_round_trips_proto_tree() {
        let proto = sample_proto();

        let dumped = dump_proto(&proto).expect("proto should dump");
        let loaded = load_proto(&dumped).expect("proto should load");

        assert_eq!(loaded, proto);
    }

    #[test]
    fn dump_rejects_runtime_identity_constants() {
        let proto = Proto::new(
            [
                Instr::abc(Op::LoadK, 0, 0, 0),
                Instr::abc(Op::Return, 0, 1, 0),
            ],
            [Value::table_index(0)],
            [],
            1,
            0,
            false,
            DebugInfo::new(None::<Box<str>>, [1, 1]),
        );

        assert_eq!(
            dump_proto(&proto),
            Err(DumpError::UnsupportedConstant {
                tag: Value::table_index(0).tag()
            })
        );
    }

    #[test]
    fn load_rejects_bad_header_version_and_trailing_bytes() {
        assert_eq!(load_proto(b"not elbc"), Err(LoadError::BadMagic));

        let proto = sample_proto();
        let mut dumped = dump_proto(&proto).expect("proto should dump");
        dumped[8] = 2;
        assert_eq!(
            load_proto(&dumped),
            Err(LoadError::UnsupportedVersion { version: 2 })
        );

        let mut dumped = dump_proto(&proto).expect("proto should dump");
        dumped.push(0);
        assert_eq!(
            load_proto(&dumped),
            Err(LoadError::TrailingBytes {
                offset: dumped.len() - 1,
                trailing: 1,
            })
        );
    }

    #[test]
    fn load_rejects_invalid_opcode_and_verifier_failures() {
        let mut dumped = dump_proto(&sample_proto()).expect("proto should dump");
        let first_instr_offset = 8 + 2 + 2 + 1 + 1 + 4;
        dumped[first_instr_offset] = 0xff;
        let word = u64::from_le_bytes(
            dumped[first_instr_offset..first_instr_offset + 8]
                .try_into()
                .expect("instruction word should have eight bytes"),
        );
        assert_eq!(load_proto(&dumped), Err(LoadError::InvalidOpcode { word }));

        let proto = Proto::new(
            [Instr::abx(Op::LoadK, 0, 2), Instr::abc(Op::Return, 0, 1, 0)],
            [Value::integer(1)],
            [],
            1,
            0,
            false,
            DebugInfo::new(None::<Box<str>>, [1, 1]),
        );
        assert!(matches!(
            load_proto(&dump_proto(&proto).expect("proto should dump")),
            Err(LoadError::Verification(_))
        ));

        let invalid_child = proto;
        let parent = Proto::new(
            [
                Instr::abx(Op::Closure, 0, 0),
                Instr::abc(Op::Return, 0, 1, 0),
            ],
            [],
            [],
            1,
            0,
            false,
            DebugInfo::new(None::<Box<str>>, [1, 1]),
        )
        .with_children([invalid_child]);
        assert!(matches!(
            load_proto(&dump_proto(&parent).expect("proto should dump")),
            Err(LoadError::Verification(_))
        ));
    }

    fn sample_proto() -> Proto {
        let mut child = ProtoBuilder::new()
            .with_signature(2, 1, true)
            .with_source_name("child.lua");
        let constant = child.add_constant(Value::float(3.5));
        child.add_upvalue(UpvalueDesc::new(Some("env"), true, 0));
        child.add_local_var("arg", 0, 0, 2);
        child.emit_line(Instr::abx(Op::LoadK, 1, u64::from(constant)), 7);
        child.emit_line(Instr::abc(Op::Return, 1, 1, 0), 8);
        let child = child.finish();

        let mut builder = ProtoBuilder::new()
            .with_signature(3, 0, false)
            .with_source_name("main.lua");
        let hello = builder.add_string_constant(b"hello\0world");
        let integer = builder.add_constant(Value::integer(-42));
        let truth = builder.add_constant(Value::boolean(true));
        let nil = builder.add_constant(Value::nil());
        let child_index = builder.add_child(child);
        builder.add_local_var("x", 0, 0, 5);
        builder.emit_line(Instr::abx(Op::LoadString, 0, u64::from(hello)), 1);
        builder.emit_line(Instr::abx(Op::LoadK, 1, u64::from(integer)), 2);
        builder.emit_line(Instr::abx(Op::LoadK, 2, u64::from(truth)), 3);
        builder.emit_line(Instr::abx(Op::LoadK, 2, u64::from(nil)), 4);
        builder.emit_line(Instr::abx(Op::Closure, 0, u64::from(child_index)), 5);
        builder.emit_line(Instr::abc(Op::Return, 0, 1, 0), 6);
        builder.finish()
    }
}
