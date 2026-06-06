use elara_bytecode::{Op, disassemble};
use elara_core::SourceId;
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
        "0000 LOAD_K        A=0 Bx=0 ; 1\n0001 LOAD_K        A=1 Bx=1 ; 2\n0002 LOAD_K        A=2 Bx=2 ; 3\n0003 MUL           A=1 B=1 C=2\n0004 ADD           A=0 B=0 C=1\n0005 RETURN        A=0 B=1 C=0\n",
    );
}

#[test]
fn simple_expr_compiles_unary_arithmetic() {
    let compiled = compile_simple_chunk(SourceId::new(0), "return -1");
    assert_eq!(compiled.diagnostics, Vec::new());
    let proto = compiled.proto.expect("expected compiled proto");

    assert_snapshot_eq(
        disassemble(&proto),
        "0000 LOAD_K        A=0 Bx=0 ; 1\n0001 UNM           A=0 B=0 C=0\n0002 RETURN        A=0 B=1 C=0\n",
    );
}

#[test]
fn simple_expr_compiles_concat() {
    let compiled = compile_simple_chunk(SourceId::new(0), "return \"a\" .. \"b\"");
    assert_eq!(compiled.diagnostics, Vec::new());
    let proto = compiled.proto.expect("expected compiled proto");

    assert_snapshot_eq(
        disassemble(&proto),
        "0000 LOAD_STRING   A=0 Bx=0 ; \"a\"\n0001 LOAD_STRING   A=1 Bx=1 ; \"b\"\n0002 CONCAT        A=0 B=0 C=1\n0003 RETURN        A=0 B=1 C=0\n",
    );
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
fn loops_compile_while_with_break() {
    let compiled = compile_simple_chunk(
        SourceId::new(0),
        "local x = 0\nwhile true do\n  x = x + 1\n  break\nend\nreturn x",
    );
    assert_eq!(compiled.diagnostics, Vec::new());
    let proto = compiled.proto.expect("expected compiled proto");

    assert_snapshot_eq(
        disassemble(&proto),
        "0000 LOAD_K        A=0 Bx=0 ; 0\n0001 MOVE          A=1 B=0 C=0\n0002 LOAD_BOOL     A=2 B=1 C=0\n0003 TEST          A=2 B=0 C=0\n0004 JMP           A=0 sBx=4\n0005 LOAD_K        A=3 Bx=1 ; 1\n0006 ADD           A=1 B=1 C=3\n0007 JMP           A=0 sBx=1\n0008 JMP           A=0 sBx=-7\n0009 RETURN        A=1 B=1 C=0\n",
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
        "0000 LOAD_K        A=0 Bx=0 ; 0\n0001 MOVE          A=1 B=0 C=0\n0002 LOAD_K        A=2 Bx=1 ; 1\n0003 ADD           A=1 B=1 C=2\n0004 LOAD_BOOL     A=3 B=1 C=0\n0005 TEST          A=3 B=0 C=0\n0006 JMP           A=0 sBx=-5\n0007 RETURN        A=1 B=1 C=0\n",
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
        "0000 LOAD_K        A=0 Bx=0 ; 0\n0001 MOVE          A=1 B=0 C=0\n0002 LOAD_K        A=5 Bx=1 ; 1\n0003 MOVE          A=2 B=5 C=0\n0004 LOAD_K        A=6 Bx=2 ; 3\n0005 MOVE          A=3 B=6 C=0\n0006 LOAD_K        A=7 Bx=3 ; 1\n0007 MOVE          A=4 B=7 C=0\n0008 FOR_PREP      A=2 sBx=2\n0009 ADD           A=1 B=1 C=4\n0010 FOR_LOOP      A=2 sBx=-2\n0011 RETURN        A=1 B=1 C=0\n",
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
        "0000 LOAD_K        A=0 Bx=0 ; 41\n0001 GET_ENV       A=1 Bx=0 ; \"answer\"\n0002 DECL_GLOBAL   A=1 Bx=1 ; \"answer\"\n0003 SET_ENV       A=0 Bx=2 ; \"answer\"\n0004 GET_ENV       A=2 Bx=3 ; \"answer\"\n0005 LOAD_K        A=3 Bx=1 ; 1\n0006 ADD           A=2 B=2 C=3\n0007 SET_ENV       A=2 Bx=4 ; \"answer\"\n0008 GET_ENV       A=4 Bx=5 ; \"answer\"\n0009 RETURN        A=4 B=1 C=0\n",
    );
}

#[test]
fn globals_compile_implicit_preambular_global_access() {
    let compiled = compile_simple_chunk(SourceId::new(0), "answer = 42\nreturn answer");
    assert_eq!(compiled.diagnostics, Vec::new());
    let proto = compiled.proto.expect("expected compiled proto");

    assert_snapshot_eq(
        disassemble(&proto),
        "0000 LOAD_K        A=0 Bx=0 ; 42\n0001 SET_ENV       A=0 Bx=0 ; \"answer\"\n0002 GET_ENV       A=1 Bx=1 ; \"answer\"\n0003 RETURN        A=1 B=1 C=0\n",
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
fn locals_compile_local_return() {
    let compiled = compile_simple_chunk(SourceId::new(0), "local x = 1 + 2\nreturn x");
    assert_eq!(compiled.diagnostics, Vec::new());
    let proto = compiled.proto.expect("expected compiled proto");

    assert_eq!(proto.constants.len(), 2);
    assert_eq!(proto.code.last().map(|instr| instr.op()), Some(Op::Return));
    assert!(disassemble(&proto).contains("MOVE"));
}

#[test]
fn locals_compile_assignment() {
    let compiled = compile_simple_chunk(SourceId::new(0), "local x = 1\nx = x + 2\nreturn x");
    assert_eq!(compiled.diagnostics, Vec::new());
    let proto = compiled.proto.expect("expected compiled proto");

    assert_snapshot_eq(
        disassemble(&proto),
        "0000 LOAD_K        A=0 Bx=0 ; 1\n0001 MOVE          A=1 B=0 C=0\n0002 LOAD_K        A=2 Bx=1 ; 2\n0003 ADD           A=1 B=1 C=2\n0004 RETURN        A=1 B=1 C=0\n",
    );
}

#[test]
fn locals_report_unknown_assignment_target() {
    let compiled = compile_simple_chunk(SourceId::new(0), "x = 1\nreturn x");

    assert!(compiled.proto.is_none());
    assert_eq!(
        compiled.diagnostics[0].message(),
        "assignment target is not a declared local"
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
        "0000 CLOSURE       A=0 Bx=0\n0001 CALL          A=0 B=1 C=1\n0002 RETURN        A=0 B=1 C=0\n",
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
fn functions_reject_parameters_for_now() {
    let compiled = compile_simple_chunk(SourceId::new(0), "local function id(x) return x end");

    assert!(compiled.proto.is_none());
    assert_eq!(
        compiled.diagnostics[0].message(),
        "function parameters are not supported yet"
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
        "0000 GET_UPVALUE   A=0 B=0 C=0\n0001 LOAD_K        A=1 Bx=0 ; 1\n0002 ADD           A=0 B=0 C=1\n0003 RETURN        A=0 B=1 C=0\n",
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
        "0000 VARARG        A=0 B=1 C=0\n0001 RETURN        A=0 B=1 C=0\n",
    );
    assert_snapshot_eq(
        disassemble(&proto),
        "0000 CLOSURE       A=0 Bx=0\n0001 LOAD_K        A=1 Bx=0 ; 42\n0002 CALL          A=0 B=2 C=1\n0003 RETURN        A=0 B=1 C=0\n",
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
        "0000 CLOSURE       A=0 Bx=0\n0001 LOAD_K        A=1 Bx=0 ; 42\n0002 CALL          A=0 B=2 C=1\n0003 RETURN        A=0 B=1 C=0\n",
    );
}
