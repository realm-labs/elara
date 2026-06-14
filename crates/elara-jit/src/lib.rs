//! Optional Cranelift JIT support for Elara.
//!
//! This crate owns optional Cranelift-based compilation from verified Elara
//! bytecode into native code. JIT behavior must stay semantically equivalent to
//! the interpreter for every supported path.
//!
//! It consumes bytecode and runtime metadata. It must not parse Lua source.

pub mod abi;

use cranelift_codegen::{isa, settings, settings::Configurable};
use cranelift_frontend::FunctionBuilderContext;

pub use abi::{
    JitFn, JitRuntimeContext, JitStatus, RuntimeHelper, RuntimeHelperError, RuntimeHelperFn,
    RuntimeHelperId, RuntimeHelperRegistry,
};

/// Baseline JIT configuration placeholder.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct JitConfig {
    optimization_level: OptimizationLevel,
}

impl JitConfig {
    /// Creates a baseline JIT configuration.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            optimization_level: OptimizationLevel::Speed,
        }
    }

    /// Returns the configured Cranelift optimization level.
    #[must_use]
    pub const fn optimization_level(&self) -> OptimizationLevel {
        self.optimization_level
    }

    /// Selects the Cranelift optimization level.
    #[must_use]
    pub const fn with_optimization_level(mut self, level: OptimizationLevel) -> Self {
        self.optimization_level = level;
        self
    }

    /// Builds Cranelift settings for this configuration.
    pub fn cranelift_flags(&self) -> Result<settings::Flags, settings::SetError> {
        let mut builder = settings::builder();
        builder.set("opt_level", self.optimization_level.as_cranelift())?;
        Ok(settings::Flags::new(builder))
    }
}

impl Default for JitConfig {
    fn default() -> Self {
        Self::new()
    }
}

/// Cranelift optimization level used by the baseline JIT.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OptimizationLevel {
    /// Minimize compile time.
    None,
    /// Optimize generated code for speed.
    Speed,
    /// Optimize generated code for speed and size.
    SpeedAndSize,
}

impl OptimizationLevel {
    const fn as_cranelift(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Speed => "speed",
            Self::SpeedAndSize => "speed_and_size",
        }
    }
}

/// Creates a host ISA builder for future Cranelift modules.
///
/// The first M16 slice only wires dependencies and configuration; later slices
/// lower bytecode and allocate executable code.
pub fn host_isa_builder() -> Result<isa::Builder, &'static str> {
    cranelift_native::builder()
}

/// Creates reusable Cranelift frontend state for a function translation.
#[must_use]
pub fn function_builder_context() -> FunctionBuilderContext {
    FunctionBuilderContext::new()
}

#[cfg(test)]
mod tests {
    use super::{JitConfig, OptimizationLevel, function_builder_context, host_isa_builder};

    #[test]
    fn config_builds_cranelift_flags() {
        let flags = JitConfig::new()
            .with_optimization_level(OptimizationLevel::None)
            .cranelift_flags()
            .expect("valid cranelift flags");

        assert_eq!(
            flags.opt_level(),
            cranelift_codegen::settings::OptLevel::None
        );
    }

    #[test]
    fn scaffold_can_create_cranelift_contexts() {
        let _context = function_builder_context();
        assert!(host_isa_builder().is_ok());
    }
}
