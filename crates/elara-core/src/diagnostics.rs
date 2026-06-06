//! Source locations and diagnostics shared across Elara crates.

use core::fmt;

/// Identifier for a source document known to the embedding runtime or test
/// harness.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct SourceId(u32);

impl SourceId {
    /// Creates a source identifier from a stable numeric index.
    #[must_use]
    pub const fn new(raw: u32) -> Self {
        Self(raw)
    }

    /// Returns the underlying numeric source identifier.
    #[must_use]
    pub const fn get(self) -> u32 {
        self.0
    }
}

impl fmt::Display for SourceId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "source#{}", self.0)
    }
}

/// Half-open byte range in one source document.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct Span {
    source: SourceId,
    start: u32,
    end: u32,
}

impl Span {
    /// Creates a half-open byte span.
    ///
    /// # Panics
    ///
    /// Panics if `start` is greater than `end`.
    #[must_use]
    pub fn new(source: SourceId, start: u32, end: u32) -> Self {
        assert!(start <= end, "span start must not exceed span end");
        Self { source, start, end }
    }

    /// Source document that owns this span.
    #[must_use]
    pub const fn source(self) -> SourceId {
        self.source
    }

    /// Start byte offset, inclusive.
    #[must_use]
    pub const fn start(self) -> u32 {
        self.start
    }

    /// End byte offset, exclusive.
    #[must_use]
    pub const fn end(self) -> u32 {
        self.end
    }

    /// Length of the span in bytes.
    #[must_use]
    pub const fn len(self) -> u32 {
        self.end - self.start
    }

    /// Returns true for an empty byte range.
    #[must_use]
    pub const fn is_empty(self) -> bool {
        self.start == self.end
    }

    /// Returns true when `offset` is inside this half-open range.
    #[must_use]
    pub const fn contains(self, offset: u32) -> bool {
        self.start <= offset && offset < self.end
    }
}

impl fmt::Display for Span {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}:{}..{}", self.source, self.start, self.end)
    }
}

/// A value paired with its source span.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct Spanned<T> {
    /// Wrapped value.
    pub value: T,
    /// Source span for `value`.
    pub span: Span,
}

impl<T> Spanned<T> {
    /// Creates a spanned value.
    #[must_use]
    pub const fn new(value: T, span: Span) -> Self {
        Self { value, span }
    }

    /// Maps the wrapped value while preserving the span.
    pub fn map<U>(self, map_value: impl FnOnce(T) -> U) -> Spanned<U> {
        Spanned {
            value: map_value(self.value),
            span: self.span,
        }
    }
}

/// Diagnostic severity level.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum DiagnosticSeverity {
    /// Compilation or runtime error.
    Error,
    /// Suspicious construct that can still proceed.
    Warning,
    /// Informational note attached to another diagnostic.
    Note,
    /// Help text or a suggested action.
    Help,
}

impl fmt::Display for DiagnosticSeverity {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let text = match self {
            Self::Error => "error",
            Self::Warning => "warning",
            Self::Note => "note",
            Self::Help => "help",
        };
        formatter.write_str(text)
    }
}

/// Secondary source label attached to a diagnostic.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DiagnosticLabel {
    span: Span,
    message: String,
}

impl DiagnosticLabel {
    /// Creates a diagnostic label for a specific source span.
    pub fn new(span: Span, message: impl Into<String>) -> Self {
        Self {
            span,
            message: message.into(),
        }
    }

    /// Label span.
    #[must_use]
    pub const fn span(&self) -> Span {
        self.span
    }

    /// Label message.
    #[must_use]
    pub fn message(&self) -> &str {
        &self.message
    }
}

/// Structured diagnostic message.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Diagnostic {
    severity: DiagnosticSeverity,
    message: String,
    primary_span: Option<Span>,
    labels: Vec<DiagnosticLabel>,
}

impl Diagnostic {
    /// Creates a diagnostic with no source span.
    pub fn new(severity: DiagnosticSeverity, message: impl Into<String>) -> Self {
        Self {
            severity,
            message: message.into(),
            primary_span: None,
            labels: Vec::new(),
        }
    }

    /// Creates an error diagnostic.
    pub fn error(message: impl Into<String>) -> Self {
        Self::new(DiagnosticSeverity::Error, message)
    }

    /// Attaches a primary source span.
    #[must_use]
    pub const fn with_primary_span(mut self, span: Span) -> Self {
        self.primary_span = Some(span);
        self
    }

    /// Adds a secondary source label.
    #[must_use]
    pub fn with_label(mut self, label: DiagnosticLabel) -> Self {
        self.labels.push(label);
        self
    }

    /// Severity level.
    #[must_use]
    pub const fn severity(&self) -> DiagnosticSeverity {
        self.severity
    }

    /// Primary diagnostic message.
    #[must_use]
    pub fn message(&self) -> &str {
        &self.message
    }

    /// Primary source span, when available.
    #[must_use]
    pub const fn primary_span(&self) -> Option<Span> {
        self.primary_span
    }

    /// Secondary labels.
    #[must_use]
    pub fn labels(&self) -> &[DiagnosticLabel] {
        &self.labels
    }
}

impl fmt::Display for Diagnostic {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}: {}", self.severity, self.message)?;

        if let Some(span) = self.primary_span {
            write!(formatter, "\n --> {span}")?;
        }

        for label in &self.labels {
            write!(formatter, "\n  = {}: {}", label.span, label.message)?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::{Diagnostic, DiagnosticLabel, DiagnosticSeverity, SourceId, Span, Spanned};

    #[test]
    fn diagnostics_span_reports_range_information() {
        let span = Span::new(SourceId::new(3), 4, 9);

        assert_eq!(span.source(), SourceId::new(3));
        assert_eq!(span.start(), 4);
        assert_eq!(span.end(), 9);
        assert_eq!(span.len(), 5);
        assert!(!span.is_empty());
        assert!(span.contains(4));
        assert!(span.contains(8));
        assert!(!span.contains(9));
        assert_eq!(span.to_string(), "source#3:4..9");
    }

    #[test]
    #[should_panic(expected = "span start must not exceed span end")]
    fn diagnostics_span_rejects_inverted_ranges() {
        let _span = Span::new(SourceId::new(0), 2, 1);
    }

    #[test]
    fn diagnostics_spanned_map_preserves_span() {
        let span = Span::new(SourceId::new(1), 0, 6);
        let token = Spanned::new("return", span);

        let mapped = token.map(str::len);

        assert_eq!(mapped.value, 6);
        assert_eq!(mapped.span, span);
    }

    #[test]
    fn diagnostics_severity_displays_lowercase_names() {
        assert_eq!(DiagnosticSeverity::Error.to_string(), "error");
        assert_eq!(DiagnosticSeverity::Warning.to_string(), "warning");
        assert_eq!(DiagnosticSeverity::Note.to_string(), "note");
        assert_eq!(DiagnosticSeverity::Help.to_string(), "help");
    }

    #[test]
    fn diagnostics_display_includes_message_span_and_labels() {
        let primary = Span::new(SourceId::new(2), 10, 16);
        let label = DiagnosticLabel::new(Span::new(SourceId::new(2), 4, 9), "declared here");

        let diagnostic = Diagnostic::error("unknown global")
            .with_primary_span(primary)
            .with_label(label);

        assert_eq!(diagnostic.severity(), DiagnosticSeverity::Error);
        assert_eq!(diagnostic.message(), "unknown global");
        assert_eq!(diagnostic.primary_span(), Some(primary));
        assert_eq!(diagnostic.labels()[0].message(), "declared here");
        assert_eq!(
            diagnostic.to_string(),
            "error: unknown global\n --> source#2:10..16\n  = source#2:4..9: declared here"
        );
    }
}
