//! Human-readable bytecode disassembly.

use core::fmt::Write as _;

use elara_core::Value;

use crate::{Instr, Op, Proto};

/// Disassembles a prototype into text.
#[must_use]
pub fn disassemble(proto: &Proto) -> String {
    let mut output = String::new();
    for (offset, instr) in proto.code.iter().copied().enumerate() {
        write_instruction(&mut output, offset, instr, proto);
        output.push('\n');
    }
    output
}

fn write_instruction(output: &mut String, offset: usize, instr: Instr, proto: &Proto) {
    let _ = write!(output, "{offset:04} {:<13}", instr.op().mnemonic());
    match instr_format(instr.op()) {
        InstrFormat::Abc => {
            let _ = write!(output, " A={} B={} C={}", instr.a(), instr.b(), instr.c());
        }
        InstrFormat::Abx => {
            let _ = write!(output, " A={} Bx={}", instr.a(), instr.bx());
        }
        InstrFormat::Asbx => {
            let _ = write!(output, " A={} sBx={}", instr.a(), instr.sbx());
        }
    }

    if instr.op() == Op::LoadK
        && let Some(value) = proto.constants.get(instr.bx() as usize)
    {
        let _ = write!(output, " ; {}", display_value(*value));
    }
}

fn display_value(value: Value) -> String {
    if value.is_nil() {
        return "nil".to_owned();
    }
    if let Some(value) = value.as_bool() {
        return value.to_string();
    }
    if let Some(value) = value.as_integer() {
        return value.to_string();
    }
    if let Some(value) = value.as_float() {
        return value.to_string();
    }
    format!("{value:?}")
}

fn instr_format(op: Op) -> InstrFormat {
    match op {
        Op::LoadK | Op::Closure | Op::DeclGlobal => InstrFormat::Abx,
        Op::Jmp | Op::ForPrep | Op::ForLoop | Op::TForPrep | Op::TForLoop => InstrFormat::Asbx,
        _ => InstrFormat::Abc,
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum InstrFormat {
    Abc,
    Abx,
    Asbx,
}

#[cfg(test)]
mod tests {
    use elara_core::Value;

    use crate::{Instr, Op, ProtoBuilder, disassemble};

    #[test]
    fn disasm_builder_constructs_proto_with_constants() {
        let mut builder = ProtoBuilder::new()
            .with_signature(2, 0, false)
            .with_source_name("chunk");
        let constant = builder.add_constant(Value::integer(42));

        builder.emit_abx(Op::LoadK, 0, u64::from(constant));
        builder.emit_abc(Op::Return, 0, 1, 0);

        let proto = builder.finish();

        assert_eq!(proto.max_stack, 2);
        assert_eq!(proto.constants[0], Value::integer(42));
        assert_eq!(proto.debug.source_name.as_deref(), Some("chunk"));
        assert_eq!(&*proto.debug.line_info, &[0, 0]);
    }

    #[test]
    fn disasm_formats_simple_instruction_sequence() {
        let mut builder = ProtoBuilder::new().with_signature(2, 0, false);
        let constant = builder.add_constant(Value::integer(42));
        builder.emit_abx(Op::LoadK, 0, u64::from(constant));
        builder.emit_abc(Op::Add, 1, 0, 0);
        builder.emit_abc(Op::Return, 1, 1, 0);
        let proto = builder.finish();

        assert_eq!(
            disassemble(&proto),
            "0000 LOAD_K        A=0 Bx=0 ; 42\n0001 ADD           A=1 B=0 C=0\n0002 RETURN        A=1 B=1 C=0\n"
        );
    }

    #[test]
    fn disasm_formats_signed_jump_operands() {
        let mut builder = ProtoBuilder::new().with_signature(0, 0, false);
        builder.emit(Instr::asbx(Op::Jmp, 0, -2));
        let proto = builder.finish();

        assert_eq!(disassemble(&proto), "0000 JMP           A=0 sBx=-2\n");
    }
}
