//! Benchmark harness support for Elara.
//!
//! This crate is for benchmark-only helpers and runtime performance harnesses.
//! It must not provide production APIs or become a dependency of runtime crates.
//!
//! Benchmarks should exercise the same public or internal execution paths used
//! by Elara rather than carrying independent VM semantics.

use std::time::{Duration, Instant};

use elara_api::{EvalError, Lua};

/// One benchmark workload.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BenchmarkCase {
    /// Stable benchmark name.
    pub name: &'static str,
    /// Lua source executed by this benchmark.
    pub source: &'static str,
    /// Number of iterations for one benchmark run.
    pub iterations: u32,
}

/// Result from one benchmark case.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BenchmarkResult {
    /// Stable benchmark name.
    pub name: &'static str,
    /// Number of iterations completed.
    pub iterations: u32,
    /// Total elapsed time.
    pub elapsed: Duration,
    /// Return value count from the final iteration.
    pub last_result_count: usize,
}

/// Interpreter microbenchmarks.
pub const MICRO_BENCHMARKS: &[BenchmarkCase] = &[
    BenchmarkCase {
        name: "micro_arithmetic_for_loop",
        source: "local x = 0\nfor i = 1, 50 do x = x + i end\nreturn x",
        iterations: 200,
    },
    BenchmarkCase {
        name: "micro_table_access",
        source: "local t = { a = 1, [2] = 3 }\nlocal x = t.a + t[2]\nt[3] = x\nreturn t[3]",
        iterations: 200,
    },
    BenchmarkCase {
        name: "micro_zero_arg_calls",
        source: "local function value() return 1 end\nreturn value() + value()",
        iterations: 200,
    },
    BenchmarkCase {
        name: "micro_string_stdlib",
        source: "return string.len(string.reverse(\"abcdef\"))",
        iterations: 200,
    },
];

/// Larger representative Lua workloads.
pub const MACRO_BENCHMARKS: &[BenchmarkCase] = &[
    BenchmarkCase {
        name: "macro_arithmetic_accumulator",
        source: concat!(
            "local total = 0\n",
            "for i = 1, 400 do\n",
            "  total = total + i\n",
            "end\n",
            "return total",
        ),
        iterations: 25,
    },
    BenchmarkCase {
        name: "macro_table_build_and_sum",
        source: concat!(
            "local t = {}\n",
            "for i = 1, 40 do t[i] = i end\n",
            "local total = 0\n",
            "for i = 1, 40 do total = total + t[i] end\n",
            "return total",
        ),
        iterations: 25,
    },
    BenchmarkCase {
        name: "macro_string_patterns",
        source: "local s = \"a1 b2 c3 d4\"\nreturn string.gsub(s, \"%a\", \"x\")",
        iterations: 25,
    },
];

/// Runs one benchmark case through the public API interpreter path.
pub fn run_case(case: BenchmarkCase) -> Result<BenchmarkResult, EvalError> {
    let lua = Lua::new();
    let mut last_result_count = 0;
    let started = Instant::now();
    for _ in 0..case.iterations {
        last_result_count = lua.eval(case.source)?.len();
    }
    Ok(BenchmarkResult {
        name: case.name,
        iterations: case.iterations,
        elapsed: started.elapsed(),
        last_result_count,
    })
}

/// Returns all benchmark cases in stable display order.
pub fn all_benchmarks() -> impl Iterator<Item = BenchmarkCase> {
    MICRO_BENCHMARKS
        .iter()
        .chain(MACRO_BENCHMARKS.iter())
        .copied()
}
