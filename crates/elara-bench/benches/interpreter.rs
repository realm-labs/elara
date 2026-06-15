use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use elara_bench::{BenchmarkCase, BenchmarkResult, all_benchmarks, run_case, run_case_jit};

fn main() {
    println!("mode,benchmark,iterations,total_ns,ns_per_iter,result_count");
    let official_lua = official_lua();

    for case in all_benchmarks() {
        print_result(
            "interpreter_api",
            run_case(case).expect("interpreter benchmark case must execute successfully"),
        );
        print_result(
            "jit_api",
            run_case_jit(case).expect("JIT benchmark case must execute successfully"),
        );

        if let Some(executable) = official_lua.as_deref() {
            print_result(
                "official_lua",
                run_official_case(case, executable)
                    .expect("official Lua benchmark case must execute successfully"),
            );
        }
    }

    if official_lua.is_none() {
        eprintln!("official_lua_unavailable: set ELARA_LUA or install lua5.5 for reference rows");
    }
}

fn print_result(mode: &str, result: BenchmarkResult) {
    let total_ns = result.elapsed.as_nanos();
    let ns_per_iter = total_ns / u128::from(result.iterations);
    println!(
        "{},{},{},{},{},{}",
        mode, result.name, result.iterations, total_ns, ns_per_iter, result.last_result_count
    );
}

fn official_lua() -> Option<PathBuf> {
    if let Some(path) = std::env::var_os("ELARA_LUA").map(PathBuf::from) {
        return command_available(&path).then_some(path);
    }

    let candidate = PathBuf::from("lua5.5");
    command_available(&candidate).then_some(candidate)
}

fn command_available(command: &Path) -> bool {
    Command::new(command)
        .arg("-v")
        .output()
        .is_ok_and(|output| output.status.success())
}

fn run_official_case(case: BenchmarkCase, executable: &Path) -> Result<BenchmarkResult, String> {
    let work_dir = std::env::temp_dir().join(format!(
        "elara-bench-official-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|error| error.to_string())?
            .as_nanos()
    ));
    fs::create_dir_all(&work_dir).map_err(|error| error.to_string())?;
    let script = work_dir.join(format!("{}.lua", case.name));
    fs::write(&script, official_script(case)).map_err(|error| error.to_string())?;

    let output = Command::new(executable)
        .arg(&script)
        .output()
        .map_err(|error| error.to_string());
    let _ = fs::remove_dir_all(&work_dir);

    let output = output?;
    if !output.status.success() {
        return Err(format!(
            "official Lua benchmark failed for {}\nstatus: {}\nstdout:\n{}\nstderr:\n{}",
            case.name,
            output.status,
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    let stdout = String::from_utf8(output.stdout).map_err(|error| error.to_string())?;
    let (elapsed_ns, result_count) = stdout
        .trim()
        .split_once(',')
        .ok_or_else(|| format!("unexpected official Lua output for {}: {stdout}", case.name))?;
    let elapsed_ns = elapsed_ns
        .parse::<u64>()
        .map_err(|error| error.to_string())?;
    let last_result_count = result_count
        .parse::<usize>()
        .map_err(|error| error.to_string())?;

    Ok(BenchmarkResult {
        name: case.name,
        iterations: case.iterations,
        elapsed: Duration::from_nanos(elapsed_ns),
        last_result_count,
    })
}

fn official_script(case: BenchmarkCase) -> String {
    format!(
        r#"
local __bench = function()
{}
end

local __started = os.clock()
local __result_count = 0
for __i = 1, {} do
  local __results = {{ __bench() }}
  __result_count = #__results
end
local __elapsed = os.clock() - __started
io.write(string.format("%d,%d", math.floor(__elapsed * 1000000000 + 0.5), __result_count))
"#,
        case.source, case.iterations
    )
}
