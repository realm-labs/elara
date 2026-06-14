//! Optional JIT configuration exposed by the public API.

/// Optional JIT execution mode for a Lua runtime.
///
/// This is configuration plumbing only. M16 later milestones add lowering,
/// runtime helpers, hot counters, and interpreter/JIT transitions.
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
