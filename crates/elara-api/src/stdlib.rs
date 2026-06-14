//! Standard-library integration for the public Rust API.

use std::{
    io::{self, Write},
    sync::{Arc, Mutex},
    time::{SystemTime, UNIX_EPOCH},
};

use elara_core::{ThreadStatus, Value};
use elara_interp::{NativeContext, RuntimeEnvironment, RuntimeErrorKind};
use elara_stdlib::{
    BASE_IPAIRS_AUX_NATIVE, BASE_NEXT_NATIVE, DebugInfoTarget, LuaRandomState, MATH_CONSTANTS,
    NativeError, NativeErrorKind, NativeRuntime, PACKAGE_C_ROOT_SEARCHER_NATIVE,
    PACKAGE_C_SEARCHER_NATIVE, PACKAGE_CONFIG, PACKAGE_CPATH, PACKAGE_LUA_SEARCHER_NATIVE,
    PACKAGE_PATH, PACKAGE_PRELOAD_SEARCHER_NATIVE, STRING_GMATCH_AUX_NATIVE, StdLib, StdLibProfile,
    StdLibSet, UTF8_CHAR_PATTERN, UTF8_CODES_AUX_LAX_NATIVE, UTF8_CODES_AUX_STRICT_NATIVE,
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
        let helpers = register_base_helpers(environment);
        for spec in functions {
            let function = spec.function();
            let helpers = helpers.clone();
            environment.register_native_global(spec.descriptor().name(), move |context, args| {
                let mut runtime = InterpNativeRuntime {
                    context,
                    random_state: None,
                    helpers: helpers.clone(),
                };
                function(&mut runtime, args).map_err(native_error_to_runtime_error)
            });
        }
        return;
    }

    let random_state =
        (library == StdLib::Math).then(|| Arc::new(Mutex::new(LuaRandomState::default())));
    let helpers = match library {
        StdLib::Coroutine => register_coroutine_helpers(),
        StdLib::Debug => register_debug_helpers(),
        StdLib::String => register_string_helpers(environment),
        StdLib::Utf8 => register_utf8_helpers(environment),
        _ => NativeHelpers::default(),
    };
    let mut fields: Vec<_> = functions
        .iter()
        .map(|spec| {
            let function = spec.function();
            let random_state = random_state.clone();
            let helpers = helpers.clone();
            let index = environment.push_native(move |context, args| {
                let mut runtime = InterpNativeRuntime {
                    context,
                    random_state: random_state.clone(),
                    helpers: helpers.clone(),
                };
                function(&mut runtime, args).map_err(native_error_to_runtime_error)
            });
            (
                spec.descriptor().name(),
                Value::native_function_index(index),
            )
        })
        .collect();
    if library == StdLib::Math {
        fields.extend(MATH_CONSTANTS.iter().copied());
    }
    match library {
        StdLib::Package => {
            let require = fields
                .iter()
                .find_map(|(name, value)| (*name == "require").then_some(*value));
            let preload_searcher =
                register_hidden_native(environment, PACKAGE_PRELOAD_SEARCHER_NATIVE.function());
            let lua_searcher =
                register_hidden_native(environment, PACKAGE_LUA_SEARCHER_NATIVE.function());
            let c_searcher =
                register_hidden_native(environment, PACKAGE_C_SEARCHER_NATIVE.function());
            let c_root_searcher =
                register_hidden_native(environment, PACKAGE_C_ROOT_SEARCHER_NATIVE.function());
            environment.set_global_table_with_string_and_table_fields(
                library.name(),
                fields,
                [
                    ("config", PACKAGE_CONFIG.as_bytes()),
                    ("path", PACKAGE_PATH.as_bytes()),
                    ("cpath", PACKAGE_CPATH.as_bytes()),
                ],
                [
                    ("loaded", Vec::new()),
                    ("preload", Vec::new()),
                    (
                        "searchers",
                        vec![
                            (
                                Value::integer(1),
                                Value::native_function_index(preload_searcher),
                            ),
                            (
                                Value::integer(2),
                                Value::native_function_index(lua_searcher),
                            ),
                            (Value::integer(3), Value::native_function_index(c_searcher)),
                            (
                                Value::integer(4),
                                Value::native_function_index(c_root_searcher),
                            ),
                        ],
                    ),
                ],
            );
            if let Some(require) = require {
                environment.set_global("require", require);
            }
        }
        StdLib::Utf8 => {
            environment.set_global_table_with_string_fields(
                library.name(),
                fields,
                [("charpattern", UTF8_CHAR_PATTERN)],
            );
        }
        _ => {
            environment.set_global_table(library.name(), fields);
        }
    }
}

fn register_base_helpers(environment: &mut RuntimeEnvironment) -> NativeHelpers {
    let base_next = register_hidden_native(environment, BASE_NEXT_NATIVE.function());
    let base_ipairs_aux = register_hidden_native(environment, BASE_IPAIRS_AUX_NATIVE.function());
    NativeHelpers {
        base_next: Some(base_next),
        base_ipairs_aux: Some(base_ipairs_aux),
        ..NativeHelpers::default()
    }
}

fn register_string_helpers(environment: &mut RuntimeEnvironment) -> NativeHelpers {
    let string_gmatch_aux =
        register_hidden_native(environment, STRING_GMATCH_AUX_NATIVE.function());
    NativeHelpers {
        string_gmatch_aux: Some(string_gmatch_aux),
        ..NativeHelpers::default()
    }
}

fn register_coroutine_helpers() -> NativeHelpers {
    NativeHelpers {
        coroutine_registry: Some(Arc::new(Mutex::new(CoroutineRegistry::default()))),
        ..NativeHelpers::default()
    }
}

fn register_debug_helpers() -> NativeHelpers {
    NativeHelpers {
        debug_registry: Some(Arc::new(Mutex::new(None))),
        ..NativeHelpers::default()
    }
}

fn register_utf8_helpers(environment: &mut RuntimeEnvironment) -> NativeHelpers {
    let utf8_codes_aux_strict =
        register_hidden_native(environment, UTF8_CODES_AUX_STRICT_NATIVE.function());
    let utf8_codes_aux_lax =
        register_hidden_native(environment, UTF8_CODES_AUX_LAX_NATIVE.function());
    NativeHelpers {
        utf8_codes_aux_strict: Some(utf8_codes_aux_strict),
        utf8_codes_aux_lax: Some(utf8_codes_aux_lax),
        ..NativeHelpers::default()
    }
}

fn register_hidden_native(
    environment: &mut RuntimeEnvironment,
    function: elara_stdlib::NativeStdFunction,
) -> u32 {
    environment.push_native(move |context, args| {
        let mut runtime = InterpNativeRuntime {
            context,
            random_state: None,
            helpers: NativeHelpers::default(),
        };
        function(&mut runtime, args).map_err(native_error_to_runtime_error)
    })
}

#[derive(Clone, Default)]
struct NativeHelpers {
    base_next: Option<u32>,
    base_ipairs_aux: Option<u32>,
    coroutine_registry: Option<Arc<Mutex<CoroutineRegistry>>>,
    debug_registry: Option<Arc<Mutex<Option<u32>>>>,
    string_gmatch_aux: Option<u32>,
    utf8_codes_aux_strict: Option<u32>,
    utf8_codes_aux_lax: Option<u32>,
}

#[derive(Clone, Copy)]
struct RegisteredCoroutine {
    function: Option<u32>,
    status: ThreadStatus,
}

struct CoroutineRegistry {
    coroutines: Vec<RegisteredCoroutine>,
}

impl Default for CoroutineRegistry {
    fn default() -> Self {
        Self {
            coroutines: vec![RegisteredCoroutine {
                function: None,
                status: ThreadStatus::Running,
            }],
        }
    }
}

impl CoroutineRegistry {
    fn create(&mut self, function: Value) -> Value {
        let index = u32::try_from(self.coroutines.len()).expect("coroutine count must fit in u32");
        self.coroutines.push(RegisteredCoroutine {
            function: function.as_closure_index(),
            status: ThreadStatus::Runnable,
        });
        Value::thread_index(index)
    }

    fn begin_resume(&mut self, thread: Value) -> Result<Result<Value, Box<str>>, NativeError> {
        let index = thread.as_thread_index().ok_or(NativeErrorKind::TypeError {
            index: 1,
            expected: "thread",
        })? as usize;
        let coroutine =
            self.coroutines
                .get_mut(index)
                .ok_or_else(|| NativeErrorKind::RuntimeError {
                    message: "unknown coroutine".into(),
                })?;
        match coroutine.status {
            ThreadStatus::Runnable | ThreadStatus::Suspended => {
                coroutine.status = ThreadStatus::Running;
                let function = coroutine
                    .function
                    .ok_or_else(|| NativeErrorKind::RuntimeError {
                        message: "coroutine function is not registered".into(),
                    })?;
                Ok(Ok(Value::closure_index(function)))
            }
            ThreadStatus::Dead => Ok(Err("cannot resume dead coroutine".into())),
            ThreadStatus::Running => Ok(Err("cannot resume non-suspended coroutine".into())),
        }
    }

    fn close(&mut self, thread: Value) -> Result<Result<(), Box<str>>, NativeError> {
        let index = thread.as_thread_index().ok_or(NativeErrorKind::TypeError {
            index: 1,
            expected: "thread",
        })? as usize;
        let coroutine =
            self.coroutines
                .get_mut(index)
                .ok_or_else(|| NativeErrorKind::RuntimeError {
                    message: "unknown coroutine".into(),
                })?;
        match coroutine.status {
            ThreadStatus::Runnable | ThreadStatus::Suspended | ThreadStatus::Dead => {
                coroutine.status = ThreadStatus::Dead;
                Ok(Ok(()))
            }
            ThreadStatus::Running if index == 0 => Err(NativeError::lua_error(
                "cannot close main thread".to_owned().into_boxed_str(),
            )),
            ThreadStatus::Running => Err(NativeError::lua_error(
                "cannot close a running coroutine"
                    .to_owned()
                    .into_boxed_str(),
            )),
        }
    }

    fn finish_resume(&mut self, thread: Value) -> Result<(), NativeError> {
        let index = thread.as_thread_index().ok_or(NativeErrorKind::TypeError {
            index: 1,
            expected: "thread",
        })? as usize;
        let coroutine =
            self.coroutines
                .get_mut(index)
                .ok_or_else(|| NativeErrorKind::RuntimeError {
                    message: "unknown coroutine".into(),
                })?;
        coroutine.status = ThreadStatus::Dead;
        Ok(())
    }

    fn status(&self, thread: Value) -> Option<ThreadStatus> {
        let index = thread.as_thread_index()? as usize;
        self.coroutines.get(index).map(|coroutine| coroutine.status)
    }

    fn is_yieldable(&self, thread: Value) -> Option<bool> {
        let index = thread.as_thread_index()? as usize;
        Some(index != 0 && self.coroutines.get(index)?.status != ThreadStatus::Dead)
    }

    fn running(&self) -> (Value, bool) {
        (Value::thread_index(0), true)
    }
}

struct InterpNativeRuntime<'a, 'runtime> {
    context: &'a mut NativeContext<'runtime>,
    random_state: Option<Arc<Mutex<LuaRandomState>>>,
    helpers: NativeHelpers,
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

    fn intern_string(&mut self, bytes: &[u8]) -> Result<Value, NativeError> {
        Ok(self.context.intern_string(bytes))
    }

    fn short_string_bytes(&self, value: Value) -> Option<&[u8]> {
        self.context.short_string_bytes(value)
    }

    fn string_bytes(&self, value: Value) -> Option<&[u8]> {
        self.context.string_bytes(value)
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

    fn global_get(&mut self, name: &[u8]) -> Result<Value, NativeError> {
        self.context.global_get(name).map_err(|error| {
            NativeErrorKind::RuntimeError {
                message: error.to_string().into(),
            }
            .into()
        })
    }

    fn debug_registry(&mut self) -> Result<Value, NativeError> {
        let registry =
            self.helpers
                .debug_registry
                .clone()
                .ok_or_else(|| NativeErrorKind::RuntimeError {
                    message: "debug registry is not registered".into(),
                })?;
        let mut registry = registry.lock().map_err(|_| NativeErrorKind::RuntimeError {
            message: "debug registry lock poisoned".into(),
        })?;
        if let Some(registry) = *registry {
            return Ok(Value::table_index(registry));
        }
        let table = self.create_table(&[])?;
        let index = table
            .as_table_index()
            .ok_or_else(|| NativeErrorKind::RuntimeError {
                message: "debug registry allocation did not return a table".into(),
            })?;
        *registry = Some(index);
        Ok(table)
    }

    fn debug_getinfo(
        &mut self,
        target: DebugInfoTarget,
        options: Option<&[u8]>,
    ) -> Result<Value, NativeError> {
        let options = options.unwrap_or(b"flnSrtu");
        let result = match target {
            DebugInfoTarget::Level(level) => self.context.debug_info_for_level(level, options),
            DebugInfoTarget::Function(function) => {
                self.context.debug_info_for_function(function, options)
            }
        };
        result.map_err(runtime_error_to_native_error)
    }

    fn debug_getlocal(
        &mut self,
        level: i64,
        local: i64,
    ) -> Result<Option<(Value, Value)>, NativeError> {
        self.context
            .debug_getlocal(level, local)
            .map_err(runtime_error_to_native_error)
    }

    fn debug_getlocal_function(
        &mut self,
        function: Value,
        local: i64,
    ) -> Result<Option<Value>, NativeError> {
        self.context
            .debug_getlocal_function(function, local)
            .map_err(runtime_error_to_native_error)
    }

    fn debug_setlocal(
        &mut self,
        level: i64,
        local: i64,
        value: Value,
    ) -> Result<Option<Value>, NativeError> {
        self.context
            .debug_setlocal(level, local, value)
            .map_err(runtime_error_to_native_error)
    }

    fn debug_getupvalue(
        &mut self,
        function: Value,
        index: i64,
    ) -> Result<Option<(Value, Value)>, NativeError> {
        self.context
            .debug_getupvalue(function, index)
            .map_err(runtime_error_to_native_error)
    }

    fn debug_setupvalue(
        &mut self,
        function: Value,
        index: i64,
        value: Value,
    ) -> Result<Option<Value>, NativeError> {
        self.context
            .debug_setupvalue(function, index, value)
            .map_err(runtime_error_to_native_error)
    }

    fn debug_upvalueid(
        &mut self,
        function: Value,
        index: i64,
    ) -> Result<Option<Value>, NativeError> {
        self.context
            .debug_upvalueid(function, index)
            .map_err(runtime_error_to_native_error)
    }

    fn debug_upvaluejoin(
        &mut self,
        target_function: Value,
        target_index: i64,
        source_function: Value,
        source_index: i64,
    ) -> Result<bool, NativeError> {
        self.context
            .debug_upvaluejoin(target_function, target_index, source_function, source_index)
            .map_err(runtime_error_to_native_error)
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

    fn protected_call(
        &mut self,
        function: Value,
        args: &[Value],
    ) -> Result<Result<Vec<Value>, Box<str>>, NativeError> {
        Ok(self
            .context
            .protected_call(function, args)
            .map_err(|error| error.message().into()))
    }

    fn native_function(&self, library: StdLib, name: &str) -> Result<Value, NativeError> {
        match (library, name) {
            (StdLib::Base, "next") => self
                .helpers
                .base_next
                .map(Value::native_function_index)
                .ok_or_else(missing_native_helper),
            (StdLib::Base, "__ipairs_aux") => self
                .helpers
                .base_ipairs_aux
                .map(Value::native_function_index)
                .ok_or_else(missing_native_helper),
            (StdLib::String, "__gmatch_aux") => self
                .helpers
                .string_gmatch_aux
                .map(Value::native_function_index)
                .ok_or_else(missing_native_helper),
            (StdLib::Utf8, "__codes_aux_strict") => self
                .helpers
                .utf8_codes_aux_strict
                .map(Value::native_function_index)
                .ok_or_else(missing_native_helper),
            (StdLib::Utf8, "__codes_aux_lax") => self
                .helpers
                .utf8_codes_aux_lax
                .map(Value::native_function_index)
                .ok_or_else(missing_native_helper),
            _ => Err(NativeErrorKind::RuntimeError {
                message: "native helper is not registered".into(),
            }
            .into()),
        }
    }

    fn create_coroutine(&mut self, function: Value) -> Result<Value, NativeError> {
        if !function.is_closure() {
            return Err(NativeErrorKind::TypeError {
                index: 1,
                expected: "function",
            }
            .into());
        }
        let registry = self.helpers.coroutine_registry.clone().ok_or_else(|| {
            NativeErrorKind::RuntimeError {
                message: "coroutine registry is not registered".into(),
            }
        })?;
        let mut registry = registry.lock().map_err(|_| NativeErrorKind::RuntimeError {
            message: "coroutine registry lock poisoned".into(),
        })?;
        Ok(registry.create(function))
    }

    fn create_coroutine_wrapper(&mut self, function: Value) -> Result<Value, NativeError> {
        if !function.is_closure() {
            return Err(NativeErrorKind::TypeError {
                index: 1,
                expected: "function",
            }
            .into());
        }
        let thread = self.create_coroutine(function)?;
        let thread_index = thread
            .as_thread_index()
            .expect("created coroutine must return a thread handle");
        let helpers = self.helpers.clone();
        Ok(self.context.create_native_function(move |context, args| {
            let mut runtime = InterpNativeRuntime {
                context,
                random_state: None,
                helpers: helpers.clone(),
            };
            match runtime
                .resume_coroutine(Value::thread_index(thread_index), args)
                .map_err(native_error_to_runtime_error)?
            {
                Ok(values) => Ok(values),
                Err(message) => Err(RuntimeErrorKind::NativeFunctionError { message }.into()),
            }
        }))
    }

    fn close_coroutine(&mut self, thread: Value) -> Result<Result<(), Box<str>>, NativeError> {
        let registry = self.helpers.coroutine_registry.as_ref().ok_or_else(|| {
            NativeErrorKind::RuntimeError {
                message: "coroutine registry is not registered".into(),
            }
        })?;
        let mut registry = registry.lock().map_err(|_| NativeErrorKind::RuntimeError {
            message: "coroutine registry lock poisoned".into(),
        })?;
        registry.close(thread)
    }

    fn resume_coroutine(
        &mut self,
        thread: Value,
        args: &[Value],
    ) -> Result<Result<Vec<Value>, Box<str>>, NativeError> {
        let registry = self.helpers.coroutine_registry.clone().ok_or_else(|| {
            NativeErrorKind::RuntimeError {
                message: "coroutine registry is not registered".into(),
            }
        })?;
        let function = {
            let mut registry = registry.lock().map_err(|_| NativeErrorKind::RuntimeError {
                message: "coroutine registry lock poisoned".into(),
            })?;
            match registry.begin_resume(thread)? {
                Ok(function) => function,
                Err(message) => return Ok(Err(message)),
            }
        };
        let result = self
            .context
            .protected_call(function, args)
            .map_err(|error| error.message().into());
        {
            let mut registry = registry.lock().map_err(|_| NativeErrorKind::RuntimeError {
                message: "coroutine registry lock poisoned".into(),
            })?;
            registry.finish_resume(thread)?;
        }
        Ok(result)
    }

    fn yield_coroutine(&mut self, _args: &[Value]) -> Result<Vec<Value>, NativeError> {
        Err(NativeError::lua_error(
            "attempt to yield from outside a coroutine",
        ))
    }

    fn running_thread(&self) -> Result<(Value, bool), NativeError> {
        let registry = self.helpers.coroutine_registry.as_ref().ok_or_else(|| {
            NativeErrorKind::RuntimeError {
                message: "coroutine registry is not registered".into(),
            }
        })?;
        let registry = registry.lock().map_err(|_| NativeErrorKind::RuntimeError {
            message: "coroutine registry lock poisoned".into(),
        })?;
        Ok(registry.running())
    }

    fn thread_is_yieldable(&self, thread: Value) -> Result<bool, NativeError> {
        let registry = self.helpers.coroutine_registry.as_ref().ok_or_else(|| {
            NativeErrorKind::RuntimeError {
                message: "coroutine registry is not registered".into(),
            }
        })?;
        let registry = registry.lock().map_err(|_| NativeErrorKind::RuntimeError {
            message: "coroutine registry lock poisoned".into(),
        })?;
        registry.is_yieldable(thread).ok_or_else(|| {
            NativeErrorKind::TypeError {
                index: 1,
                expected: "thread",
            }
            .into()
        })
    }

    fn thread_status(&self, thread: Value) -> Result<ThreadStatus, NativeError> {
        let registry = self.helpers.coroutine_registry.as_ref().ok_or_else(|| {
            NativeErrorKind::RuntimeError {
                message: "coroutine registry is not registered".into(),
            }
        })?;
        let registry = registry.lock().map_err(|_| NativeErrorKind::RuntimeError {
            message: "coroutine registry lock poisoned".into(),
        })?;
        registry.status(thread).ok_or_else(|| {
            NativeErrorKind::TypeError {
                index: 1,
                expected: "thread",
            }
            .into()
        })
    }
}

fn missing_native_helper() -> NativeError {
    NativeErrorKind::RuntimeError {
        message: "native helper is not registered".into(),
    }
    .into()
}

fn runtime_error_to_native_error(error: elara_interp::RuntimeError) -> NativeError {
    NativeErrorKind::RuntimeError {
        message: error.message().into(),
    }
    .into()
}

fn native_error_to_runtime_error(error: NativeError) -> elara_interp::RuntimeError {
    RuntimeErrorKind::NativeFunctionError {
        message: error.message().into(),
    }
    .into()
}
