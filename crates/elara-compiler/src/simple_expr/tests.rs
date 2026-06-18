use elara_bytecode::{LocalVarDesc, Op, disassemble};
use elara_core::{SHORT_STRING_MAX_BYTES, SourceId};
use elara_test::assert_snapshot_eq;

use crate::compile_simple_chunk;

#[test]
fn simple_expr_compiles_return_arithmetic() {
    let compiled = compile_simple_chunk(SourceId::new(0), "return 1 + 2 * 3");
    assert_eq!(compiled.diagnostics, Vec::new());
    let proto = compiled.proto.expect("expected compiled proto");

    assert_eq!(proto.constants.len(), 3);
    assert_eq!(proto.code.last().map(|instr| instr.op()), Some(Op::Return));
    assert_snapshot_eq(
        disassemble(&proto),
        "0000 LOAD_K        A=0 Bx=0 ; 1\n0001 LOAD_K        A=1 Bx=1 ; 2\n0002 LOAD_K        A=2 Bx=2 ; 3\n0003 MUL           A=3 B=1 C=2\n0004 ADD           A=4 B=0 C=3\n0005 RETURN        A=4 B=1 C=0\n",
    );
}

#[test]
fn simple_expr_compiles_add_integer_superinstruction() {
    let compiled = compile_simple_chunk(SourceId::new(0), "local x = 1\nreturn x + 2");
    assert_eq!(compiled.diagnostics, Vec::new());
    let proto = compiled.proto.expect("expected compiled proto");

    assert_snapshot_eq(
        disassemble(&proto),
        "0000 LOAD_K        A=0 Bx=0 ; 1\n0001 MOVE          A=1 B=0 C=0\n0002 ADD_INT       A=2 B=1 C=2\n0003 RETURN        A=2 B=1 C=0\n",
    );
}

#[test]
fn simple_expr_compiles_unary_arithmetic() {
    let compiled = compile_simple_chunk(SourceId::new(0), "return -1");
    assert_eq!(compiled.diagnostics, Vec::new());
    let proto = compiled.proto.expect("expected compiled proto");

    assert_snapshot_eq(
        disassemble(&proto),
        "0000 LOAD_K        A=0 Bx=0 ; 1\n0001 UNM           A=1 B=0 C=0\n0002 RETURN        A=1 B=1 C=0\n",
    );
}

#[test]
fn simple_expr_compiles_unary_not() {
    let compiled = compile_simple_chunk(SourceId::new(0), "return not nil");
    assert_eq!(compiled.diagnostics, Vec::new());
    let proto = compiled.proto.expect("expected compiled proto");

    assert_snapshot_eq(
        disassemble(&proto),
        "0000 LOAD_NIL      A=0 B=0 C=0\n0001 NOT           A=1 B=0 C=0\n0002 RETURN        A=1 B=1 C=0\n",
    );
}

#[test]
fn simple_expr_compiles_unary_length() {
    let compiled = compile_simple_chunk(SourceId::new(0), "return #'abc'");
    assert_eq!(compiled.diagnostics, Vec::new());
    let proto = compiled.proto.expect("expected compiled proto");

    assert_snapshot_eq(
        disassemble(&proto),
        "0000 LOAD_STRING   A=0 Bx=0 ; \"abc\"\n0001 LEN           A=1 B=0 C=0\n0002 RETURN        A=1 B=1 C=0\n",
    );
}

#[test]
fn simple_expr_compiles_unary_without_clobbering_operand_register() {
    let compiled = compile_simple_chunk(SourceId::new(0), "local value = 'ab'\nreturn #value, value");
    assert_eq!(compiled.diagnostics, Vec::new());
    let proto = compiled.proto.expect("expected compiled proto");

    assert_snapshot_eq(
        disassemble(&proto),
        "0000 LOAD_STRING   A=0 Bx=0 ; \"ab\"\n0001 MOVE          A=1 B=0 C=0\n0002 LEN           A=2 B=1 C=0\n0003 MOVE          A=3 B=2 C=0\n0004 MOVE          A=4 B=1 C=0\n0005 RETURN        A=3 B=2 C=0\n",
    );
}

#[test]
fn simple_expr_compiles_long_string_literal() {
    let literal = "a".repeat(SHORT_STRING_MAX_BYTES + 1);
    let source = format!("return '{literal}'");
    let compiled = compile_simple_chunk(SourceId::new(0), &source);
    assert_eq!(compiled.diagnostics, Vec::new());
    let proto = compiled.proto.expect("expected compiled proto");

    assert_eq!(proto.string_constants.len(), 1);
    assert_eq!(proto.string_constants[0].as_ref(), literal.as_bytes());
    assert_snapshot_eq(
        disassemble(&proto),
        "0000 LOAD_STRING   A=0 Bx=0 ; \"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\"\n0001 RETURN        A=0 B=1 C=0\n",
    );
}

#[test]
fn simple_expr_compiles_long_bracket_string_literal() {
    let compiled = compile_simple_chunk(SourceId::new(0), "return [=[alpha]=], [=[\nbeta]=]");
    assert_eq!(compiled.diagnostics, Vec::new());
    let proto = compiled.proto.expect("expected compiled proto");

    assert_eq!(proto.string_constants.len(), 2);
    assert_eq!(proto.string_constants[0].as_ref(), b"alpha");
    assert_eq!(proto.string_constants[1].as_ref(), b"beta");
    assert_snapshot_eq(
        disassemble(&proto),
        "0000 LOAD_STRING   A=0 Bx=0 ; \"alpha\"\n0001 LOAD_STRING   A=1 Bx=1 ; \"beta\"\n0002 RETURN        A=0 B=2 C=0\n",
    );
}

#[test]
fn simple_expr_decodes_quoted_string_escapes() {
    let source = "return \"a\\n\\t\\\\\\\"\\'\\x41\\65\\z\n        b\\u{20ac}\", \"x\\\ny\"";
    let compiled = compile_simple_chunk(SourceId::new(0), source);
    assert_eq!(compiled.diagnostics, Vec::new());
    let proto = compiled.proto.expect("expected compiled proto");

    assert_eq!(proto.string_constants.len(), 2);
    assert_eq!(
        proto.string_constants[0].as_ref(),
        b"a\n\t\\\"'AAb\xe2\x82\xac"
    );
    assert_eq!(proto.string_constants[1].as_ref(), b"x\ny");
}

#[test]
fn simple_expr_compiles_logical_short_circuit() {
    let compiled = compile_simple_chunk(SourceId::new(0), "return false and 1, true or 2");
    assert_eq!(compiled.diagnostics, Vec::new());
    let proto = compiled.proto.expect("expected compiled proto");
    let disassembly = disassemble(&proto);

    assert!(
        disassembly.contains("TEST          A=0 B=0 C=0"),
        "`and` should skip the right operand when the left value is falsey"
    );
    assert!(
        disassembly.contains("TEST          A=2 B=1 C=0"),
        "`or` should skip the right operand when the left value is truthy"
    );
}

#[test]
fn simple_expr_compiles_bitwise_operations() {
    let compiled = compile_simple_chunk(SourceId::new(0), "return ~(1 & 3) | (8 >> 1)");
    assert_eq!(compiled.diagnostics, Vec::new());
    let proto = compiled.proto.expect("expected compiled proto");

    assert_snapshot_eq(
        disassemble(&proto),
        "0000 LOAD_K        A=0 Bx=0 ; 1\n0001 LOAD_K        A=1 Bx=1 ; 3\n0002 BAND          A=2 B=0 C=1\n0003 BNOT          A=3 B=2 C=0\n0004 LOAD_K        A=4 Bx=2 ; 8\n0005 LOAD_K        A=5 Bx=3 ; 1\n0006 SHR           A=6 B=4 C=5\n0007 BOR           A=7 B=3 C=6\n0008 RETURN        A=7 B=1 C=0\n",
    );
}

#[test]
fn simple_expr_compiles_comparisons() {
    let compiled = compile_simple_chunk(
        SourceId::new(0),
        "return 1 == 1, 1 ~= 2, 2 < 3, 3 <= 3, 5 > 4, 5 >= 5",
    );
    assert_eq!(compiled.diagnostics, Vec::new());
    let proto = compiled.proto.expect("expected compiled proto");
    let disassembly = disassemble(&proto);

    assert!(disassembly.contains("EQ"));
    assert!(disassembly.contains("LT"));
    assert!(disassembly.contains("LE"));
    assert!(
        disassembly.contains("LOAD_BOOL"),
        "not-equal should invert an equality result through false comparison"
    );
}

#[test]
fn simple_expr_compiles_concat() {
    let compiled = compile_simple_chunk(SourceId::new(0), "return \"a\" .. \"b\"");
    assert_eq!(compiled.diagnostics, Vec::new());
    let proto = compiled.proto.expect("expected compiled proto");

    assert_snapshot_eq(
        disassemble(&proto),
        "0000 LOAD_STRING   A=0 Bx=0 ; \"a\"\n0001 LOAD_STRING   A=1 Bx=1 ; \"b\"\n0002 CONCAT        A=2 B=0 C=1\n0003 RETURN        A=2 B=1 C=0\n",
    );
}

#[test]
fn simple_expr_compiles_binary_without_clobbering_operand_registers() {
    let compiled = compile_simple_chunk(
        SourceId::new(0),
        "local n = 12\nlocal text = 'ab'\nreturn n + 1, n, text .. 'c', text, n == 12",
    );
    assert_eq!(compiled.diagnostics, Vec::new());
    let proto = compiled.proto.expect("expected compiled proto");
    let disassembly = disassemble(&proto);

    assert!(disassembly.contains("ADD_INT"));
    assert!(disassembly.contains("CONCAT"));
    assert!(disassembly.contains("EQ"));
}

#[test]
fn simple_expr_reports_unsupported_statement() {
    let compiled = compile_simple_chunk(SourceId::new(0), "::again::");

    assert!(compiled.proto.is_none());
    assert_eq!(
        compiled.diagnostics[0].message(),
        "unsupported statement in simple expression compiler"
    );
}

#[test]
fn simple_expr_compiles_call_statement() {
    let compiled = compile_simple_chunk(
        SourceId::new(0),
        "local function noop()\n  return 42\nend\nnoop()\nreturn 1",
    );
    assert_eq!(compiled.diagnostics, Vec::new());
    let proto = compiled.proto.expect("expected compiled proto");

    assert!(
        proto
            .code
            .iter()
            .any(|instr| instr.op() == Op::Call && instr.c() == 1),
        "expected a fixed-result call for the call statement"
    );
}

#[test]
fn loops_compile_while_with_break() {
    let compiled = compile_simple_chunk(
        SourceId::new(0),
        "local x = 0\nwhile true do\n  x = x + 1\n  break\nend\nreturn x",
    );
    assert_eq!(compiled.diagnostics, Vec::new());
    let proto = compiled.proto.expect("expected compiled proto");

    assert_snapshot_eq(
        disassemble(&proto),
        "0000 LOAD_K        A=0 Bx=0 ; 0\n0001 MOVE          A=1 B=0 C=0\n0002 LOAD_BOOL     A=2 B=1 C=0\n0003 TEST          A=2 B=0 C=0\n0004 JMP           A=0 sBx=4\n0005 ADD_INT       A=3 B=1 C=1\n0006 MOVE          A=1 B=3 C=0\n0007 JMP           A=0 sBx=1\n0008 JMP           A=0 sBx=-7\n0009 RETURN        A=1 B=1 C=0\n",
    );
}

#[test]
fn loops_compile_repeat_until_condition() {
    let compiled = compile_simple_chunk(
        SourceId::new(0),
        "local x = 0\nrepeat\n  x = x + 1\nuntil true\nreturn x",
    );
    assert_eq!(compiled.diagnostics, Vec::new());
    let proto = compiled.proto.expect("expected compiled proto");

    assert_snapshot_eq(
        disassemble(&proto),
        "0000 LOAD_K        A=0 Bx=0 ; 0\n0001 MOVE          A=1 B=0 C=0\n0002 ADD_INT       A=2 B=1 C=1\n0003 MOVE          A=1 B=2 C=0\n0004 LOAD_BOOL     A=3 B=1 C=0\n0005 TEST          A=3 B=0 C=0\n0006 JMP           A=0 sBx=-5\n0007 RETURN        A=1 B=1 C=0\n",
    );
}

#[test]
fn loops_report_break_outside_loop() {
    let compiled = compile_simple_chunk(SourceId::new(0), "break");

    assert!(compiled.proto.is_none());
    assert_eq!(compiled.diagnostics[0].message(), "break outside loop");
}

#[test]
fn numeric_for_compiles_default_step_loop() {
    let compiled = compile_simple_chunk(
        SourceId::new(0),
        "local sum = 0\nfor i = 1, 3 do\n  sum = sum + i\nend\nreturn sum",
    );
    assert_eq!(compiled.diagnostics, Vec::new());
    let proto = compiled.proto.expect("expected compiled proto");

    assert_snapshot_eq(
        disassemble(&proto),
        "0000 LOAD_K        A=0 Bx=0 ; 0\n0001 MOVE          A=1 B=0 C=0\n0002 LOAD_K        A=5 Bx=1 ; 1\n0003 MOVE          A=2 B=5 C=0\n0004 LOAD_K        A=6 Bx=2 ; 3\n0005 MOVE          A=3 B=6 C=0\n0006 LOAD_K        A=7 Bx=3 ; 1\n0007 MOVE          A=4 B=7 C=0\n0008 FOR_PREP      A=2 sBx=3\n0009 ADD           A=8 B=1 C=4\n0010 MOVE          A=1 B=8 C=0\n0011 FOR_LOOP      A=2 sBx=-3\n0012 RETURN        A=1 B=1 C=0\n",
    );
}

#[test]
fn generic_for_compiles_iterator_protocol() {
    let compiled = compile_simple_chunk(
        SourceId::new(0),
        "local function once()\n  return 1\nend\nfor x in once do\n  return x\nend\nreturn 0",
    );
    assert_eq!(compiled.diagnostics, Vec::new());
    let proto = compiled.proto.expect("expected compiled proto");

    assert_snapshot_eq(
        disassemble(&proto),
        "0000 CLOSURE       A=0 Bx=0\n0001 MOVE          A=1 B=0 C=0\n0002 LOAD_NIL      A=2 B=0 C=0\n0003 LOAD_NIL      A=3 B=0 C=0\n0004 TFOR_PREP     A=1 sBx=1\n0005 RETURN        A=4 B=1 C=0\n0006 TFOR_CALL     A=1 B=0 C=1\n0007 TFOR_LOOP     A=1 sBx=-3\n0008 LOAD_K        A=5 Bx=0 ; 0\n0009 RETURN        A=5 B=1 C=0\n",
    );
}

#[test]
fn generic_for_compiles_call_iterator_triplet_at_loop_base() {
    let compiled = compile_simple_chunk(
        SourceId::new(0),
        "local t = { 10 }\nfor i, v in ipairs(t) do\n  return i + v\nend\nreturn 0",
    );
    assert_eq!(compiled.diagnostics, Vec::new());
    let proto = compiled.proto.expect("expected compiled proto");

    assert_snapshot_eq(
        disassemble(&proto),
        "0000 NEW_TABLE     A=0 B=1 C=0\n0001 LOAD_K        A=1 Bx=0 ; 1\n0002 LOAD_K        A=2 Bx=1 ; 10\n0003 SET_TABLE     A=0 B=1 C=2\n0004 MOVE          A=3 B=0 C=0\n0005 GET_UPVALUE   A=4 B=0 C=0\n0006 LOAD_STRING   A=5 Bx=0 ; \"ipairs\"\n0007 GET_TABLE     A=6 B=4 C=5\n0008 MOVE          A=4 B=6 C=0\n0009 MOVE          A=5 B=3 C=0\n0010 CALL          A=4 B=2 C=3\n0011 TFOR_PREP     A=4 sBx=2\n0012 ADD           A=9 B=7 C=8\n0013 RETURN        A=9 B=1 C=0\n0014 TFOR_CALL     A=4 B=0 C=2\n0015 TFOR_LOOP     A=4 sBx=-4\n0016 LOAD_K        A=10 Bx=2 ; 0\n0017 RETURN        A=10 B=1 C=0\n",
    );
}

#[test]
fn table_constructor_compiles_array_record_and_keyed_fields() {
    let compiled = compile_simple_chunk(SourceId::new(0), "return { 1, named = 2, [3] = 4 }");
    assert_eq!(compiled.diagnostics, Vec::new());
    let proto = compiled.proto.expect("expected compiled proto");

    assert_snapshot_eq(
        disassemble(&proto),
        "0000 NEW_TABLE     A=0 B=1 C=2\n0001 LOAD_K        A=1 Bx=0 ; 1\n0002 LOAD_K        A=2 Bx=1 ; 1\n0003 SET_TABLE     A=0 B=1 C=2\n0004 LOAD_STRING   A=3 Bx=0 ; \"named\"\n0005 LOAD_K        A=4 Bx=2 ; 2\n0006 SET_TABLE     A=0 B=3 C=4\n0007 LOAD_K        A=5 Bx=3 ; 3\n0008 LOAD_K        A=6 Bx=4 ; 4\n0009 SET_TABLE     A=0 B=5 C=6\n0010 RETURN        A=0 B=1 C=0\n",
    );
}

#[test]
fn table_constructor_compiles_final_open_array_field_with_set_list() {
    let compiled = compile_simple_chunk(
        SourceId::new(0),
        "local function f() return 2, 3 end\nreturn { 1, f() }",
    );
    assert_eq!(compiled.diagnostics, Vec::new());
    let proto = compiled.proto.expect("expected compiled proto");

    assert!(
        proto
            .code
            .iter()
            .any(|instr| instr.op() == Op::SetList && instr.b() == 2)
    );
}

#[test]
fn table_access_compiles_index_read_and_write() {
    let compiled = compile_simple_chunk(SourceId::new(0), "local t = {}\nt[1] = 42\nreturn t[1]");
    assert_eq!(compiled.diagnostics, Vec::new());
    let proto = compiled.proto.expect("expected compiled proto");

    assert_snapshot_eq(
        disassemble(&proto),
        "0000 NEW_TABLE     A=0 B=0 C=0\n0001 MOVE          A=1 B=0 C=0\n0002 LOAD_K        A=2 Bx=0 ; 42\n0003 LOAD_K        A=3 Bx=1 ; 1\n0004 SET_TABLE     A=1 B=3 C=2\n0005 LOAD_K        A=4 Bx=2 ; 1\n0006 GET_TABLE     A=5 B=1 C=4\n0007 RETURN        A=5 B=1 C=0\n",
    );
}

#[test]
fn table_access_compiles_field_read_and_write() {
    let compiled = compile_simple_chunk(
        SourceId::new(0),
        "local t = {}\nt.answer = 42\nreturn t.answer",
    );
    assert_eq!(compiled.diagnostics, Vec::new());
    let proto = compiled.proto.expect("expected compiled proto");

    assert_snapshot_eq(
        disassemble(&proto),
        "0000 NEW_TABLE     A=0 B=0 C=0\n0001 MOVE          A=1 B=0 C=0\n0002 LOAD_K        A=2 Bx=0 ; 42\n0003 LOAD_STRING   A=3 Bx=0 ; \"answer\"\n0004 SET_TABLE     A=1 B=3 C=2\n0005 LOAD_STRING   A=4 Bx=1 ; \"answer\"\n0006 GET_TABLE     A=5 B=1 C=4\n0007 RETURN        A=5 B=1 C=0\n",
    );
}

#[test]
fn globals_compile_declaration_assignment_and_read() {
    let compiled = compile_simple_chunk(
        SourceId::new(0),
        "global answer = 41\nanswer = answer + 1\nreturn answer",
    );
    assert_eq!(compiled.diagnostics, Vec::new());
    let proto = compiled.proto.expect("expected compiled proto");

    assert_snapshot_eq(
        disassemble(&proto),
        "0000 LOAD_K        A=0 Bx=0 ; 41\n0001 GET_UPVALUE   A=1 B=0 C=0\n0002 LOAD_STRING   A=2 Bx=0 ; \"answer\"\n0003 GET_TABLE     A=3 B=1 C=2\n0004 DECL_GLOBAL   A=3 Bx=1 ; \"answer\"\n0005 GET_UPVALUE   A=4 B=0 C=0\n0006 LOAD_STRING   A=5 Bx=2 ; \"answer\"\n0007 SET_TABLE     A=4 B=5 C=0\n0008 GET_UPVALUE   A=6 B=0 C=0\n0009 LOAD_STRING   A=7 Bx=3 ; \"answer\"\n0010 GET_TABLE     A=8 B=6 C=7\n0011 ADD_INT       A=9 B=8 C=1\n0012 GET_UPVALUE   A=10 B=0 C=0\n0013 LOAD_STRING   A=11 Bx=4 ; \"answer\"\n0014 SET_TABLE     A=10 B=11 C=9\n0015 GET_UPVALUE   A=12 B=0 C=0\n0016 LOAD_STRING   A=13 Bx=5 ; \"answer\"\n0017 GET_TABLE     A=14 B=12 C=13\n0018 RETURN        A=14 B=1 C=0\n",
    );
}

#[test]
fn globals_compile_implicit_preambular_global_access() {
    let compiled = compile_simple_chunk(SourceId::new(0), "answer = 42\nreturn answer");
    assert_eq!(compiled.diagnostics, Vec::new());
    let proto = compiled.proto.expect("expected compiled proto");

    assert_eq!(proto.upvalues.len(), 1);
    assert_eq!(proto.upvalues[0].name.as_deref(), Some("_ENV"));
    assert_snapshot_eq(
        disassemble(&proto),
        "0000 LOAD_K        A=0 Bx=0 ; 42\n0001 GET_UPVALUE   A=1 B=0 C=0\n0002 LOAD_STRING   A=2 Bx=0 ; \"answer\"\n0003 SET_TABLE     A=1 B=2 C=0\n0004 GET_UPVALUE   A=3 B=0 C=0\n0005 LOAD_STRING   A=4 Bx=1 ; \"answer\"\n0006 GET_TABLE     A=5 B=3 C=4\n0007 RETURN        A=5 B=1 C=0\n",
    );
}

#[test]
fn globals_compile_root_env_as_default_upvalue() {
    let compiled = compile_simple_chunk(SourceId::new(0), "return _ENV");
    assert_eq!(compiled.diagnostics, Vec::new());
    let proto = compiled.proto.expect("expected compiled proto");

    assert_eq!(proto.upvalues.len(), 1);
    assert_eq!(proto.upvalues[0].name.as_deref(), Some("_ENV"));
    assert!(!proto.upvalues[0].in_stack);
    assert_eq!(proto.upvalues[0].index, 0);
    assert_snapshot_eq(
        disassemble(&proto),
        "0000 GET_UPVALUE   A=0 B=0 C=0\n0001 RETURN        A=0 B=1 C=0\n",
    );
}

#[test]
fn globals_report_undeclared_name_after_declaration() {
    let compiled = compile_simple_chunk(SourceId::new(0), "global answer\nreturn missing");

    assert!(compiled.proto.is_none());
    assert_eq!(
        compiled.diagnostics[0].message(),
        "variable 'missing' not declared"
    );
}

#[test]
fn globals_report_read_only_collective_assignment() {
    let compiled = compile_simple_chunk(SourceId::new(0), "global<const> *\nanswer = 42");

    assert!(compiled.proto.is_none());
    assert_eq!(
        compiled.diagnostics[0].message(),
        "global variable 'answer' is read-only"
    );
}

#[test]
fn globals_report_read_only_named_assignment() {
    let compiled = compile_simple_chunk(SourceId::new(0), "global answer<const>\nanswer = 42");

    assert!(compiled.proto.is_none());
    assert_eq!(
        compiled.diagnostics[0].message(),
        "global variable 'answer' is read-only"
    );
}

#[test]
fn globals_compile_reads_through_local_env() {
    let compiled = compile_simple_chunk(
        SourceId::new(0),
        "local _ENV = { answer = 42 }\nreturn answer",
    );
    assert_eq!(compiled.diagnostics, Vec::new());
    let proto = compiled.proto.expect("expected compiled proto");
    let disassembly = disassemble(&proto);

    assert!(disassembly.contains("GET_TABLE"));
    assert!(!disassembly.contains("GET_ENV"));
}

#[test]
fn globals_compile_writes_through_local_env() {
    let compiled = compile_simple_chunk(
        SourceId::new(0),
        "local _ENV = {}\nanswer = 42\nreturn _ENV.answer",
    );
    assert_eq!(compiled.diagnostics, Vec::new());
    let proto = compiled.proto.expect("expected compiled proto");
    let disassembly = disassemble(&proto);

    assert!(disassembly.contains("SET_TABLE"));
    assert!(!disassembly.contains("SET_ENV"));
}

#[test]
fn globals_compile_global_function_declaration() {
    let compiled = compile_simple_chunk(
        SourceId::new(0),
        "global function answer()\n  return 42\nend\nreturn answer()",
    );
    assert_eq!(compiled.diagnostics, Vec::new());
    let proto = compiled.proto.expect("expected compiled proto");
    let disassembly = disassemble(&proto);

    assert!(disassembly.contains("CLOSURE"));
    assert!(disassembly.contains("DECL_GLOBAL"));
    assert!(disassembly.contains("SET_TABLE"));
    assert_eq!(proto.children.len(), 1);
}

#[test]
fn globals_nested_if_declaration_does_not_escape_block() {
    let compiled = compile_simple_chunk(
        SourceId::new(0),
        "global answer\nif true then\n  global hidden = 42\nend\nreturn hidden",
    );

    assert!(compiled.proto.is_none());
    assert_eq!(
        compiled.diagnostics[0].message(),
        "variable 'hidden' not declared"
    );
}

#[test]
fn globals_nested_loop_declaration_does_not_escape_block() {
    let compiled = compile_simple_chunk(
        SourceId::new(0),
        "global answer\nwhile false do\n  global hidden = 42\nend\nreturn hidden",
    );

    assert!(compiled.proto.is_none());
    assert_eq!(
        compiled.diagnostics[0].message(),
        "variable 'hidden' not declared"
    );
}

#[test]
fn globals_inner_declaration_shadows_collective_read_only_scope() {
    let compiled = compile_simple_chunk(
        SourceId::new(0),
        "global<const> *\nif true then\n  global answer\n  answer = 42\nend\nanswer = 99",
    );

    assert!(compiled.proto.is_none());
    assert_eq!(
        compiled.diagnostics[0].message(),
        "global variable 'answer' is read-only"
    );
}

#[test]
fn globals_report_global_env_on_read() {
    let compiled = compile_simple_chunk(SourceId::new(0), "global _ENV, answer\nreturn answer");

    assert!(compiled.proto.is_none());
    assert_eq!(
        compiled.diagnostics[0].message(),
        "_ENV is global when accessing variable 'answer'"
    );
}

#[test]
fn globals_report_global_env_on_write() {
    let compiled = compile_simple_chunk(SourceId::new(0), "global _ENV, answer\nanswer = 42");

    assert!(compiled.proto.is_none());
    assert_eq!(
        compiled.diagnostics[0].message(),
        "_ENV is global when accessing variable 'answer'"
    );
}

#[test]
fn locals_compile_local_return() {
    let compiled = compile_simple_chunk(SourceId::new(0), "local x = 1 + 2\nreturn x");
    assert_eq!(compiled.diagnostics, Vec::new());
    let proto = compiled.proto.expect("expected compiled proto");

    assert_eq!(proto.constants.len(), 1);
    assert_eq!(proto.code.last().map(|instr| instr.op()), Some(Op::Return));
    assert!(disassemble(&proto).contains("MOVE"));
    assert_eq!(
        proto.debug.local_vars.as_ref(),
        [LocalVarDesc::new("x", 2, 3, u32::MAX)]
    );
}

#[test]
fn locals_compile_assignment() {
    let compiled = compile_simple_chunk(SourceId::new(0), "local x = 1\nx = x + 2\nreturn x");
    assert_eq!(compiled.diagnostics, Vec::new());
    let proto = compiled.proto.expect("expected compiled proto");

    assert_snapshot_eq(
        disassemble(&proto),
        "0000 LOAD_K        A=0 Bx=0 ; 1\n0001 MOVE          A=1 B=0 C=0\n0002 ADD_INT       A=2 B=1 C=2\n0003 MOVE          A=1 B=2 C=0\n0004 RETURN        A=1 B=1 C=0\n",
    );
}

#[test]
fn locals_compile_to_be_closed_local() {
    let compiled = compile_simple_chunk(SourceId::new(0), "local<close> x = nil\nreturn 42");
    assert_eq!(compiled.diagnostics, Vec::new());
    let proto = compiled.proto.expect("expected compiled proto");

    assert_snapshot_eq(
        disassemble(&proto),
        "0000 LOAD_NIL      A=0 B=0 C=0\n0001 MOVE          A=1 B=0 C=0\n0002 TBC           A=1 B=0 C=0\n0003 LOAD_K        A=2 Bx=0 ; 42\n0004 CLOSE         A=1 B=0 C=0\n0005 RETURN        A=2 B=1 C=0\n",
    );
    assert_eq!(
        proto.debug.local_vars.as_ref(),
        [LocalVarDesc::new("x", 1, 3, u32::MAX)]
    );
}

#[test]
fn globals_report_undeclared_assignment_after_declaration() {
    let compiled = compile_simple_chunk(SourceId::new(0), "global answer\nmissing = 1");

    assert!(compiled.proto.is_none());
    assert_eq!(
        compiled.diagnostics[0].message(),
        "variable 'missing' not declared"
    );
}

#[test]
fn functions_compile_local_function_call() {
    let compiled = compile_simple_chunk(
        SourceId::new(0),
        "local function answer()\n  return 42\nend\nreturn answer()",
    );
    assert_eq!(compiled.diagnostics, Vec::new());
    let proto = compiled.proto.expect("expected compiled proto");

    assert_eq!(proto.children.len(), 1);
    assert_snapshot_eq(
        disassemble(&proto),
        "0000 CLOSURE       A=0 Bx=0\n0001 MOVE          A=1 B=0 C=0\n0002 CALL          A=1 B=1 C=0\n0003 RETURN        A=1 B=0 C=0\n",
    );
}

#[test]
fn functions_preserve_local_callable_across_assignment_calls() {
    let compiled = compile_simple_chunk(
        SourceId::new(0),
        "local function answer()\n  return 42\nend\nlocal a = answer()\nlocal b = answer()\nreturn a + b",
    );
    assert_eq!(compiled.diagnostics, Vec::new());
    let proto = compiled.proto.expect("expected compiled proto");

    assert_eq!(proto.children.len(), 1);
    assert_snapshot_eq(
        disassemble(&proto),
        "0000 CLOSURE       A=0 Bx=0\n0001 MOVE          A=1 B=0 C=0\n0002 CALL          A=1 B=1 C=1\n0003 MOVE          A=2 B=1 C=0\n0004 MOVE          A=3 B=0 C=0\n0005 CALL          A=3 B=1 C=1\n0006 MOVE          A=4 B=3 C=0\n0007 ADD           A=5 B=2 C=4\n0008 RETURN        A=5 B=1 C=0\n",
    );
}

#[test]
fn functions_expand_final_call_for_local_declarations() {
    let compiled = compile_simple_chunk(
        SourceId::new(0),
        "local function values()\n  return 10, 20\nend\nlocal a, b = values()\nreturn a, b",
    );
    assert_eq!(compiled.diagnostics, Vec::new());
    let proto = compiled.proto.expect("expected compiled proto");

    assert_eq!(proto.children.len(), 1);
    assert_snapshot_eq(
        disassemble(&proto),
        "0000 CLOSURE       A=0 Bx=0\n0001 MOVE          A=1 B=0 C=0\n0002 CALL          A=1 B=1 C=2\n0003 MOVE          A=3 B=1 C=0\n0004 MOVE          A=4 B=2 C=0\n0005 RETURN        A=3 B=2 C=0\n",
    );
}

#[test]
fn functions_preserve_fixed_call_results_with_argument_registers() {
    let compiled = compile_simple_chunk(
        SourceId::new(0),
        "local values = {}\nlocal _ = setmetatable(values, { __metatable = 'locked' })\nlocal ok, message = pcall(setmetatable, values, {})\nreturn ok, string.byte(type(message), 1)",
    );
    assert_eq!(compiled.diagnostics, Vec::new());
    let proto = compiled.proto.expect("expected compiled proto");

    assert_snapshot_eq(
        disassemble(&proto),
        "0000 NEW_TABLE     A=0 B=0 C=0\n0001 MOVE          A=1 B=0 C=0\n0002 GET_UPVALUE   A=3 B=0 C=0\n0003 LOAD_STRING   A=4 Bx=0 ; \"setmetatable\"\n0004 GET_TABLE     A=5 B=3 C=4\n0005 MOVE          A=2 B=5 C=0\n0006 MOVE          A=3 B=1 C=0\n0007 NEW_TABLE     A=6 B=0 C=1\n0008 LOAD_STRING   A=7 Bx=1 ; \"__metatable\"\n0009 LOAD_STRING   A=8 Bx=2 ; \"locked\"\n0010 SET_TABLE     A=6 B=7 C=8\n0011 MOVE          A=4 B=6 C=0\n0012 CALL          A=2 B=3 C=1\n0013 MOVE          A=9 B=2 C=0\n0014 GET_UPVALUE   A=12 B=0 C=0\n0015 LOAD_STRING   A=13 Bx=3 ; \"pcall\"\n0016 GET_TABLE     A=14 B=12 C=13\n0017 MOVE          A=10 B=14 C=0\n0018 GET_UPVALUE   A=15 B=0 C=0\n0019 LOAD_STRING   A=16 Bx=4 ; \"setmetatable\"\n0020 GET_TABLE     A=17 B=15 C=16\n0021 MOVE          A=11 B=17 C=0\n0022 MOVE          A=12 B=1 C=0\n0023 NEW_TABLE     A=18 B=0 C=0\n0024 MOVE          A=13 B=18 C=0\n0025 CALL          A=10 B=4 C=2\n0026 MOVE          A=19 B=10 C=0\n0027 MOVE          A=20 B=11 C=0\n0028 MOVE          A=21 B=19 C=0\n0029 GET_UPVALUE   A=23 B=0 C=0\n0030 LOAD_STRING   A=24 Bx=5 ; \"string\"\n0031 GET_TABLE     A=25 B=23 C=24\n0032 LOAD_STRING   A=26 Bx=6 ; \"byte\"\n0033 GET_TABLE     A=27 B=25 C=26\n0034 MOVE          A=22 B=27 C=0\n0035 GET_UPVALUE   A=28 B=0 C=0\n0036 LOAD_STRING   A=29 Bx=7 ; \"type\"\n0037 GET_TABLE     A=30 B=28 C=29\n0038 MOVE          A=31 B=30 C=0\n0039 MOVE          A=32 B=20 C=0\n0040 CALL          A=31 B=2 C=1\n0041 MOVE          A=23 B=31 C=0\n0042 LOAD_K        A=33 Bx=0 ; 1\n0043 MOVE          A=24 B=33 C=0\n0044 CALL          A=22 B=3 C=0\n0045 RETURN        A=21 B=0 C=0\n",
    );
}

#[test]
fn functions_compile_recursive_self_reference() {
    let compiled = compile_simple_chunk(
        SourceId::new(0),
        "local function self()\n  return self\nend\nreturn self()",
    );
    assert_eq!(compiled.diagnostics, Vec::new());
    let proto = compiled.proto.expect("expected compiled proto");

    assert_eq!(proto.children.len(), 1);
    assert_eq!(proto.children[0].upvalues.len(), 1);
    assert_snapshot_eq(
        disassemble(&proto.children[0]),
        "0000 GET_UPVALUE   A=0 B=0 C=0\n0001 RETURN        A=0 B=1 C=0\n",
    );
}

#[test]
fn functions_compile_fixed_parameters() {
    let compiled = compile_simple_chunk(SourceId::new(0), "local function id(x) return x end");

    assert_eq!(compiled.diagnostics, Vec::new());
    let proto = compiled.proto.expect("expected compiled proto");

    assert_eq!(proto.children.len(), 1);
    assert_eq!(proto.children[0].params, 1);
    assert_snapshot_eq(disassemble(&proto.children[0]), "0000 RETURN        A=0 B=1 C=0\n");
}

#[test]
fn functions_compile_fixed_parameters_with_named_varargs() {
    let compiled = compile_simple_chunk(
        SourceId::new(0),
        "local function collect(first, ... rest)\n  return first, rest\nend",
    );

    assert_eq!(compiled.diagnostics, Vec::new());
    let proto = compiled.proto.expect("expected compiled proto");

    assert_eq!(proto.children.len(), 1);
    assert_eq!(proto.children[0].params, 1);
    assert!(proto.children[0].is_vararg);
    assert_snapshot_eq(
        disassemble(&proto.children[0]),
        "0000 VARARG_TABLE  A=1 B=0 C=0\n0001 RETURN        A=0 B=2 C=0\n",
    );
}

#[test]
fn closures_compile_outer_local_capture() {
    let compiled = compile_simple_chunk(
        SourceId::new(0),
        "local x = 41\nlocal function answer()\n  return x + 1\nend\nreturn answer()",
    );
    assert_eq!(compiled.diagnostics, Vec::new());
    let proto = compiled.proto.expect("expected compiled proto");

    assert_eq!(proto.children.len(), 1);
    assert_eq!(proto.children[0].upvalues.len(), 1);
    assert_snapshot_eq(
        disassemble(&proto.children[0]),
        "0000 GET_UPVALUE   A=0 B=0 C=0\n0001 ADD_INT       A=1 B=0 C=1\n0002 RETURN        A=1 B=1 C=0\n",
    );
}

#[test]
fn conditionals_compile_if_else_branches() {
    let compiled =
        compile_simple_chunk(SourceId::new(0), "if false then return 1 else return 2 end");
    assert_eq!(compiled.diagnostics, Vec::new());
    let proto = compiled.proto.expect("expected compiled proto");

    assert_snapshot_eq(
        disassemble(&proto),
        "0000 LOAD_BOOL     A=0 B=0 C=0\n0001 TEST          A=0 B=0 C=0\n0002 JMP           A=0 sBx=3\n0003 LOAD_K        A=1 Bx=0 ; 1\n0004 RETURN        A=1 B=1 C=0\n0005 JMP           A=0 sBx=2\n0006 LOAD_K        A=2 Bx=1 ; 2\n0007 RETURN        A=2 B=1 C=0\n",
    );
}

#[test]
fn varargs_compile_anonymous_vararg_call() {
    let compiled = compile_simple_chunk(
        SourceId::new(0),
        "local function first(...)\n  return ...\nend\nreturn first(42)",
    );
    assert_eq!(compiled.diagnostics, Vec::new());
    let proto = compiled.proto.expect("expected compiled proto");

    assert_eq!(proto.children.len(), 1);
    assert!(proto.children[0].is_vararg);
    assert_snapshot_eq(
        disassemble(&proto.children[0]),
        "0000 VARARG        A=0 B=0 C=0\n0001 RETURN        A=0 B=0 C=0\n",
    );
    assert_snapshot_eq(
        disassemble(&proto),
        "0000 CLOSURE       A=0 Bx=0\n0001 MOVE          A=1 B=0 C=0\n0002 LOAD_K        A=2 Bx=0 ; 42\n0003 CALL          A=1 B=2 C=0\n0004 RETURN        A=1 B=0 C=0\n",
    );
}

#[test]
fn varargs_compile_named_vararg_table() {
    let compiled = compile_simple_chunk(
        SourceId::new(0),
        "local function args(... rest)\n  return rest\nend\nreturn args(42)",
    );
    assert_eq!(compiled.diagnostics, Vec::new());
    let proto = compiled.proto.expect("expected compiled proto");

    assert_eq!(proto.children.len(), 1);
    assert!(proto.children[0].is_vararg);
    assert_snapshot_eq(
        disassemble(&proto.children[0]),
        "0000 VARARG_TABLE  A=0 B=0 C=0\n0001 RETURN        A=0 B=1 C=0\n",
    );
    assert_snapshot_eq(
        disassemble(&proto),
        "0000 CLOSURE       A=0 Bx=0\n0001 MOVE          A=1 B=0 C=0\n0002 LOAD_K        A=2 Bx=0 ; 42\n0003 CALL          A=1 B=2 C=0\n0004 RETURN        A=1 B=0 C=0\n",
    );
}
