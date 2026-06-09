//! Lua standard library support for Elara.
//!
//! This crate owns the standard libraries and profiles exposed to Lua programs.
//! It should build on the runtime and public API layers instead of reaching into
//! parser, compiler, or JIT internals.
//!
//! Standard library behavior targets the current stable Lua version only.

use std::collections::BTreeSet;

/// One standard-library module that can register itself into a target runtime.
pub trait Library<Target> {
    /// Stable library name.
    fn name(&self) -> &'static str;

    /// Registers this library into the target global environment.
    fn register(&self, target: &mut Target) -> Result<(), RegisterError>;
}

/// A target that accepts named standard-library globals.
pub trait GlobalRegistry<Value> {
    /// Registers one global value.
    fn set_global(&mut self, name: &'static str, value: Value) -> Result<(), RegisterError>;
}

/// Library implementation that registers a prebuilt value at one global name.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GlobalLibrary<Value> {
    name: &'static str,
    value: Value,
}

impl<Value> GlobalLibrary<Value> {
    /// Creates a global-value library registration.
    #[must_use]
    pub const fn new(name: &'static str, value: Value) -> Self {
        Self { name, value }
    }
}

impl<Target, Value> Library<Target> for GlobalLibrary<Value>
where
    Target: GlobalRegistry<Value>,
    Value: Clone,
{
    fn name(&self) -> &'static str {
        self.name
    }

    fn register(&self, target: &mut Target) -> Result<(), RegisterError> {
        target.set_global(self.name, self.value.clone())
    }
}

/// Error raised while registering a standard library.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RegisterError {
    library: Option<&'static str>,
    message: Box<str>,
}

impl RegisterError {
    /// Creates a registration error not tied to a specific library.
    #[must_use]
    pub fn new(message: impl Into<Box<str>>) -> Self {
        Self {
            library: None,
            message: message.into(),
        }
    }

    /// Returns this error with the current library name attached.
    #[must_use]
    pub fn with_library(mut self, library: &'static str) -> Self {
        self.library = Some(library);
        self
    }

    /// Library being registered when the error occurred, when known.
    #[must_use]
    pub const fn library(&self) -> Option<&'static str> {
        self.library
    }

    /// Human-readable error message.
    #[must_use]
    pub const fn message(&self) -> &str {
        &self.message
    }
}

/// Known standard-library groups.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum StdLib {
    /// Basic functions.
    Base,
    /// Coroutine library.
    Coroutine,
    /// Table library.
    Table,
    /// String library.
    String,
    /// UTF-8 library.
    Utf8,
    /// Math library.
    Math,
    /// I/O library.
    Io,
    /// Operating-system library.
    Os,
    /// Package/module loading library.
    Package,
    /// Debug library.
    Debug,
}

impl StdLib {
    /// Stable Lua global/module name.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Base => "_G",
            Self::Coroutine => "coroutine",
            Self::Table => "table",
            Self::String => "string",
            Self::Utf8 => "utf8",
            Self::Math => "math",
            Self::Io => "io",
            Self::Os => "os",
            Self::Package => "package",
            Self::Debug => "debug",
        }
    }
}

/// Explicit set of standard libraries to register.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct StdLibSet {
    libraries: BTreeSet<StdLib>,
}

impl StdLibSet {
    /// Creates an empty library set.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a set containing all known libraries.
    #[must_use]
    pub fn full() -> Self {
        Self::from_iter([
            StdLib::Base,
            StdLib::Coroutine,
            StdLib::Table,
            StdLib::String,
            StdLib::Utf8,
            StdLib::Math,
            StdLib::Io,
            StdLib::Os,
            StdLib::Package,
            StdLib::Debug,
        ])
    }

    /// Creates the minimal useful library set.
    #[must_use]
    pub fn minimal() -> Self {
        Self::from_iter([
            StdLib::Base,
            StdLib::Coroutine,
            StdLib::Table,
            StdLib::String,
            StdLib::Math,
        ])
    }

    /// Creates a sandbox-oriented library set.
    #[must_use]
    pub fn sandboxed() -> Self {
        Self::from_iter([
            StdLib::Base,
            StdLib::Coroutine,
            StdLib::Table,
            StdLib::String,
            StdLib::Utf8,
            StdLib::Math,
        ])
    }

    /// Adds one library.
    pub fn insert(&mut self, library: StdLib) {
        self.libraries.insert(library);
    }

    /// Returns true when this set contains a library.
    #[must_use]
    pub fn contains(&self, library: StdLib) -> bool {
        self.libraries.contains(&library)
    }

    /// Iterates libraries in deterministic registration order.
    pub fn iter(&self) -> impl Iterator<Item = StdLib> + '_ {
        self.libraries.iter().copied()
    }

    /// Number of selected libraries.
    #[must_use]
    pub fn len(&self) -> usize {
        self.libraries.len()
    }

    /// Returns true when no libraries are selected.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.libraries.is_empty()
    }
}

impl FromIterator<StdLib> for StdLibSet {
    fn from_iter<T: IntoIterator<Item = StdLib>>(iter: T) -> Self {
        Self {
            libraries: iter.into_iter().collect(),
        }
    }
}

/// Standard-library profile requested by an embedder.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum StdLibProfile {
    /// Complete current-version standard library.
    Full,
    /// Essential libraries for typical pure Lua programs.
    Minimal,
    /// Libraries suitable for sandboxed embedding.
    Sandboxed,
    /// Caller-selected library set.
    Custom(StdLibSet),
}

impl StdLibProfile {
    /// Expands this profile to an explicit library set.
    #[must_use]
    pub fn libraries(&self) -> StdLibSet {
        match self {
            Self::Full => StdLibSet::full(),
            Self::Minimal => StdLibSet::minimal(),
            Self::Sandboxed => StdLibSet::sandboxed(),
            Self::Custom(libraries) => libraries.clone(),
        }
    }

    /// Registers matching libraries from a registry into a target.
    pub fn register<Target>(
        &self,
        registry: &StdLibRegistry<Target>,
        target: &mut Target,
    ) -> Result<(), RegisterError> {
        let libraries = self.libraries();
        for library in registry.libraries_for(&libraries) {
            library
                .register(target)
                .map_err(|error| error.with_library(library.name()))?;
        }
        Ok(())
    }
}

/// Registry of available library implementations.
pub struct StdLibRegistry<Target> {
    entries: Vec<(StdLib, Box<dyn Library<Target>>)>,
}

impl<Target> StdLibRegistry<Target> {
    /// Creates an empty registry.
    #[must_use]
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    /// Adds one library implementation.
    pub fn add<L>(&mut self, library: StdLib, implementation: L)
    where
        L: Library<Target> + 'static,
    {
        self.entries.push((library, Box::new(implementation)));
    }

    /// Returns implementations selected by a library set.
    pub fn libraries_for(&self, set: &StdLibSet) -> impl Iterator<Item = &dyn Library<Target>> {
        self.entries
            .iter()
            .filter(|(library, _)| set.contains(*library))
            .map(|(_, implementation)| implementation.as_ref())
    }

    /// Number of registered implementations.
    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Returns true when no implementations are registered.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

impl<Target> Default for StdLibRegistry<Target> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::{
        GlobalLibrary, GlobalRegistry, Library, RegisterError, StdLib, StdLibProfile,
        StdLibRegistry, StdLibSet,
    };

    struct NamedLibrary(&'static str);

    impl Library<Vec<&'static str>> for NamedLibrary {
        fn name(&self) -> &'static str {
            self.0
        }

        fn register(&self, target: &mut Vec<&'static str>) -> Result<(), RegisterError> {
            target.push(self.0);
            Ok(())
        }
    }

    struct FailingLibrary;

    impl Library<Vec<&'static str>> for FailingLibrary {
        fn name(&self) -> &'static str {
            "fail"
        }

        fn register(&self, _target: &mut Vec<&'static str>) -> Result<(), RegisterError> {
            Err(RegisterError::new("boom"))
        }
    }

    #[derive(Default)]
    struct Globals(Vec<(&'static str, i32)>);

    impl GlobalRegistry<i32> for Globals {
        fn set_global(&mut self, name: &'static str, value: i32) -> Result<(), RegisterError> {
            self.0.push((name, value));
            Ok(())
        }
    }

    #[test]
    fn registry_profiles_expand_expected_libraries() {
        let full = StdLibProfile::Full.libraries();
        let minimal = StdLibProfile::Minimal.libraries();
        let sandboxed = StdLibProfile::Sandboxed.libraries();

        assert!(full.contains(StdLib::Io));
        assert!(minimal.contains(StdLib::Coroutine));
        assert!(!minimal.contains(StdLib::Utf8));
        assert!(sandboxed.contains(StdLib::Utf8));
        assert!(!sandboxed.contains(StdLib::Io));
        assert!(!sandboxed.contains(StdLib::Os));
        assert!(!sandboxed.contains(StdLib::Package));
        assert!(!sandboxed.contains(StdLib::Debug));
    }

    #[test]
    fn registry_registers_selected_libraries_in_registry_order() {
        let mut registry = StdLibRegistry::new();
        registry.add(StdLib::Math, NamedLibrary("math"));
        registry.add(StdLib::Base, NamedLibrary("base"));
        registry.add(StdLib::Io, NamedLibrary("io"));
        let profile = StdLibProfile::Custom(StdLibSet::from_iter([StdLib::Base, StdLib::Math]));
        let mut target = Vec::new();

        profile
            .register(&registry, &mut target)
            .expect("registration should pass");

        assert_eq!(target, vec!["math", "base"]);
    }

    #[test]
    fn registry_attaches_library_name_to_errors() {
        let mut registry = StdLibRegistry::new();
        registry.add(StdLib::Base, FailingLibrary);
        let mut target = Vec::new();

        let error = StdLibProfile::Minimal
            .register(&registry, &mut target)
            .expect_err("registration should fail");

        assert_eq!(error.library(), Some("fail"));
        assert_eq!(error.message(), "boom");
    }

    #[test]
    fn registry_global_library_registers_named_value() {
        let mut registry = StdLibRegistry::new();
        registry.add(StdLib::Base, GlobalLibrary::new("base", 1));
        let mut globals = Globals::default();

        StdLibProfile::Minimal
            .register(&registry, &mut globals)
            .expect("registration should pass");

        assert_eq!(globals.0, vec![("base", 1)]);
    }
}
