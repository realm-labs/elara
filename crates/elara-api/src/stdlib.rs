//! Standard-library integration for the public Rust API.

use std::{
    io::{self, Write},
    sync::{Arc, Mutex},
    time::{SystemTime, UNIX_EPOCH},
};

use elara_core::Value;
use elara_interp::{NativeContext, RuntimeEnvironment, RuntimeErrorKind};
use elara_stdlib::{
    LuaRandomState, NativeError, NativeErrorKind, NativeRuntime, StdLib, StdLibProfile, StdLibSet,
    native_functions,
};

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
            environment.register_native_global(spec.descriptor().name(), move |context, args| {
                let mut runtime = InterpNativeRuntime {
                    context,
                    random_state: None,
                };
                function(&mut runtime, args).map_err(native_error_to_runtime_error)
            });
        }
        return;
    }

    let random_state =
        (library == StdLib::Math).then(|| Arc::new(Mutex::new(LuaRandomState::default())));
    let fields: Vec<_> = functions
        .iter()
        .map(|spec| {
            let function = spec.function();
            let random_state = random_state.clone();
            let index = environment.push_native(move |context, args| {
                let mut runtime = InterpNativeRuntime {
                    context,
                    random_state: random_state.clone(),
                };
                function(&mut runtime, args).map_err(native_error_to_runtime_error)
            });
            (
                spec.descriptor().name(),
                Value::native_function_index(index),
            )
        })
        .collect();
    environment.set_global_table(library.name(), fields);
}

struct InterpNativeRuntime<'a, 'runtime> {
    context: &'a mut NativeContext<'runtime>,
    random_state: Option<Arc<Mutex<LuaRandomState>>>,
}

impl NativeRuntime for InterpNativeRuntime<'_, '_> {
    fn intern_short_string(&mut self, bytes: &[u8]) -> Result<Value, NativeError> {
        self.context.intern_short_string(bytes).map_err(|error| {
            NativeErrorKind::RuntimeError {
                message: error.to_string().into(),
            }
            .into()
        })
    }

    fn short_string_bytes(&self, value: Value) -> Option<&[u8]> {
        self.context.short_string_bytes(value)
    }

    fn create_table(&mut self, entries: &[(Value, Value)]) -> Result<Value, NativeError> {
        self.context
            .create_table(entries.iter().copied())
            .map_err(|error| {
                NativeErrorKind::RuntimeError {
                    message: error.to_string().into(),
                }
                .into()
            })
    }

    fn table_array_len(&self, table: Value) -> Result<i64, NativeError> {
        self.context.table_array_len(table).map_err(|error| {
            NativeErrorKind::RuntimeError {
                message: error.to_string().into(),
            }
            .into()
        })
    }

    fn table_get_integer(&self, table: Value, index: i64) -> Result<Value, NativeError> {
        self.context
            .table_get_integer(table, index)
            .map_err(|error| {
                NativeErrorKind::RuntimeError {
                    message: error.to_string().into(),
                }
                .into()
            })
    }

    fn table_get(&self, table: Value, key: Value) -> Result<Value, NativeError> {
        self.context.table_get(table, key).map_err(|error| {
            NativeErrorKind::RuntimeError {
                message: error.to_string().into(),
            }
            .into()
        })
    }

    fn table_next(&self, table: Value, key: Value) -> Result<Option<(Value, Value)>, NativeError> {
        self.context.table_next(table, key).map_err(|error| {
            NativeErrorKind::RuntimeError {
                message: error.to_string().into(),
            }
            .into()
        })
    }

    fn table_set_integer(
        &mut self,
        table: Value,
        index: i64,
        value: Value,
    ) -> Result<(), NativeError> {
        self.context
            .table_set_integer(table, index, value)
            .map_err(|error| {
                NativeErrorKind::RuntimeError {
                    message: error.to_string().into(),
                }
                .into()
            })
    }

    fn table_set(&mut self, table: Value, key: Value, value: Value) -> Result<(), NativeError> {
        self.context.table_set(table, key, value).map_err(|error| {
            NativeErrorKind::RuntimeError {
                message: error.to_string().into(),
            }
            .into()
        })
    }

    fn table_metatable(&self, table: Value) -> Result<Value, NativeError> {
        self.context.table_metatable(table).map_err(|error| {
            NativeErrorKind::RuntimeError {
                message: error.to_string().into(),
            }
            .into()
        })
    }

    fn table_set_metatable(&mut self, table: Value, metatable: Value) -> Result<(), NativeError> {
        self.context
            .table_set_metatable(table, metatable)
            .map_err(|error| {
                NativeErrorKind::RuntimeError {
                    message: error.to_string().into(),
                }
                .into()
            })
    }

    fn next_random_u64(&mut self) -> Result<u64, NativeError> {
        let random_state =
            self.random_state
                .as_ref()
                .ok_or_else(|| NativeErrorKind::RuntimeError {
                    message: "native runtime does not support random numbers".into(),
                })?;
        let mut random_state = random_state
            .lock()
            .map_err(|_| NativeErrorKind::RuntimeError {
                message: "random state lock poisoned".into(),
            })?;
        Ok(random_state.next_u64())
    }

    fn random_seed(&mut self) -> Result<u64, NativeError> {
        self.random_state
            .as_ref()
            .ok_or_else(|| NativeErrorKind::RuntimeError {
                message: "native runtime does not support random seeding".into(),
            })?;
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default();
        let stack_marker = (&now as *const _) as usize as u64;
        Ok((now.as_nanos() as u64) ^ stack_marker.rotate_left(17))
    }

    fn set_random_seed(&mut self, seed1: u64, seed2: u64) -> Result<(), NativeError> {
        let random_state =
            self.random_state
                .as_ref()
                .ok_or_else(|| NativeErrorKind::RuntimeError {
                    message: "native runtime does not support random seeding".into(),
                })?;
        let mut random_state = random_state
            .lock()
            .map_err(|_| NativeErrorKind::RuntimeError {
                message: "random state lock poisoned".into(),
            })?;
        *random_state = LuaRandomState::from_seeds(seed1, seed2);
        Ok(())
    }

    fn write_output(&mut self, bytes: &[u8]) -> Result<(), NativeError> {
        io::stdout().write_all(bytes).map_err(|error| {
            NativeErrorKind::RuntimeError {
                message: error.to_string().into(),
            }
            .into()
        })
    }
}

fn native_error_to_runtime_error(error: NativeError) -> elara_interp::RuntimeError {
    RuntimeErrorKind::NativeFunctionError {
        message: error.message().into(),
    }
    .into()
}
