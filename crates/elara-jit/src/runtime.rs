//! Baseline JIT runtime transition layer.

use std::collections::HashMap;

use elara_bytecode::Proto;
use elara_core::Value;
use elara_interp::{RuntimeResult, execute_proto};

use crate::ArithmeticJitFunction;

/// Baseline JIT runtime mode.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum JitRuntimeMode {
    /// Disable JIT compilation and always execute through the interpreter.
    #[default]
    Off,
    /// Compile a supported Proto after it reaches the given call threshold.
    Hot {
        /// Number of calls before a Proto is considered hot.
        threshold: u32,
    },
    /// Compile a supported Proto before its first execution.
    Always,
}

/// Public status for a Proto entry in the baseline JIT runtime.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum JitEntryStatus {
    /// The Proto has not reached a compiled or unsupported state.
    Cold,
    /// The Proto has a compiled JIT entry.
    Compiled,
    /// The Proto cannot currently be compiled and should use interpreter fallback.
    Unsupported,
}

/// Baseline JIT transition counters.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct JitRuntimeStats {
    /// Number of direct interpreter executions.
    pub interpreter_runs: u64,
    /// Number of generated-code executions.
    pub jit_runs: u64,
    /// Number of interpreter fallbacks after the JIT path was considered.
    pub fallback_runs: u64,
    /// Number of JIT compile attempts.
    pub compile_attempts: u64,
    /// Number of successful JIT compile attempts.
    pub compile_successes: u64,
}

/// Baseline JIT runtime state.
#[derive(Default)]
pub struct JitRuntime {
    mode: JitRuntimeMode,
    debug_hooks_active: bool,
    entries: HashMap<usize, ProtoJitEntry>,
    stats: JitRuntimeStats,
}

impl JitRuntime {
    /// Creates a JIT runtime with the given mode.
    #[must_use]
    pub fn new(mode: JitRuntimeMode) -> Self {
        Self {
            mode,
            debug_hooks_active: false,
            entries: HashMap::new(),
            stats: JitRuntimeStats::default(),
        }
    }

    /// Returns the current runtime mode.
    #[must_use]
    pub const fn mode(&self) -> JitRuntimeMode {
        self.mode
    }

    /// Returns transition counters.
    #[must_use]
    pub const fn stats(&self) -> JitRuntimeStats {
        self.stats
    }

    /// Returns whether active debug hooks force interpreter execution.
    #[must_use]
    pub const fn debug_hooks_active(&self) -> bool {
        self.debug_hooks_active
    }

    /// Sets whether active debug hooks should force interpreter execution.
    pub const fn set_debug_hooks_active(&mut self, active: bool) {
        self.debug_hooks_active = active;
    }

    /// Returns the current JIT entry status for a Proto.
    #[must_use]
    pub fn entry_status(&self, proto: &Proto) -> JitEntryStatus {
        self.entries
            .get(&proto_key(proto))
            .map_or(JitEntryStatus::Cold, ProtoJitEntry::status)
    }

    /// Returns the current hot counter for a Proto.
    #[must_use]
    pub fn hot_count(&self, proto: &Proto) -> u32 {
        self.entries
            .get(&proto_key(proto))
            .map_or(0, |entry| entry.hot_count)
    }

    /// Executes a Proto through the configured baseline JIT transition path.
    pub fn execute(&mut self, proto: &Proto) -> RuntimeResult<Vec<Value>> {
        if self.debug_hooks_active {
            return self.execute_interpreter(proto);
        }

        match self.mode {
            JitRuntimeMode::Off => self.execute_interpreter(proto),
            JitRuntimeMode::Always => self.execute_always(proto),
            JitRuntimeMode::Hot { threshold } => self.execute_hot(proto, threshold),
        }
    }

    fn execute_hot(&mut self, proto: &Proto, threshold: u32) -> RuntimeResult<Vec<Value>> {
        let key = proto_key(proto);
        let entry = self.entries.entry(key).or_default();
        entry.hot_count = entry.hot_count.saturating_add(1);

        if entry.is_compiled() {
            return self.execute_jit_or_fallback(proto, key);
        }

        if entry.unsupported {
            return self.execute_fallback(proto);
        }

        if entry.hot_count >= threshold {
            self.compile_entry(proto, key);
            if self
                .entries
                .get(&key)
                .is_some_and(ProtoJitEntry::is_compiled)
            {
                return self.execute_jit_or_fallback(proto, key);
            }
            return self.execute_fallback(proto);
        }

        self.execute_interpreter(proto)
    }

    fn execute_always(&mut self, proto: &Proto) -> RuntimeResult<Vec<Value>> {
        let key = proto_key(proto);
        let entry = self.entries.entry(key).or_default();
        entry.hot_count = entry.hot_count.saturating_add(1);

        if !self.entries[&key].is_compiled() && !self.entries[&key].unsupported {
            self.compile_entry(proto, key);
        }

        if self.entries[&key].is_compiled() {
            self.execute_jit_or_fallback(proto, key)
        } else {
            self.execute_fallback(proto)
        }
    }

    fn compile_entry(&mut self, proto: &Proto, key: usize) {
        self.stats.compile_attempts = self.stats.compile_attempts.saturating_add(1);
        let entry = self.entries.entry(key).or_default();
        match ArithmeticJitFunction::compile(proto) {
            Ok(function) => {
                entry.jit_entry = Some(JitEntry::Arithmetic(function));
                self.stats.compile_successes = self.stats.compile_successes.saturating_add(1);
            }
            Err(_error) => {
                entry.unsupported = true;
            }
        }
    }

    fn execute_jit_or_fallback(&mut self, proto: &Proto, key: usize) -> RuntimeResult<Vec<Value>> {
        let result = self
            .entries
            .get(&key)
            .and_then(|entry| entry.jit_entry.as_ref())
            .map(JitEntry::execute);
        match result {
            Some(Ok(values)) => {
                self.stats.jit_runs = self.stats.jit_runs.saturating_add(1);
                Ok(values)
            }
            Some(Err(_)) | None => self.execute_fallback(proto),
        }
    }

    fn execute_interpreter(&mut self, proto: &Proto) -> RuntimeResult<Vec<Value>> {
        self.stats.interpreter_runs = self.stats.interpreter_runs.saturating_add(1);
        execute_proto(proto)
    }

    fn execute_fallback(&mut self, proto: &Proto) -> RuntimeResult<Vec<Value>> {
        self.stats.fallback_runs = self.stats.fallback_runs.saturating_add(1);
        execute_proto(proto)
    }
}

fn proto_key(proto: &Proto) -> usize {
    std::ptr::from_ref(proto).addr()
}

#[derive(Default)]
struct ProtoJitEntry {
    hot_count: u32,
    jit_entry: Option<JitEntry>,
    unsupported: bool,
}

impl ProtoJitEntry {
    fn is_compiled(&self) -> bool {
        self.jit_entry.is_some()
    }

    fn status(&self) -> JitEntryStatus {
        if self.jit_entry.is_some() {
            JitEntryStatus::Compiled
        } else if self.unsupported {
            JitEntryStatus::Unsupported
        } else {
            JitEntryStatus::Cold
        }
    }
}

enum JitEntry {
    Arithmetic(ArithmeticJitFunction),
}

impl JitEntry {
    fn execute(&self) -> Result<Vec<Value>, crate::ArithmeticJitError> {
        match self {
            Self::Arithmetic(function) => function.execute(),
        }
    }
}

#[cfg(test)]
mod tests {
    use elara_bytecode::{Op, Proto, ProtoBuilder};
    use elara_core::Value;
    use elara_interp::execute_proto;

    use super::{JitEntryStatus, JitRuntime, JitRuntimeMode};

    #[test]
    fn jit_transition_off_matches_interpreter_without_compiling() {
        let proto = arithmetic_proto();
        let mut runtime = JitRuntime::new(JitRuntimeMode::Off);

        assert_eq!(runtime.execute(&proto), execute_proto(&proto));
        assert_eq!(runtime.entry_status(&proto), JitEntryStatus::Cold);
        assert_eq!(runtime.stats().interpreter_runs, 1);
        assert_eq!(runtime.stats().compile_attempts, 0);
        assert_eq!(runtime.stats().jit_runs, 0);
    }

    #[test]
    fn jit_transition_hot_counter_compiles_after_threshold() {
        let proto = arithmetic_proto();
        let mut runtime = JitRuntime::new(JitRuntimeMode::Hot { threshold: 2 });

        assert_eq!(runtime.execute(&proto), execute_proto(&proto));
        assert_eq!(runtime.entry_status(&proto), JitEntryStatus::Cold);
        assert_eq!(runtime.hot_count(&proto), 1);

        assert_eq!(runtime.execute(&proto), execute_proto(&proto));
        assert_eq!(runtime.entry_status(&proto), JitEntryStatus::Compiled);
        assert_eq!(runtime.hot_count(&proto), 2);
        assert_eq!(runtime.stats().compile_attempts, 1);
        assert_eq!(runtime.stats().compile_successes, 1);
        assert_eq!(runtime.stats().jit_runs, 1);
    }

    #[test]
    fn jit_transition_always_compiles_before_first_execution() {
        let proto = arithmetic_proto();
        let mut runtime = JitRuntime::new(JitRuntimeMode::Always);

        assert_eq!(runtime.execute(&proto), execute_proto(&proto));
        assert_eq!(runtime.entry_status(&proto), JitEntryStatus::Compiled);
        assert_eq!(runtime.stats().interpreter_runs, 0);
        assert_eq!(runtime.stats().jit_runs, 1);
    }

    #[test]
    fn jit_transition_falls_back_to_interpreter_for_unsupported_proto() {
        let proto = boolean_proto();
        let mut runtime = JitRuntime::new(JitRuntimeMode::Always);

        assert_eq!(runtime.execute(&proto), execute_proto(&proto));
        assert_eq!(runtime.entry_status(&proto), JitEntryStatus::Unsupported);
        assert_eq!(runtime.stats().compile_attempts, 1);
        assert_eq!(runtime.stats().compile_successes, 0);
        assert_eq!(runtime.stats().fallback_runs, 1);
        assert_eq!(runtime.stats().jit_runs, 0);
    }

    #[test]
    fn jit_transition_debug_hooks_force_interpreter_without_compiling() {
        let proto = arithmetic_proto();
        let mut runtime = JitRuntime::new(JitRuntimeMode::Always);
        runtime.set_debug_hooks_active(true);

        assert!(runtime.debug_hooks_active());
        assert_eq!(runtime.execute(&proto), execute_proto(&proto));
        assert_eq!(runtime.entry_status(&proto), JitEntryStatus::Cold);
        assert_eq!(runtime.hot_count(&proto), 0);
        assert_eq!(runtime.stats().interpreter_runs, 1);
        assert_eq!(runtime.stats().compile_attempts, 0);
        assert_eq!(runtime.stats().jit_runs, 0);
    }

    #[test]
    fn jit_transition_resumes_after_debug_hooks_clear() {
        let proto = arithmetic_proto();
        let mut runtime = JitRuntime::new(JitRuntimeMode::Always);
        runtime.set_debug_hooks_active(true);

        assert_eq!(runtime.execute(&proto), execute_proto(&proto));
        runtime.set_debug_hooks_active(false);

        assert!(!runtime.debug_hooks_active());
        assert_eq!(runtime.execute(&proto), execute_proto(&proto));
        assert_eq!(runtime.entry_status(&proto), JitEntryStatus::Compiled);
        assert_eq!(runtime.hot_count(&proto), 1);
        assert_eq!(runtime.stats().interpreter_runs, 1);
        assert_eq!(runtime.stats().compile_attempts, 1);
        assert_eq!(runtime.stats().compile_successes, 1);
        assert_eq!(runtime.stats().jit_runs, 1);
    }

    fn arithmetic_proto() -> Proto {
        let mut builder = ProtoBuilder::new().with_signature(3, 0, false);
        let left = builder.add_constant(Value::integer(8));
        let right = builder.add_constant(Value::integer(5));
        builder.emit_abx(Op::LoadK, 0, u64::from(left));
        builder.emit_abx(Op::LoadK, 1, u64::from(right));
        builder.emit_abc(Op::Add, 2, 0, 1);
        builder.emit_abc(Op::AddInt, 2, 2, 1);
        builder.emit_abc(Op::Return, 2, 1, 0);
        builder.finish()
    }

    fn boolean_proto() -> Proto {
        let mut builder = ProtoBuilder::new().with_signature(1, 0, false);
        builder.emit_abc(Op::LoadBool, 0, 1, 0);
        builder.emit_abc(Op::Return, 0, 1, 0);
        builder.finish()
    }
}
