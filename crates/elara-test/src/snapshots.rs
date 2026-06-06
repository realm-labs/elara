//! Snapshot helpers for Elara tests.

use std::{
    ffi::OsStr,
    path::{Path, PathBuf},
};

use elara_core::Diagnostic;

/// Snapshot category for compiler pipeline artifacts.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum SnapshotKind {
    /// Parsed AST snapshot.
    Ast,
    /// Lowered bytecode snapshot.
    Bytecode,
    /// Diagnostic output snapshot.
    Diagnostics,
}

impl SnapshotKind {
    /// File-name suffix for this snapshot category.
    #[must_use]
    pub const fn suffix(self) -> &'static str {
        match self {
            Self::Ast => "ast",
            Self::Bytecode => "bytecode",
            Self::Diagnostics => "diagnostics",
        }
    }
}

/// Returns the conventional snapshot path for a fixture and artifact kind.
#[must_use]
pub fn snapshot_path(fixture_path: impl AsRef<Path>, kind: SnapshotKind) -> PathBuf {
    let fixture_path = fixture_path.as_ref();
    let fixture_group = fixture_path
        .parent()
        .and_then(Path::file_name)
        .unwrap_or_else(|| OsStr::new("unknown"));
    let fixture_stem = fixture_path
        .file_stem()
        .and_then(OsStr::to_str)
        .unwrap_or("fixture");

    PathBuf::from("tests")
        .join("snapshots")
        .join(fixture_group)
        .join(format!("{fixture_stem}.{}.snap", kind.suffix()))
}

/// Normalizes snapshot text for stable comparisons.
#[must_use]
pub fn normalize_snapshot_text(text: impl AsRef<str>) -> String {
    let mut normalized = text.as_ref().replace("\r\n", "\n");
    if !normalized.ends_with('\n') {
        normalized.push('\n');
    }
    normalized
}

/// Asserts that two snapshot strings match after normalization.
///
/// # Panics
///
/// Panics when the normalized strings differ.
pub fn assert_snapshot_eq(actual: impl AsRef<str>, expected: impl AsRef<str>) {
    let actual = normalize_snapshot_text(actual);
    let expected = normalize_snapshot_text(expected);

    assert_eq!(expected, actual, "snapshot mismatch");
}

/// Formats diagnostics for snapshot comparison.
#[must_use]
pub fn format_diagnostics_snapshot(diagnostics: &[Diagnostic]) -> String {
    if diagnostics.is_empty() {
        return "ok\n".to_owned();
    }

    let mut output = String::new();
    for diagnostic in diagnostics {
        output.push_str(&diagnostic.to_string());
        output.push('\n');
    }
    output
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use elara_core::{Diagnostic, DiagnosticLabel, SourceId, Span};

    use super::{
        SnapshotKind, assert_snapshot_eq, format_diagnostics_snapshot, normalize_snapshot_text,
        snapshot_path,
    };

    #[test]
    fn snapshots_build_paths_from_fixture_group_and_kind() {
        let fixture_path = "tests/fixtures/pass/return_42.lua";

        assert_eq!(
            snapshot_path(fixture_path, SnapshotKind::Ast),
            PathBuf::from("tests/snapshots/pass/return_42.ast.snap")
        );
        assert_eq!(
            snapshot_path(fixture_path, SnapshotKind::Bytecode),
            PathBuf::from("tests/snapshots/pass/return_42.bytecode.snap")
        );
        assert_eq!(
            snapshot_path(fixture_path, SnapshotKind::Diagnostics),
            PathBuf::from("tests/snapshots/pass/return_42.diagnostics.snap")
        );
    }

    #[test]
    fn snapshots_normalize_line_endings_and_final_newline() {
        assert_eq!(normalize_snapshot_text("a\r\nb"), "a\nb\n");
        assert_eq!(normalize_snapshot_text("a\n"), "a\n");
    }

    #[test]
    fn snapshots_match_return_42_empty_diagnostics_baseline() {
        let expected = include_str!("../../../tests/snapshots/pass/return_42.diagnostics.snap");

        assert_snapshot_eq(format_diagnostics_snapshot(&[]), expected);
    }

    #[test]
    fn snapshots_format_diagnostics_with_labels() {
        let diagnostic = Diagnostic::error("unexpected token")
            .with_primary_span(Span::new(SourceId::new(0), 7, 8))
            .with_label(DiagnosticLabel::new(
                Span::new(SourceId::new(0), 0, 6),
                "statement starts here",
            ));

        assert_snapshot_eq(
            format_diagnostics_snapshot(&[diagnostic]),
            "error: unexpected token\n --> source#0:7..8\n  = source#0:0..6: statement starts here\n",
        );
    }
}
