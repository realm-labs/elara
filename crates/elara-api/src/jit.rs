//! Optional JIT configuration exposed by the public API.

use elara_jit::JitRuntimeMode;

/// Optional JIT execution mode for a Lua runtime.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum JitMode {
    /// Always execute through the interpreter.
    #[default]
    Off,
    /// Compile functions after they cross the configured hotness threshold.
    Hot {
        /// Number of observed executions before a function becomes hot.
        threshold: u32,
    },
    /// Compile supported functions before first execution.
    Always,
}

impl JitMode {
    pub(crate) const fn runtime_mode(self) -> JitRuntimeMode {
        match self {
            Self::Off => JitRuntimeMode::Off,
            Self::Hot { threshold } => JitRuntimeMode::Hot { threshold },
            Self::Always => JitRuntimeMode::Always,
        }
    }
}
