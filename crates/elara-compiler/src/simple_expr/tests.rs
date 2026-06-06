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
fn simple_expr_reports_unsupported_statement() {
    let compiled = compile_simple_chunk(SourceId::new(0), "x = 1");

    assert!(compiled.proto.is_none());
    assert_eq!(
        compiled.diagnostics[0].message(),
        "unsupported statement in simple expression compiler"
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
