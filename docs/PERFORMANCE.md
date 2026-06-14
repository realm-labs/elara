# Elara Performance Notes

Status: M15 baseline
Last updated: 2026-06-14

This document records lightweight performance checkpoints. It is not a release
performance report; M20 owns the final interpreter/API/JIT comparison report.

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

Known gaps before the release performance report:

- The harness does not yet run the same workloads under official Lua for an
  automated ratio.
- The selected superinstruction set is intentionally narrow; more bytecode
  frequency data should be gathered from larger conformance and application
  workloads before adding more combined operations.
- JIT comparisons start in M16 and are finalized in M20.
