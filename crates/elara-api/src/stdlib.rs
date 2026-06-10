//! Standard-library integration for the public Rust API.

use elara_core::Value;
use elara_interp::{RuntimeEnvironment, RuntimeErrorKind};
use elara_stdlib::{NativeError, StdLib, StdLibProfile, StdLibSet, native_functions};

/// Builds a primitive runtime environment containing implemented stdlib natives
/// from the given profile.
#[must_use]
pub fn runtime_environment_for_stdlib(profile: &StdLibProfile) -> RuntimeEnvironment {
    runtime_environment_for_libraries(&profile.libraries())
}

/// Builds a primitive runtime environment containing implemented stdlib natives
/// from the given library set.
#[must_use]
pub fn runtime_environment_for_libraries(libraries: &StdLibSet) -> RuntimeEnvironment {
    let mut environment = RuntimeEnvironment::new();
    register_libraries(&mut environment, libraries);
    environment
}

fn register_libraries(environment: &mut RuntimeEnvironment, libraries: &StdLibSet) {
    for library in libraries.iter() {
        register_library(environment, library);
    }
}

fn register_library(environment: &mut RuntimeEnvironment, library: StdLib) {
    let functions = native_functions(library);
    if functions.is_empty() {
        return;
    }

    if library == StdLib::Base {
        for spec in functions {
            let function = spec.function();
            environment.register_native_global(spec.descriptor().name(), move |args| {
                function(args).map_err(native_error_to_runtime_error)
            });
        }
        return;
    }

    let fields: Vec<_> = functions
        .iter()
        .map(|spec| {
            let function = spec.function();
            let index = environment
                .push_native(move |args| function(args).map_err(native_error_to_runtime_error));
            (
                spec.descriptor().name(),
                Value::native_function_index(index),
            )
        })
        .collect();
    environment.set_global_table(library.name(), fields);
}

fn native_error_to_runtime_error(error: NativeError) -> elara_interp::RuntimeError {
    RuntimeErrorKind::NativeFunctionError {
        message: error.message().into(),
    }
    .into()
}
