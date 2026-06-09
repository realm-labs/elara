//! Structured Lua runtime errors.

use core::fmt;

/// One Lua call frame captured in a traceback.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TraceFrame {
    function: Option<Box<str>>,
    source: Option<Box<str>>,
    line: Option<u32>,
    pc: Option<u32>,
}

impl TraceFrame {
    /// Creates a traceback frame with the available debug metadata.
    #[must_use]
    pub fn new(function: Option<impl Into<Box<str>>>, source: Option<impl Into<Box<str>>>) -> Self {
        Self {
            function: function.map(Into::into),
            source: source.map(Into::into),
            line: None,
            pc: None,
        }
    }

    /// Adds a source line when known.
    #[must_use]
    pub const fn with_line(mut self, line: u32) -> Self {
        self.line = Some(line);
        self
    }

    /// Adds a bytecode program counter when known.
    #[must_use]
    pub const fn with_pc(mut self, pc: u32) -> Self {
        self.pc = Some(pc);
        self
    }

    /// Source-level function name, when known.
    #[must_use]
    pub fn function(&self) -> Option<&str> {
        match &self.function {
            Some(function) => Some(function.as_ref()),
            None => None,
        }
    }

    /// Chunk or source name, when known.
    #[must_use]
    pub fn source(&self) -> Option<&str> {
        match &self.source {
            Some(source) => Some(source.as_ref()),
            None => None,
        }
    }

    /// Source line, when known.
    #[must_use]
    pub const fn line(&self) -> Option<u32> {
        self.line
    }

    /// Bytecode program counter, when known.
    #[must_use]
    pub const fn pc(&self) -> Option<u32> {
        self.pc
    }
}

/// Runtime error object with a stable kind and traceback metadata.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LuaError<K> {
    kind: K,
    message: Box<str>,
    traceback: Vec<TraceFrame>,
}

impl<K> LuaError<K> {
    /// Creates a runtime error.
    #[must_use]
    pub fn new(kind: K, message: impl Into<Box<str>>) -> Self {
        Self {
            kind,
            message: message.into(),
            traceback: Vec::new(),
        }
    }

    /// Error kind for programmatic matching.
    #[must_use]
    pub const fn kind(&self) -> &K {
        &self.kind
    }

    /// Error message suitable for display.
    #[must_use]
    pub const fn message(&self) -> &str {
        &self.message
    }

    /// Captured traceback frames, innermost frame first.
    #[must_use]
    pub fn traceback(&self) -> &[TraceFrame] {
        &self.traceback
    }

    /// Adds one traceback frame if an equivalent frame is not already present.
    pub fn push_trace_frame(&mut self, frame: TraceFrame) {
        if self.traceback.last() != Some(&frame) {
            self.traceback.push(frame);
        }
    }

    /// Consumes this error and maps its kind.
    #[must_use]
    pub fn map_kind<T>(self, f: impl FnOnce(K) -> T) -> LuaError<T> {
        LuaError {
            kind: f(self.kind),
            message: self.message,
            traceback: self.traceback,
        }
    }
}

impl<K: fmt::Debug> fmt::Display for LuaError<K> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)?;
        if !self.traceback.is_empty() {
            formatter.write_str("\nstack traceback:")?;
            for frame in &self.traceback {
                formatter.write_str("\n\t")?;
                match (frame.source(), frame.line()) {
                    (Some(source), Some(line)) => write!(formatter, "{source}:{line}")?,
                    (Some(source), None) => formatter.write_str(source)?,
                    (None, _) => formatter.write_str("?")?,
                }
                formatter.write_str(": in ")?;
                formatter.write_str(frame.function().unwrap_or("?"))?;
            }
        }
        Ok(())
    }
}

impl<K: fmt::Debug> std::error::Error for LuaError<K> {}

#[cfg(test)]
mod tests {
    use super::{LuaError, TraceFrame};

    #[test]
    fn lua_error_preserves_kind_message_and_traceback() {
        let mut error = LuaError::new("kind", "boom");
        error.push_trace_frame(TraceFrame::new(Some("child"), Some("chunk")).with_pc(7));

        assert_eq!(error.kind(), &"kind");
        assert_eq!(error.message(), "boom");
        assert_eq!(error.traceback()[0].function(), Some("child"));
        assert_eq!(error.traceback()[0].source(), Some("chunk"));
        assert_eq!(error.traceback()[0].pc(), Some(7));
    }

    #[test]
    fn lua_error_display_includes_traceback() {
        let mut error = LuaError::new("kind", "boom");
        error.push_trace_frame(TraceFrame::new(Some("child"), Some("chunk")).with_line(3));

        assert_eq!(
            error.to_string(),
            "boom\nstack traceback:\n\tchunk:3: in child"
        );
    }
}
