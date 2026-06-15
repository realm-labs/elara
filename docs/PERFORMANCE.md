# Elara Performance Notes

Status: M20 release report
Last updated: 2026-06-15

This document records lightweight performance checkpoints and the M20 release
performance report. The benchmark harness is intentionally simple and prints
CSV rows from a custom `cargo bench` target.

## M15 Interpreter Baseline

The current benchmark harness runs through the public API interpreter path with
stable micro and macro workloads.

Command:

```bash
cargo bench -p elara-bench
```

Latest local result:

```text
benchmark,iterations,total_ns,ns_per_iter,result_count
micro_arithmetic_for_loop,200,6921250,34606,1
micro_table_access,200,5952333,29761,1
micro_zero_arg_calls,200,6435792,32178,1
micro_string_stdlib,200,5944625,29723,1
macro_arithmetic_accumulator,25,931292,37251,1
macro_table_build_and_sum,25,1080459,43218,1
macro_string_patterns,25,753958,30158,1
```

M15 optimizations delivered:

- Checked-once unsafe stack helpers on interpreter hot register paths.
- Version-guarded runtime table inline caches for raw, integer, and global table reads.
- `ADD_INT` superinstruction for register plus unsigned integer immediate addition.

## M20 Release Performance Report

Command:

```bash
cargo bench -p elara-bench
```

Methodology notes:

- `interpreter_api` measures the public `Lua::eval` path once per iteration,
  including source loading, parsing, compilation, and interpreter execution.
- `jit_api` measures the same public API path with `JitMode::Always`, so it
  includes JIT eligibility checks and compilation/fallback overhead.
- `official_lua` is measured with `lua5.5` from `PATH`; the harness writes a
  temporary Lua script that loads each workload once and times repeated calls
  with `os.clock`.
- Because the official Lua row excludes per-iteration parse/compile work while
  the Elara API rows include it, the official ratios are useful release
  pressure, not a pure VM-to-VM comparison.

Latest local result:

```text
mode,benchmark,iterations,total_ns,ns_per_iter,result_count
interpreter_api,api_return_constant,500,23983083,47966,1
jit_api,api_return_constant,500,20562167,41124,1
official_lua,api_return_constant,500,56000,112,1
interpreter_api,micro_arithmetic_for_loop,200,10194708,50973,1
jit_api,micro_arithmetic_for_loop,200,7211917,36059,1
official_lua,micro_arithmetic_for_loop,200,100000,500,1
interpreter_api,micro_table_access,200,12710083,63550,1
jit_api,micro_table_access,200,9823000,49115,1
official_lua,micro_table_access,200,75000,375,1
interpreter_api,micro_zero_arg_calls,200,15523209,77616,1
jit_api,micro_zero_arg_calls,200,3173417,15867,1
official_lua,micro_zero_arg_calls,200,42000,210,1
interpreter_api,micro_string_stdlib,200,11011458,55057,1
jit_api,micro_string_stdlib,200,11541250,57706,1
official_lua,micro_string_stdlib,200,41000,205,1
interpreter_api,macro_arithmetic_accumulator,25,1863208,74528,1
jit_api,macro_arithmetic_accumulator,25,946875,37875,1
official_lua,macro_arithmetic_accumulator,25,54000,2160,1
interpreter_api,macro_table_build_and_sum,25,1652875,66115,1
jit_api,macro_table_build_and_sum,25,1535417,61416,1
official_lua,macro_table_build_and_sum,25,48000,1920,1
interpreter_api,macro_string_patterns,25,1688208,67528,1
jit_api,macro_string_patterns,25,1207583,48303,1
official_lua,macro_string_patterns,25,12000,480,1
```

Summary:

- API overhead is visible in `api_return_constant`: the interpreter API path is
  roughly 428x the official Lua loop timing, and the JIT API path is roughly
  367x.
- JIT improves API-path arithmetic and simple call workloads in this local run:
  arithmetic loop is 0.71x interpreter time, zero-arg calls are 0.20x, and the
  macro arithmetic accumulator is 0.51x.
- JIT has little effect on table and string workloads that currently fall back
  or spend most time outside the narrow compiled arithmetic subset.
- Official Lua remains much faster on every selected workload, ranging from
  roughly 17.5x faster than the JIT API path on macro arithmetic to over 200x
  faster on string/API-overhead-heavy paths.

Known gaps after the release performance report:

- The current Elara harness measures public API evaluation overhead rather than
  a precompiled Proto execution loop; a future VM-only benchmark should separate
  parse/compile/API costs from interpreter dispatch costs.
- Official Lua timing uses `os.clock` inside a generated Lua script; this is
  stable enough for a release checkpoint but not a statistical benchmark suite.
- The selected JIT subset is intentionally narrow; table, string, coroutine,
  and environment-aware workloads still mostly exercise interpreter fallback.
