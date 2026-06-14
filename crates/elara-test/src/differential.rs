//! Differential runner helpers.

use std::{
    env, io,
    io::Write,
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

use elara_api::Lua;

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
    pub const fn class(&self) -> RunClass {
        if self.success {
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
}

#[cfg(test)]
mod tests {
    use super::{DifferentialComparison, DifferentialRunner, LuaRunner, RunClass, RunOutput};

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
