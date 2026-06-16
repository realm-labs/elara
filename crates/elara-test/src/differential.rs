//! Differential runner helpers.

use std::{
    env, io,
    io::Write,
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

use elara_api::Lua;
use elara_core::{LuaFloat, Value, ValueTag};

/// Environment variable used to locate the official Lua executable.
pub const OFFICIAL_LUA_ENV: &str = "ELARA_LUA";

/// Runner for an official Lua executable.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LuaRunner {
    executable: PathBuf,
}

impl LuaRunner {
    /// Creates a runner for an explicit executable path.
    #[must_use]
    pub fn new(executable: impl Into<PathBuf>) -> Self {
        Self {
            executable: executable.into(),
        }
    }

    /// Creates a runner from `ELARA_LUA` when configured.
    #[must_use]
    pub fn from_env() -> Option<Self> {
        env::var_os(OFFICIAL_LUA_ENV).map(Self::new)
    }

    /// Official Lua executable path.
    #[must_use]
    pub fn executable(&self) -> &Path {
        &self.executable
    }

    /// Runs Lua source through the official executable's stdin.
    pub fn run_source(&self, source: &str) -> io::Result<RunOutput> {
        let mut child = Command::new(&self.executable)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;
        child
            .stdin
            .as_mut()
            .expect("stdin was requested")
            .write_all(source.as_bytes())?;
        let output = child.wait_with_output()?;
        Ok(RunOutput {
            success: output.status.success(),
            stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        })
    }

    /// Runs Lua source and serializes returned primitive values to stdout.
    pub fn run_source_values(&self, source: &str) -> io::Result<RunOutput> {
        self.run_source(&official_value_wrapper(source))
    }
}

/// Output from running one Lua implementation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RunOutput {
    /// Whether the process or evaluation succeeded.
    pub success: bool,
    /// Captured stdout.
    pub stdout: String,
    /// Captured stderr.
    pub stderr: String,
}

impl RunOutput {
    /// Success/error class for coarse differential comparisons.
    #[must_use]
    pub fn class(&self) -> RunClass {
        if self.success && self.stderr.is_empty() {
            RunClass::Success
        } else {
            RunClass::Error
        }
    }
}

/// Coarse run result class.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RunClass {
    /// Run completed successfully.
    Success,
    /// Run failed.
    Error,
}

/// Differential comparison output for one source string.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DifferentialComparison {
    /// Official Lua run output.
    pub official: RunOutput,
    /// Elara run output.
    pub elara: RunOutput,
}

impl DifferentialComparison {
    /// Returns true when both implementations agree on success vs error.
    #[must_use]
    pub fn same_error_class(&self) -> bool {
        self.official.class() == self.elara.class()
    }
}

/// Differential runner comparing official Lua with Elara.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DifferentialRunner {
    official: LuaRunner,
}

impl DifferentialRunner {
    /// Creates a differential runner from an official Lua runner.
    #[must_use]
    pub const fn new(official: LuaRunner) -> Self {
        Self { official }
    }

    /// Compares one source string.
    pub fn compare_source(&self, source: &str) -> io::Result<DifferentialComparison> {
        let official = self.official.run_source(source)?;
        let elara_result = Lua::new().eval(source);
        let elara = match elara_result {
            Ok(_) => RunOutput {
                success: true,
                stdout: String::new(),
                stderr: String::new(),
            },
            Err(error) => RunOutput {
                success: false,
                stdout: String::new(),
                stderr: format!("{error:?}"),
            },
        };
        Ok(DifferentialComparison { official, elara })
    }

    /// Compares one source string, serializing successful primitive return values.
    pub fn compare_source_values(&self, source: &str) -> io::Result<DifferentialComparison> {
        let official = self.official.run_source_values(source)?;
        let elara_result = Lua::new().eval(source);
        let elara = match elara_result {
            Ok(values) => match serialize_values(&values) {
                Ok(stdout) => RunOutput {
                    success: true,
                    stdout,
                    stderr: String::new(),
                },
                Err(message) => RunOutput {
                    success: false,
                    stdout: String::new(),
                    stderr: message,
                },
            },
            Err(error) => RunOutput {
                success: false,
                stdout: String::new(),
                stderr: format!("{error:?}"),
            },
        };
        Ok(DifferentialComparison { official, elara })
    }
}

fn serialize_values(values: &[Value]) -> Result<String, String> {
    let mut output = format!("{}\n", values.len());
    for value in values {
        match value.tag() {
            ValueTag::Nil => output.push_str("nil\n"),
            ValueTag::Bool => {
                let bit = if value
                    .as_bool()
                    .expect("boolean tag should expose boolean payload")
                {
                    '1'
                } else {
                    '0'
                };
                output.push_str("bool:");
                output.push(bit);
                output.push('\n');
            }
            ValueTag::Integer => {
                output.push_str("int:");
                output.push_str(
                    &value
                        .as_integer()
                        .expect("integer tag should expose integer payload")
                        .to_string(),
                );
                output.push('\n');
            }
            ValueTag::Float => {
                output.push_str("float:");
                output.push_str(&format_float(
                    value
                        .as_float()
                        .expect("float tag should expose float payload"),
                ));
                output.push('\n');
            }
            tag => {
                return Err(format!("unsupported differential return value tag: {tag:?}"));
            }
        }
    }
    Ok(output)
}

fn format_float(value: LuaFloat) -> String {
    format!("{value:.17}")
}

fn official_value_wrapper(source: &str) -> String {
    format!(
        r#"
local chunk, load_error = load({}, "elara-differential", "t")
if not chunk then
  error(load_error)
end

local values = table.pack(chunk())
io.write(tostring(values.n), "\n")
for index = 1, values.n do
  local value = values[index]
  local kind = type(value)
  if kind == "nil" then
    io.write("nil\n")
  elseif kind == "boolean" then
    io.write("bool:", value and "1" or "0", "\n")
  elseif kind == "number" then
    if math.type and math.type(value) == "integer" then
      io.write("int:", string.format("%d", value), "\n")
    else
      io.write("float:", string.format("%.17f", value), "\n")
    end
  else
    error("unsupported differential return value type: " .. kind)
  end
end
"#,
        lua_long_literal(source)
    )
}

fn lua_long_literal(value: &str) -> String {
    for level in 0..=16 {
        let delimiter = format!("]{}]", "=".repeat(level));
        if !value.contains(&delimiter) {
            return format!(
                "[{}[{}]{}]",
                "=".repeat(level),
                value,
                "=".repeat(level)
            );
        }
    }
    panic!("fixture source contains too many long-string delimiters");
}

#[cfg(test)]
mod tests {
    use super::{
        DifferentialComparison, LuaRunner, RunClass, RunOutput, lua_long_literal, serialize_values,
    };
    #[cfg(unix)]
    use super::DifferentialRunner;
    use elara_core::Value;

    #[test]
    fn differential_runner_compares_success_and_error_classes() {
        let comparison = DifferentialComparison {
            official: RunOutput {
                success: true,
                stdout: String::new(),
                stderr: String::new(),
            },
            elara: RunOutput {
                success: false,
                stdout: String::new(),
                stderr: String::from("boom"),
            },
        };

        assert_eq!(comparison.official.class(), RunClass::Success);
        assert_eq!(comparison.elara.class(), RunClass::Error);
        assert!(!comparison.same_error_class());
    }

    #[test]
    fn differential_serializer_formats_primitive_values() {
        assert_eq!(
            serialize_values(&[
                Value::nil(),
                Value::boolean(true),
                Value::boolean(false),
                Value::integer(42),
                Value::float(1.5),
            ]),
            Ok("5\nnil\nbool:1\nbool:0\nint:42\nfloat:1.50000000000000000\n".to_owned())
        );
    }

    #[test]
    fn differential_wrapper_uses_safe_long_literal_delimiters() {
        assert_eq!(lua_long_literal("return 42"), "[[return 42]]");
        assert_eq!(lua_long_literal("]]"), "[=[]]]=]");
    }

    #[test]
    fn differential_runner_records_configured_executable_path() {
        let runner = LuaRunner::new("/path/to/lua");

        assert_eq!(runner.executable(), std::path::Path::new("/path/to/lua"));
    }

    #[test]
    #[cfg(unix)]
    fn differential_runner_invokes_configured_executable() {
        let script = fake_lua_script("success", "cat >/dev/null\nprintf 'ok\\n'\nexit 0\n");
        let runner = LuaRunner::new(&script);

        let output = runner
            .run_source("return 42")
            .expect("fake lua should execute");

        assert!(output.success);
        assert_eq!(output.stdout, "ok\n");
    }

    #[test]
    #[cfg(unix)]
    fn differential_runner_compares_official_and_elara_classes() {
        let script = fake_lua_script("compare", "cat >/dev/null\nexit 0\n");
        let runner = DifferentialRunner::new(LuaRunner::new(&script));

        let comparison = runner
            .compare_source("return 42")
            .expect("comparison should execute");

        assert!(comparison.same_error_class());
    }

    #[test]
    #[cfg(unix)]
    fn differential_runner_compares_serialized_success_values() {
        let script = fake_lua_script(
            "compare_values",
            "cat >/dev/null\nprintf '1\\nint:42\\n'\nexit 0\n",
        );
        let runner = DifferentialRunner::new(LuaRunner::new(&script));

        let comparison = runner
            .compare_source_values("return 42")
            .expect("comparison should execute");

        assert_eq!(comparison.elara.stdout, "1\nint:42\n");
        assert_eq!(comparison.official.stdout, comparison.elara.stdout);
    }

    #[cfg(unix)]
    fn fake_lua_script(name: &str, body: &str) -> std::path::PathBuf {
        use std::{fs, os::unix::fs::PermissionsExt};

        let path = std::env::temp_dir().join(format!(
            "elara_fake_lua_{name}_{}_{}.sh",
            std::process::id(),
            unique_suffix()
        ));
        fs::write(&path, format!("#!/bin/sh\n{body}")).expect("fake lua script should be written");
        let mut permissions = fs::metadata(&path)
            .expect("fake lua metadata should be readable")
            .permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&path, permissions).expect("fake lua script should be executable");
        path
    }

    #[cfg(unix)]
    fn unique_suffix() -> u128 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system time should be after epoch")
            .as_nanos()
    }
}
