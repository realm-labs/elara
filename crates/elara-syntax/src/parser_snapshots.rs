use elara_core::SourceId;
use elara_test::{SnapshotKind, assert_snapshot_eq, format_diagnostics_snapshot, snapshot_path};

use crate::parse_chunk;

fn parse_fixture(source: &str) -> crate::ParsedChunk<'_> {
    parse_chunk(SourceId::new(0), source)
}

#[test]
fn parser_snapshots_representative_chunk_ast() {
    let fixture_path = "tests/fixtures/pass/parser_control.lua";
    let source = include_str!("../../../tests/fixtures/pass/parser_control.lua");
    let expected = include_str!("../../../tests/snapshots/pass/parser_control.ast.snap");
    let parsed = parse_fixture(source);

    assert_eq!(parsed.diagnostics, Vec::new());
    assert_eq!(
        snapshot_path(fixture_path, SnapshotKind::Ast),
        std::path::PathBuf::from("tests/snapshots/pass/parser_control.ast.snap")
    );
    assert_snapshot_eq(format!("{:#?}", parsed.block), expected);
}

#[test]
fn parser_snapshots_malformed_chunk_diagnostics() {
    let fixture_path = "tests/fixtures/fail/missing_end.lua";
    let source = include_str!("../../../tests/fixtures/fail/missing_end.lua");
    let expected = include_str!("../../../tests/snapshots/fail/missing_end.diagnostics.snap");
    let parsed = parse_fixture(source);

    assert!(!parsed.diagnostics.is_empty());
    assert_eq!(
        snapshot_path(fixture_path, SnapshotKind::Diagnostics),
        std::path::PathBuf::from("tests/snapshots/fail/missing_end.diagnostics.snap")
    );
    assert_snapshot_eq(format_diagnostics_snapshot(&parsed.diagnostics), expected);
}
