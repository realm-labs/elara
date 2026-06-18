# Elara Progress

Status: Rolling current-state document  
Last updated: 2026-06-18
Current target: latest stable Lua, currently Lua 5.5 / Lua 5.5.0  
Current milestone: M20 Release Hardening and 1.0 Candidate
Current step: Product gap work after M20.4

This document is for orientation. It is not a changelog. When work progresses,
replace stale status with the current state instead of appending history.

## Current Snapshot

Elara has a Cargo workspace skeleton with project documentation, quality gates,
documented crate boundaries, and a single current Lua language target in
`elara-core`. Core source span and diagnostic primitives are available, and the
test fixture/conformance/differential directory layout exists with a snapshot
baseline. Primitive runtime values, the basic GC skeleton, Lua strings with
short-string interning, table array/hash storage with metadata versioning, Lua
tokenization, expression parsing, statement parsing, and parser snapshot/error
coverage are implemented. Primitive values now also include an opaque
light-userdata representation for runtime identity values such as debug
upvalue identifiers. The initial bytecode prototype, instruction encoding,
opcode set, constant pool, metadata placeholders, builder, and disassembler are
implemented, along with local-variable debug descriptors, initial bytecode
verification, internal bytecode dump/load with magic and version checks,
recursive prototype-tree serialization, explicit official Lua binary chunk
detection with unsupported-format refusal, and simple expression codegen.
VM/thread stack primitives and primitive arithmetic bytecode execution are
implemented, and simple source chunks can be evaluated through the compile and
interpreter path. Local variables and assignment basics are implemented.
Simple function Protos and fixed-parameter Lua calls are implemented, including
nil fill for missing fixed arguments. Closures, shared runtime upvalue cells,
and captured outer local reads are implemented.
Anonymous vararg functions can receive call arguments and lower `...` for the
first requested value. Fixed-count Lua calls can receive multiple return values.
Named vararg tables are compiled to `VARARG_TABLE` and executed through
runtime-owned table storage. Recursive function self-references compile and
evaluate through shared runtime closure storage. Conditional branches compile
and execute through
`TEST`/`JMP` bytecode. `while`, `repeat`, and `break` compile and execute
through branch bytecode. Numeric for loops compile and execute for integer and
float control values with positive and negative steps. Generic for loops compile
and execute through the iterator-call protocol. Table constructors compile and
execute with array, record, and keyed fields. Raw table access compiles and
executes for bracket and field syntax, generic table get/set bytecode, integer
index fast paths, hash keys, and nil assignment clearing. Primitive runtime
table storage is centralized and can store metatable links for upcoming
metamethod dispatch. Table-valued and function-valued `__index` and
`__newindex` chains work in the primitive runtime table slow path. Function
metamethods for the primitive interpreter's currently executed arithmetic
opcodes work for table operands. Comparison opcodes execute with raw numeric
and string comparison plus table metamethod fallback. The simple compiler now lowers
equality, inequality, and relational comparison expressions to those comparison
opcodes. The bytecode and interpreter now support boolean `NOT`, and the simple
compiler lowers unary operators without clobbering operand registers, including
unary `not` expressions with Lua truthiness. The simple compiler also emits
binary arithmetic, comparison, and concatenation expression results into fresh
registers so locals reused later are not clobbered. The simple compiler now
also lowers `and` and `or` expressions with value-preserving short-circuit
semantics. `LEN` executes for runtime strings, runtime tables with raw array
length, and `__len` closure fallback. `CALL` can
invoke function-valued `__call` fallback for table operands. `CONCAT` executes
for runtime strings and numeric operands, including long string results, and
can invoke `__concat` closure fallback. The simple
compiler lowers declared and implicit global reads/writes through `_ENV` table
access, `DECL_GLOBAL` checks global declaration initialization against the
runtime environment, and the primitive interpreter keeps a shared runtime `_ENV`
table across Lua closure calls. Global declarations in nested simple-compiler
blocks are scoped to those blocks and can shadow outer collective declarations.
Default chunk `_ENV` upvalue behavior is implemented for the simple compiler,
including nested function capture of the default environment. Direct global
bytecode and exposed `_ENV` table access share one runtime-owned global table.
Local and captured `_ENV` tables are used for global reads, writes, and
declaration checks in the simple compiler. `global function` declarations
compile and execute with declaration-time already-defined checks. The simple
compiler reports Lua-style diagnostics when `_ENV` is itself declared global and
another global access would need it as the environment. Structured runtime
errors now preserve a
stable runtime error kind, display message, and traceback frame metadata, and
primitive Lua closure calls attach child frames when errors propagate out.
Primitive protected execution can catch structured runtime errors at an
explicit protected-call boundary. Primitive bytecode coroutines can transition
through runnable, running, suspended, and dead states, yield from Lua frames,
resume with values, and propagate errors through dead coroutine status.
To-be-closed locals now lower to `TBC`/`CLOSE`, and primitive normal-return
close paths can validate and invoke `__close` metamethods. Runtime error
unwinding runs pending close methods before returning the original error when
close succeeds, and primitive coroutines keep close variables alive across
yield before closing them on finish. The primitive interpreter can represent
runtime-registered native functions as callable values, dispatch `CALL` to
native or Lua functions, and thread the native registry through Lua closure,
metamethod, table slow-path, generic-for, and close-metamethod calls. Primitive
execution also accepts a `RuntimeEnvironment` that can seed initial globals,
including callable native globals, before running a Proto. Runtime native
registries can store closure-backed host functions, which allows API adapters to
wrap stdlib-native errors without making `elara-stdlib` depend on
`elara-interp`. `RuntimeEnvironment` can also seed table-valued globals with
native fields, enabling module-shaped entries such as `math.abs`. The
standard-library crate now exposes a profile/set/registry framework plus
generic global registration adapters, and contains descriptor-based essential
base, table, math, and string library entries. The stdlib crate also exposes
executable native specs for the currently implemented math functions `abs`,
`acos`, `asin`, `atan`, `ceil`, `cos`, `deg`, `exp`, `floor`, `fmod`,
`frexp`, `ldexp`, `log`, `max`, `min`, `modf`, `rad`, `random`,
`randomseed`, `sin`, `sqrt`, `tan`, `tointeger`, `type`, and `ult`.
The API layer can build a primitive `RuntimeEnvironment` from implemented
stdlib native specs, including shared reseedable math RNG state and math
constants `pi`, `huge`, `maxinteger`, and `mininteger`, and simple source
evaluation can run with a selected stdlib profile for supported native paths.
Base stdlib natives `assert`, safe unsupported `collectgarbage`, safe
unsupported `dofile`, `error`, `getmetatable`, `ipairs`, safe unsupported
`load`, safe unsupported `loadfile`, `next`, metamethod-backed `pairs`,
`pcall`, `print`, `rawequal`, `rawget`, `rawlen`, `rawset`, numeric and count
forms of `select`, `setmetatable`, `tonumber`, `tostring`, `type`,
`@on`/`@off` controlled host-warning `warn`, and `xpcall` are executable, and
API stdlib profile registration now installs base natives plus the `_G`
global-table alias and `_VERSION` string as direct globals while keeping module
libraries table-shaped. Native calls now
receive a `NativeContext` that can allocate and inspect runtime-owned short strings,
allocate runtime-owned tables, read/write raw runtime table entries, get/set
runtime table metatable links, traverse raw runtime table entries, write host
output for `print`, and return registered native helper functions for iterator
factories, perform protected calls of runtime callable values, and register
callable native functions during execution.
The primitive runtime and API native bridge can now allocate and inspect
runtime-owned long strings as well as short strings, and initial global table
string fields can be seeded with long string values.
The simple compiler and primitive `LOAD_STRING` path now preserve long string
literals through runtime long-string allocation instead of rejecting literals
larger than the short-string interning threshold.
`elara-stdlib` native functions now receive a crate-local `NativeRuntime` trait,
and the API bridge adapts it to the interpreter context without making stdlib
depend on interpreter internals. Remaining executable base, table, math, and
string functions, broader API surface, JIT, C API, conformance, and benchmark
implementation work remain. String natives `string.byte`, `string.char`,
literal-search `string.find`, basic `%s` with width/precision modifiers,
width, left-adjust, and zero-padding integer-family conversions, signed
decimal `+`/space flags, alternate-form octal/hex integer flags, integer
precision, float width/precision/sign/alternate-form flags for
`%f`/`%e`/`%E`/`%g`/`%G`, `%a`/`%A` width/precision/sign/alternate-form
flags, `%c`/`%p` width and left-adjust modifiers, `%q` including finite float
hex literals, and escaped-percent `string.format`, `.` wildcard,
`^`/`$` anchor, `%` character-class, bracket-class, quantifier, `%b`
balanced-delimiter, and `%f` frontier pattern matching for `string.find`,
`string.match`, and `string.gsub`, capture back-references, position captures,
literal string-replacement `string.gsub`, capture-returning `string.find` and
`string.match`, replacement captures for string-replacement `string.gsub`,
table/function replacement values for `string.gsub`, callable and generic-for
`string.gmatch` with Lua-style leading-caret behavior and empty-match advancement, `string.len`,
`string.lower`, `string.upper`, `string.reverse`, `string.rep`, `string.pack`,
`string.packsize`, `string.unpack`, and `string.sub` are executable and
covered through stdlib-backed API evaluation.
Table natives `table.concat`, `table.insert`, `table.move`, `table.pack`,
`table.remove`, default and custom-comparator `table.sort`, and
`table.unpack` are executable and covered through stdlib-backed API
evaluation.
Generic-for lowering now preserves call-expression multiple returns in iterator
protocol registers, and the primitive interpreter can call native iterator
functions from `TFOR_CALL`, enabling stdlib-backed `ipairs` and `pairs` loops.
Ordinary call-expression lowering preserves local callable values across
assignment calls, and generic-for iterator calls now honor `__call`
metamethod-backed table iterators.
Final return call-expression lowering now emits open call/open return bytecode,
and the primitive interpreter grows the stack for dynamic open-call results when
needed, so direct returns preserve the full result list from stdlib and native
calls.
Runtime native registries are now shared across cloned handles, which allows
runtime-created native callable values to remain visible to existing execution
contexts.
The core stop-the-world GC mark phase now traces transitive object references
through `GcObject` hooks, including table array/hash/metatable references,
thread stack values, registry roots, and closed-upvalue-style captured values.
Core tables now have explicit weak-key, weak-value, and combined weak modes.
The GC processes weak-key entries as ephemerons to a fixed point and prunes
dead weak table entries before sweeping unreachable objects.
Unreachable finalizable GC objects are queued before sweep, finalizer errors are
contained and counted, and userdata-kind lifecycle tests verify finalization
before drop plus reachable-object deferral.
The core GC exposes incremental mode and phase state, and mutation paths for
tables and thread stacks use write barriers that preserve tri-color invariants
by graying black containers when white children are stored.
The benchmark crate now provides a stable custom `cargo bench` harness with
microbenchmarks for arithmetic, table access, calls, and strings plus macro
workloads for accumulator, table-build/sum, and string-pattern paths.
The primitive interpreter now initializes register stacks with a single resize
and routes hot register reads/writes through checked-once unsafe stack helpers
with local safety comments.
Runtime table storage now maintains version-guarded inline caches for raw and
integer table reads, uses the same cache path for global reads, and invalidates
guards through table version changes including runtime metatable updates.
The bytecode and primitive interpreter now support an `ADD_INT`
superinstruction for register plus unsigned integer immediate addition,
including compiler emission, verifier/disassembler coverage, numeric execution,
and metamethod fallback.
The optional JIT path now has Cranelift crate dependencies, host/frontend
configuration scaffolding, a top-level `jit` feature, and a feature-gated
`JitMode::{Off, Hot, Always}` API placeholder on `LuaBuilder`.
The JIT crate now exposes a stable C-compatible baseline ABI with `JitFn`,
`JitStatus`, an opaque runtime context pointer, and a runtime helper registry
that can call registered helpers directly before generated-code integration.
The baseline JIT can lower a narrow single-return integer arithmetic Proto
subset to Cranelift, execute generated code through the JIT ABI, and compare
results against the interpreter for supported arithmetic paths.
The JIT runtime wrapper now tracks per-Proto hot counters, caches compiled
arithmetic JIT entries, transitions hot or always-JIT Protos into generated
code, and falls back to interpreter execution for unsupported JIT paths.
The JIT runtime wrapper also has an explicit debug-hook activity switch that
forces interpreter execution without hot-counter or compile activity while
debug hooks are active.
The API chunk path now routes environment-independent chunks through the JIT
runtime when `JitMode` is selected, threads runtime debug-hook activity into
that JIT runtime, and keeps runtime-environment/debug-library chunks on the
interpreter until environment-aware JIT fallback exists.
The JIT crate now has deoptimization metadata structures for live registers and
program counters, VM stack synchronization for live values, and a
deopt-to-interpreter fallback helper covered by focused tests.
The JIT crate now has table array fast-path helpers with table-tag checks,
integer-key and bounds checks, table version guards, raw get/set handling, and
explicit slow-path fallback reasons covered by focused tests.
The JIT crate now has a call trampoline helper ABI with injectable native and
Lua fallback handlers plus explicit returned, yielded, runtime-error, and
unsupported statuses covered by focused tests.
The JIT crate now has an interpreter equivalence suite covering compiled
arithmetic execution plus automatic fallback equivalence for table, Lua-call,
and yield/error paths.
The standard-library descriptor registry now covers the current-version
`io`, `os`, `package`, and `debug` library surfaces for full profiles while
leaving host-sensitive executable native registration gated off.
The `io` standard-library module now exposes executable `io.type` for
profile-selected runtimes, returning `nil` until runtime file handles are
implemented.
The `io` standard-library module now exposes executable `io.open` as a safe
pre-file-handle stub, validating filename and mode arguments and returning a
Lua-style `nil` plus unsupported-file-handle message.
The `io` standard-library module now also exposes executable `io.tmpfile`
and `io.popen` as safe pre-file-handle stubs, validating `io.popen`
command and mode arguments while returning Lua-style `nil` plus the
unsupported-file-handle message.
The `io` standard-library module now also exposes executable `io.close` and
`io.flush` as safe pre-file-handle stubs, returning Lua-style `nil` plus the
unsupported-file-handle message until runtime file handles exist.
The `io` standard-library module now also exposes executable `io.input` and
`io.output` as safe pre-file-handle stubs, validating optional filename
arguments while returning Lua-style `nil` plus the unsupported-file-handle
message until runtime file handles exist.
The `io` standard-library module now also exposes executable `io.read` and
`io.write` as safe pre-file-handle stubs, validating read format and writable
value argument types while returning Lua-style `nil` plus the
unsupported-file-handle message until runtime file handles exist.
The `io` standard-library module now also exposes executable `io.lines` as a
safe pre-file-handle stub, validating optional filename and read-format
arguments while returning Lua-style `nil` plus the unsupported-file-handle
message until runtime file handles exist.
The `os` standard-library module now exposes executable `difftime` for
profile-selected runtimes, including stdlib-native tests and public API
evaluation coverage.
The `os` standard-library module now also exposes executable `os.time`,
returning the current Unix time without arguments and converting date tables
through deterministic UTC normalization for profile-selected runtimes.
The `os` standard-library module now exposes executable `os.clock` for
profile-selected runtimes, backed by a monotonic elapsed-time clock.
The `os` standard-library module now exposes executable `os.getenv` for
profile-selected runtimes, returning `nil` for absent host variables and short
string values when the current native string support can represent them.
The `package` standard-library module now exposes executable
`package.searchpath` for profile-selected runtimes, including Lua-style module
separator replacement, path-template expansion, readable-file lookup, and
nil-plus-message misses.
The profile-selected `package` table now also registers the Lua-style
`package.config` platform configuration string.
The `debug` standard-library module now exposes executable raw
`debug.getmetatable` for profile-selected runtimes, returning table metatables
without honoring protected `__metatable` markers.
The `debug` standard-library module also exposes executable raw
`debug.setmetatable` for runtime-supported table values, bypassing base-library
protected-metatable checks.
The `debug` standard-library module now exposes executable
`debug.getregistry`, returning a stable mutable runtime registry table for the
current evaluation.
The `debug` standard-library module now also exposes executable
`debug.traceback` message handling and current-thread stack-frame
materialization, returning Lua-style traceback headers plus source/current-line
frame entries for interpreter frames while preserving non-string message
values.
The `debug` standard-library module exposes executable `debug.gethook`,
returning `nil` when no hook is installed.
The `debug` standard-library module now installs, clears, and returns current
runtime hook metadata through `debug.sethook` and `debug.gethook`, including
Lua-style mask normalization and count preservation.
The primitive interpreter now dispatches `debug.sethook` call, return, line,
and count hook callbacks for one-shot native and Lua execution paths, with
Lua-style event strings and line arguments for line events.
The `debug` standard-library module now registers executable `debug.getinfo`
through a runtime `DebugInfoTarget` hook, validating level/function targets and
optional option strings while returning `nil` until interpreter frame metadata
is supplied.
The primitive interpreter and API stdlib bridge now materialize initial
`debug.getinfo` tables for current-thread Lua and native function frames,
including source, current-line, parameter, tail-call, transfer, function, and
active-line fields for supported one-shot execution paths.
The `debug` standard-library module now exposes read-only `debug.getupvalue`
for Lua closures through runtime-captured upvalue metadata, returning
name/value pairs for existing upvalues and `nil` for absent or native upvalues.
The `debug` standard-library module now also exposes `debug.setupvalue` for
Lua closures through the same runtime-captured upvalue metadata, returning the
upvalue name on mutation and `nil` for absent or native upvalues.
Primitive runtime closures now store upvalues as shared cells, so sibling
closures that capture the same parent stack slot observe `debug.setupvalue`
mutations through the same runtime upvalue identity.
The `debug` standard-library module now exposes `debug.upvalueid`, returning a
light-userdata identity for existing Lua closure upvalues and `nil` for absent
or native upvalues.
The `debug` standard-library module now exposes `debug.upvaluejoin`, making one
Lua closure upvalue share another closure's runtime upvalue cell and rejecting
native functions or invalid upvalue indexes.
Bytecode debug metadata now records source-level local variable descriptors,
and the simple compiler emits descriptors for compiled local declarations as a
prerequisite for `debug.getlocal` and `debug.setlocal`.
The `debug` standard-library module now exposes read-only stack-level
`debug.getlocal` for current-thread Lua frames in one-shot interpreter
execution paths, returning materialized local names and register values where
local debug descriptors are active.
Function-target `debug.getlocal` now returns Lua closure parameter names when
the target Proto has parameter debug descriptors, and returns `nil` for absent
or native function parameter names.
The `debug` standard-library module now also exposes stack-level
`debug.setlocal` for current-thread Lua frames in one-shot interpreter
execution paths, mutating active local register values and returning the local
name when a supported descriptor is found.
Primitive bytecode coroutines now materialize their active Lua debug frame
stack for native calls during coroutine execution, so native debug queries can
inspect current coroutine Lua frames and their immediate Lua callers.
The `elara-stdlib` debug local and upvalue native tests now live in focused
sibling test modules so the main debug module remains under the workflow
source-size limit before more M18.2 debug work.
The `debug` standard-library module now exposes current-runtime
`debug.getuservalue` and `debug.setuservalue` behavior for the pre-userdata
runtime: non-userdata reads return `nil` and writes reject non-userdata values.
The `os` standard-library module now exposes executable `os.execute` for
profile-selected runtimes, reporting shell availability without a command and
returning Lua-style exit tuples for host shell commands.
The `os` standard-library module now exposes executable `os.exit` as a safe
unsupported process-termination native, validating the status and close
arguments without terminating the embedding host.
The `os` standard-library module now exposes executable `os.remove` for
profile-selected runtimes, returning Lua-style file-result tuples for host
filesystem success and failure.
The `os` standard-library module now also exposes executable `os.rename` for
profile-selected runtimes, returning Lua-style file-result tuples for host
filesystem success and failure.
The `os` standard-library module now exposes executable `os.tmpname` for
profile-selected runtimes, returning short unique relative filename candidates
without creating host files.
The profile-selected `package` table now registers distinct empty
`package.loaded` and `package.preload` state tables.
The `package` standard-library module now exposes executable `package.loadlib`
for profile-selected runtimes as a safe unsupported-C-loader stub, validating
string arguments and returning Lua-style `nil`, error message, and `"open"`
stage results.
The `os` standard-library module now exposes executable `os.setlocale` for a
deterministic C-locale subset, including Lua locale category validation and
`nil` results for unsupported locale names.
The `os.time` date-table form now reads required and optional date fields,
applies Lua's default noon hour, normalizes overflowed fields in UTC, writes
normalized fields back to the table, and returns a Unix timestamp.
The `os.date` standard-library module now exposes executable UTC table output
for the deterministic `"!*t"` format, including Lua-style calendar fields and
`isdst = false`.
The `os.date` implementation now also formats deterministic UTC strings for
common `strftime`-style specifiers when the format is prefixed with `!`.
The profile-selected `package` table now also registers a distinct
`package.searchers` state table. `require` honors user-populated searchers that
return loader functions and loader data before falling back to direct
`package.preload` lookup, leaving built-in path and C searcher functions for
future package-loading work.
The profile-selected `package.searchers` table now seeds the default preload
searcher at index 1, backed by a hidden native searcher and nested initial
table-entry seeding in the primitive runtime environment.
The profile-selected `package.searchers` table now also seeds the default Lua
path searcher at index 2. It is backed by `package.searchpath`, returns
Lua-style path-miss strings, and reports explicit unsupported Lua file loading
when a matching file is found until source file chunk loading is wired.
The profile-selected `package.searchers` table now also seeds the default C
and all-in-one C searchers at indexes 3 and 4. They use `package.cpath` through
`package.searchpath`, preserve Lua-style path-miss strings, and report explicit
unsupported dynamic-library loading when a matching library path exists.
The executable `package.require` path now aggregates string results returned by
package searchers into the final module-not-found error, so the default
preload, Lua path, and C path searchers all contribute Lua-style miss details.
The `package` standard-library module now exposes executable
`package.require` for preloaded modules, using `package.loaded` as the module
cache, calling loaders from `package.preload`, defaulting nil loader results to
`true`, and returning Lua-style preload loader data from the native path. The
same implementation is exposed as the Lua-style global `require` when the
package library is registered.
The profile-selected `package` table now registers platform-default
`package.path` and `package.cpath` strings, and `package.searchpath` reads and
returns general runtime strings instead of being limited to short strings.
The common byte-oriented `string` primitives `len`, `byte`, `char`, `lower`,
`upper`, `reverse`, `rep`, and `sub` now read and return general runtime
strings instead of being limited to short-string storage.
`table.concat` now also reads table elements and separators as general runtime
strings and can return long string results.
Base-library string-facing paths now also handle general runtime strings for
`rawlen`, `tonumber`, `tostring`, print/error messages, and protected-call
error strings.
The executable `utf8` primitives now read general runtime strings, and
`utf8.char` can return long string results.
`string.format` now also reads format strings and string/numeric conversion
arguments as general runtime strings and can return long string results.
`math.tointeger` now also parses numeric input from general runtime strings.
`table.sort` default string comparisons now also read general runtime strings.
The executable string pattern functions now also return captures/results and
read `gsub` replacement strings through general runtime strings.
Public native Rust callbacks now also receive and return general runtime
strings through the API bridge.
Executable `os` string arguments and string results now also use general
runtime strings for supported host-safe functions.

Current state:

```text
Completed:
  - Project name selected: Elara.
  - Direction selected: only support the latest stable Lua version on main.
  - Architecture direction selected: Rust-native VM, high-level API, optional Cranelift JIT.
  - Documentation drafts prepared.
  - M0.1 Create workspace skeleton.
  - M0.2 Add lint, formatting, and CI configuration.
  - M0.3 Define crate-level module policies.
  - M1.1 Add spec module.
  - M1.2 Add diagnostic and source span primitives.
  - M1.3 Add test fixture layout.
  - M1.4 Add snapshot testing baseline.
  - M2.1 Implement Value and primitive conversions.
  - M2.2 Add GC pointer and object header types.
  - M2.3 Implement basic arena/list allocator.
  - M2.4 Implement stop-the-world mark-sweep MVP.
  - M3.1 Implement Lua strings and interning.
  - M3.2 Implement table array storage.
  - M3.3 Implement table hash storage.
  - M3.4 Add metatable field and table versioning.
  - M4.1 Implement lexer for tokens and literals.
  - M4.2 Implement expression parser.
  - M4.3 Implement statement parser.
  - M4.4 Implement parser snapshots and error tests.
  - M5.1 Define Proto, Instr, and opcode encoding.
  - M5.2 Add bytecode builder and disassembler.
  - M5.3 Add bytecode verifier.
  - M5.4 Compile constants and arithmetic expressions.
  - M6.1 Add VM state and thread stack.
  - M6.2 Implement interpreter loop for constants and arithmetic.
  - M6.3 Connect source compile and eval path.
  - M7.1 Implement local variables and assignment.
  - M7.2 Implement function Protos and Lua calls.
  - M7.3 Implement closures and upvalues.
  - M7.4 Implement varargs and named vararg table.
  - M7 exit criteria validation.
  - M8.1 Implement conditional branches.
  - M8.2 Implement while and repeat loops.
  - M8.3 Implement numeric for loops.
  - M8.4 Implement generic for loops.
  - M8 exit criteria validation.
  - M9.1 Compile and execute table constructors.
  - M9.2 Implement table get/set bytecode.
  - M9.3 Implement metatable and metamethod dispatch.
  - M9.4 Implement global declaration semantics.
  - M10.1 Implement structured runtime errors.
  - M10.2 Implement protected calls.
  - M10.3 Implement coroutines and yield/resume.
  - M10.4 Implement to-be-closed variables.
  - M11.1 Add library registration framework.
  - M11.2 runtime native-call support for executable standard-library functions.
  - M11.2 executable math native specs for abs, ceil, floor, and sqrt.
  - M11.2 executable math acos, asin, and atan native specs.
  - M11.2 executable math sin, cos, and tan native specs.
  - M11.2 executable math deg and rad native specs.
  - M11.2 executable math exp and log native specs.
  - M11.2 executable math fmod and modf native specs.
  - M11.2 executable math frexp and ldexp native specs.
  - M11.2 executable math ult native spec.
  - M11.2 executable math.tointeger native spec.
  - M11.2 primitive runtime environment seeding for native globals.
  - M11.2 closure-backed runtime native registry entries.
  - M11.2 primitive runtime environment seeding for table-valued globals.
  - M11.2 API bridge from implemented stdlib native specs to RuntimeEnvironment.
  - M11.2 executable math min/max native specs.
  - M11.2 executable math.type native spec.
  - M11.2 executable math.random native spec.
  - M11.2 executable math.randomseed native spec.
  - M11.2 executable math constants pi, huge, maxinteger, and mininteger.
  - M11.2 executable base assert, rawequal, and numeric select native specs.
  - M11.2 executable base select count form.
  - M11.2 executable base error native spec.
  - M11.2 executable base getmetatable and setmetatable native specs.
  - M11.2 executable base next native spec.
  - M11.2 executable base print native spec.
  - M11.2 NativeContext support for protected native calls.
  - M11.2 executable base pcall native spec.
  - M11.2 executable base xpcall native spec.
  - M11.2 NativeContext support for runtime short strings.
  - M11.2 NativeRuntime abstraction for context-aware stdlib natives.
  - M11.2 NativeContext support for runtime table allocation.
  - M11.2 NativeContext support for raw runtime table length and reads.
  - M11.2 NativeContext support for raw runtime table writes.
  - M11.2 NativeContext support for raw runtime table value-key reads and writes.
  - M11.2 NativeContext support for runtime table metatable links.
  - M11.2 NativeContext support for raw runtime table traversal.
  - M11.2 executable base rawget, rawlen, and rawset native specs.
  - M11.2 executable base tonumber native spec.
  - M11.2 executable base tostring native spec.
  - M11.2 executable table.concat native spec.
  - M11.2 executable table.insert native spec.
  - M11.2 executable table.move native spec.
  - M11.2 executable table.pack native spec.
  - M11.2 executable table.remove native spec.
  - M11.2 executable default-comparator table.sort native spec.
  - M11.2 executable custom-comparator table.sort native spec.
  - M11.2 executable table.unpack native spec.
  - M11.2 executable base type native spec.
  - M11.2 executable string.byte native spec.
  - M11.2 executable string.char native spec.
  - M11.2 executable literal-search string.find native spec.
  - M11.2 executable basic `%s`, `%c`, `%q`, `%p`, `%f`, `%e`, `%E`, `%g`, `%G`, integer-family, and escaped-percent string.format native spec.
  - M11.2 split string.format tests into a focused sibling module.
  - M11.2 executable string.format `%s` width/precision modifiers.
  - M11.2 executable string.format integer-family width, left-adjust, and zero-padding modifiers.
  - M11.2 executable string.format signed decimal `+` and space flags.
  - M11.2 executable string.format octal/hex alternate-form flags.
  - M11.2 executable string.format integer precision modifiers.
  - M11.2 executable string.format float precision modifiers.
  - M11.2 executable string.format non-hex float width and flag modifiers.
  - M11.2 executable string.format basic hex-float `%a` and `%A` conversions.
  - M11.2 executable string.format hex-float width and flag modifiers.
  - M11.2 executable string.format hex-float precision modifiers.
  - M11.2 executable literal string-replacement string.gsub native spec.
  - M11.2 executable literal-search string.match native spec.
  - M11.2 executable `.` wildcard matching for string.find, string.match, and string.gsub.
  - M11.2 executable `^`/`$` anchor matching for string.find, string.match, and string.gsub.
  - M11.2 executable `%` character-class matching for string.find, string.match, and string.gsub.
  - M11.2 executable bracket-class pattern matching for string.find, string.match, and string.gsub.
  - M11.2 executable `?`, `*`, `+`, and `-` quantifier matching for string.find, string.match, and string.gsub.
  - M11.2 executable `%b` balanced-delimiter matching for string.find, string.match, and string.gsub.
  - M11.2 executable `%f` frontier matching for string.find, string.match, and string.gsub.
  - M11.2 executable string pattern capture back-references.
  - M11.2 executable string pattern position captures.
  - M11.2 executable capture-returning string.match native spec.
  - M11.2 executable capture-returning string.find native spec.
  - M11.2 executable string.gsub string replacement captures.
  - M11.2 executable string.gsub table and function replacements.
  - M11.2 executable capture-returning string.gmatch generic-for iterator path.
  - M11.2 executable string.len native spec.
  - M11.2 executable string lower, upper, and reverse native specs.
  - M11.2 executable string.rep native spec.
  - M11.2 executable string.sub native spec.
  - M11.2 executable base ipairs and raw pairs native specs.
  - M11.2 executable base pairs `__pairs` metamethod support.
  - M11.2 native iterator support for generic for loops.
  - M11.2 stdlib-backed string.gmatch generic-for iterator path.
  - M11.2 call lowering preserves local callable values across assignment calls.
  - M11.2 Lua-style callable string.gmatch iterator state.
  - M11.2 Lua-style string.gmatch leading-caret semantics.
  - M11.2 Implement base, table, math, and string essentials.
  - M11.2 exit criteria validation.
  - M11.3 Implement coroutine and utf8 libraries, with documented full-profile coroutine gaps.
  - M11.4 Add sandboxed profile tests.
  - M11 exit criteria validation.
  - M12.1 Add `LuaBuilder`, `Lua`, and `Chunk`.
  - M12.2 Add `IntoLua` and `FromLua`.
  - M12.3 Add native Rust functions.
  - M12.4 Add tables, registry keys, and userdata.
  - M12 exit criteria validation.
  - M13.1 Add official Lua runner integration.
  - M13.2 Add conformance test subsets.
  - M13.3 Add fuzz targets.
  - M13 exit criteria validation.
  - M14.1 Implement complete tracing for all object types.
  - M14.2 Add weak tables and ephemeron behavior.
  - M14.3 Add finalization and userdata lifecycle.
  - M14.4 Add incremental collection and write barriers.
  - M14 exit criteria validation.
  - M15.1 Add benchmark harness.
  - M15.2 Optimize VM dispatch and stack access.
  - M15.3 Add inline caches.
  - M15.4 Add selected superinstructions.
  - M15 exit criteria validation.
  - M16.1 Add JIT crate and feature flag.
  - M16.2 Define JIT ABI and runtime helper layer.
  - M16.3 Lower simple arithmetic Protos to Cranelift.
  - M16.4 Add JIT call integration and hot counters.
  - M16 exit criteria validation.
  - M17.1 Add guard and deopt metadata.
  - M17.2 Lower table array fast path.
  - M17.3 Lower calls through trampoline.
  - M17.4 Add JIT equivalence suite.
  - M17 exit criteria validation.
  - M18.1 standard-library descriptor surface for host-sensitive libraries.
  - M18.1 executable pre-file-handle `io.type`.
  - M18.1 safe unsupported pre-file-handle `io.open`.
  - M18.1 safe unsupported pre-file-handle `io.tmpfile` and `io.popen`.
  - M18.1 safe unsupported pre-file-handle `io.close` and `io.flush`.
  - M18.1 safe unsupported pre-file-handle `io.input` and `io.output`.
  - M18.1 safe unsupported pre-file-handle `io.read` and `io.write`.
  - M18.1 safe unsupported pre-file-handle `io.lines`.
  - M18.1 exit criteria validation.
  - M18.1 executable `os.difftime`.
  - M18.1 executable `os.time` without arguments and with UTC-normalized date tables.
  - M18.1 executable `os.clock`.
  - M18.1 executable `os.getenv`.
  - M18.1 executable `package.searchpath`.
  - M18.1 `package.config` table field registration.
  - M18.1 executable raw `debug.getmetatable`.
  - M18.1 executable raw `debug.setmetatable`.
  - M18.1 executable `os.remove`.
  - M18.1 executable `os.rename`.
  - M18.1 executable `os.tmpname`.
  - M18.1 `package.loaded` and `package.preload` table field registration.
  - M18.1 executable unsupported-C-loader `package.loadlib`.
  - M18.1 executable C-locale subset `os.setlocale`.
  - M18.1 executable `os.date` UTC table format.
  - M18.1 `package.searchers` table field registration.
  - M18.1 executable preloaded-module `package.require`.
  - M18.1 global `require` alias for the package loader.
  - M18.1 custom `package.searchers` support for `require`.
  - M18.1 default preload searcher in `package.searchers[1]`.
  - M18.1 default Lua path searcher in `package.searchers[2]`.
  - M18.1 default C path searchers in `package.searchers[3]` and `[4]`.
  - M18.1 `package.require` searcher miss aggregation.
  - M18.1 executable `debug.getregistry`.
  - M18.1 executable no-frame `debug.traceback` message handling.
  - M18.1 executable no-hook `debug.gethook`.
  - M18.2 executable `debug.getinfo` runtime hook and argument validation.
  - M18.2 initial `debug.getinfo` interpreter frame materialization.
  - M18.2 read-only `debug.getupvalue` for Lua closure upvalues.
  - M18.2 `debug.setupvalue` mutation for Lua closure upvalues.
  - M18.2 split debug upvalue stdlib-native tests into a focused sibling module.
  - M18.2 light userdata `Value` representation for debug identity results.
  - M18.2 shared primitive runtime upvalue cells for debug identity semantics.
  - M18.2 executable `debug.upvalueid` for Lua closure upvalue identities.
  - M18.2 executable `debug.upvaluejoin` for Lua closure upvalue sharing.
  - M18.2 bytecode/compiler local-variable debug descriptors for local access.
  - M18.2 read-only stack-level `debug.getlocal` for current-thread Lua frames.
  - M18.2 function-target `debug.getlocal` parameter-name lookup.
  - M18.2 stack-level `debug.setlocal` for current-thread Lua frames.
  - M18.2 primitive coroutine debug frames for native debug calls.
  - M18.2 JIT runtime debug-hook disable switch.
  - M18.2 API JIT selection plumbing for debug/runtime-environment chunks.
  - M18.2 `debug.sethook` installation and `debug.gethook` metadata retrieval.
  - M18.2 debug hook callback dispatch for call and return events.
  - M18.2 debug hook callback dispatch for line and count events.
  - M18.1 clear-only `debug.sethook`.
  - M18.1 pre-userdata `debug.getuservalue` and `debug.setuservalue`.
  - M18.1 safe unsupported process-termination `os.exit`.
  - M18.1 executable `os.execute`.
  - M18.1 native/runtime long string allocation support.
  - M18.1 `package.path` and `package.cpath` table field registration.
  - M18.1 executable `os.date` UTC string format subset.
  - M18.1 long-string support for common byte-oriented `string` primitives.
  - M18.1 long-string support for `table.concat`.
  - M18.1 long-string support for base-library string-facing paths.
  - M18.1 long-string support for executable `utf8` primitives.
  - M18.1 long-string support for `string.format`.
  - M18.1 long-string support for `math.tointeger`.
  - M18.1 long-string support for `table.sort` string comparisons.
  - M18.1 long-string support for string pattern function results and replacements.
  - M18.1 long-string support for public native Rust callback strings.
  - M18.1 long-string support for executable `os` string arguments and results.

In progress:
  - JIT.
  - C API.
```

## Current Milestone Details

### M0 Repository Bootstrap

Goal: create a structured Rust workspace for Elara with project documentation,
CI-ready defaults, and empty crates.

### Completed Step: M0.1 Create workspace skeleton

Delivered:

- Root `Cargo.toml` workspace.
- Root `README.md`.
- `docs/ARCHITECTURE.md`.
- `docs/MILESTONES.md`.
- `docs/CODEX_GOAL.md`.
- `docs/PROGRESS.md`.
- `crates/` directory.
- Placeholder crates:
  - `elara-core`
  - `elara-syntax`
  - `elara-compiler`
  - `elara-bytecode`
  - `elara-interp`
  - `elara-stdlib`
  - `elara-api`
  - `elara-jit`
  - `elara-capi`
  - `elara-test`
  - `elara-bench`

### Completed Step: M0.2 Add lint, formatting, and CI configuration

Delivered:

- Shared lint configuration in workspace `Cargo.toml`.
- Member crates inherit workspace lint settings.
- Root `.rustfmt.toml`.
- GitHub Actions CI for fmt, clippy, and tests.

### Completed Step: M0.3 Define crate-level module policies

Delivered:

- Each workspace crate has `lib.rs` module-level boundary docs.
- `elara` facade crate exists under `crates/elara`.
- `elara` re-exports only the stable public API placeholder module from `elara-api`.

M0 is complete.

### Completed Step: M1.1 Add spec module

Delivered:

- `elara-core` exposes `LuaVersion`, `LuaSpec`, `LUA_VERSION`, and `LUA_SPEC`.
- The current target is Lua 5.5 / Lua 5.5.0.
- No old-version `LuaDialect` enum or compatibility flag was added.

### Completed Step: M1.2 Add diagnostic and source span primitives

Delivered:

- `SourceId`, `Span`, and `Spanned<T>`.
- `DiagnosticSeverity`, `DiagnosticLabel`, and `Diagnostic`.
- Display formatting and focused tests for source spans and diagnostics.

### Completed Step: M1.3 Add test fixture layout

Delivered:

- `tests/fixtures/pass/`.
- `tests/fixtures/fail/`.
- `tests/conformance/`.
- `tests/differential/`.
- Placeholder harness docs for fixture, conformance, and differential areas.

### Completed Step: M1.4 Add snapshot testing baseline

Delivered:

- Snapshot helper for AST, bytecode, and diagnostics paths.
- Diagnostics snapshot formatting helper.
- First trivial fixture: `return 42`.
- Baseline diagnostics snapshot for `return 42`.

M1 is complete.

### Completed Step: M2.1 Implement Value and primitive conversions

Delivered:

- `Value` and `ValueTag`.
- Nil, boolean, integer, and float constructors and accessors.
- Primitive value equality, including integer/float numeric equality.
- Numeric helpers for float and exact integer conversion.

### Completed Step: M2.2 Add GC pointer and object header types

Delivered:

- `GcHeader`, `GcKind`, and `GcColor`.
- `GcObject` trait for GC-managed payloads.
- `GcRef<T>` typed internal reference wrapper.
- Unsafe pointer helpers are localized and documented with SAFETY comments.
- No public raw GC pointer accessor was added.

### Completed Step: M2.3 Implement basic arena/list allocator

Delivered:

- Runtime-owned `GcArena` allocation list.
- `GcStats` allocation/root counters.
- `GcRoot` placeholder root handles.
- Drop cleanup for arena-owned objects.

### Completed Step: M2.4 Implement stop-the-world mark-sweep MVP

Delivered:

- Stop-the-world collection entry point on `GcArena`.
- Mark phase over placeholder roots.
- Sweep phase over the allocation list.
- Tests for reachable and unreachable object collection.

M2 is complete.

### Completed Step: M3.1 Implement Lua strings and interning

Delivered:

- `ShortString` and `LongString` GC objects.
- Deterministic byte hashing for Lua strings.
- `StringInterner` for rooted interned short strings.
- String equality and hashing tests.

### Completed Step: M3.2 Implement table array storage

Delivered:

- `Table` GC object with array part storage.
- Raw 1-based integer array get/set helpers.
- Nil assignment clears slots and trims trailing nils.
- Array growth tests.

### Completed Step: M3.3 Implement table hash storage

Delivered:

- Hash part with canonical Lua `Value` keys.
- Numeric key canonicalization for integer-like floats.
- Nil and NaN key rejection.
- Tests for string, integer, float, and boolean keys.

### Completed Step: M3.4 Add metatable field and table versioning

Delivered:

- Optional metatable pointer.
- Meta flags placeholder.
- Version bump on structural mutations.
- Tests for version changes.

M3 is complete.

### Completed Step: M4.1 Implement lexer for tokens and literals

Delivered:

- Keywords, identifiers, punctuation, and operators.
- Integer, float, and string literal tokenization.
- Comments and whitespace handling.
- Error diagnostics for invalid tokens.

### Completed Step: M4.2 Implement expression parser

Delivered:

- Operator precedence.
- Unary and binary operators.
- Function call expressions.
- Table constructors.
- Vararg expression.

### Completed Step: M4.3 Implement statement parser

Delivered:

- Assignment.
- Local declarations.
- Global declarations for current Lua.
- Function declarations.
- If/while/repeat/for.
- Return/break/goto/labels where current Lua supports them.

### Completed Step: M4.4 Implement parser snapshots and error tests

Delivered:

- Snapshot tests for representative chunks.
- Error tests for malformed syntax.
- AST pretty debug output.

M4 is complete.

### Completed Step: M5.1 Define Proto, Instr, and opcode encoding

Delivered:

- `Proto`, `Instr`, and `Op`.
- Constant pool.
- Upvalue descriptors placeholder.
- Debug info placeholder.

### Completed Step: M5.2 Add bytecode builder and disassembler

Delivered:

- Builder API.
- Human-readable disassembly.
- Tests for simple instruction sequences.

### Completed Step: M5.3 Add bytecode verifier

Delivered:

- Register bounds checks.
- Constant bounds checks.
- Jump target checks.
- Basic call/return layout checks.

### Completed Step: M5.4 Compile constants and arithmetic expressions

Delivered:

- Compile literal expressions.
- Compile unary and binary arithmetic expressions.
- Emit constants.
- Verify generated bytecode.

M5 is complete.

### Completed Step: M6.1 Add VM state and thread stack

Delivered:

- `Vm` or `Runtime` state.
- `LuaThread` stack.
- Basic call frame.
- Stack push/pop helpers.

### Completed Step: M6.2 Implement interpreter loop for constants and arithmetic

Delivered:

- `LOAD_*`, `ADD`, `SUB`, `MUL`, `DIV`, `IDIV`, `RETURN`.
- Primitive runtime errors.
- Tests executing simple Protos.

### Completed Step: M6.3 Connect source compile and eval path

Delivered:

- Public internal function: source -> Proto -> interpreter.
- Test `return 42` from source.
- Test arithmetic source chunks.

M6 is complete.

### Completed Step: M7.1 Implement local variables and assignment

Delivered:

- Scope resolution.
- Register allocation for locals.
- Multiple assignment basics.
- Tests for local variable behavior.

### Completed Step: M7.2 Implement function Protos and Lua calls

Delivered:

- Function AST lowering.
- Nested Proto emission.
- `CALL` and `RETURN` basics.
- Tests for simple function calls.

### Completed Step: M7.3 Implement closures and upvalues

Delivered:

- Upvalue analysis.
- Open/closed upvalue runtime.
- Closure bytecode.
- Tests for nested functions.

### Completed Step: M7.4 Implement varargs and named vararg table

Delivered:

- Vararg function handling.
- `...` lowering.
- Named vararg table support for current Lua.
- Multiple return tests.
- Anonymous vararg functions can be compiled with `Proto::is_vararg`.
- `...` lowers to `VARARG` in anonymous vararg functions.
- Lua call arguments are placed in call registers and passed to child Protos.
- The primitive interpreter executes simple `VARARG` reads with nil fill.
- Fixed-count `CALL` results write all requested return registers with nil fill.
- Open-ended `VARARG`, `CALL`, and `RETURN` propagate all available results.
- Named vararg tables lower to `VARARG_TABLE`.
- The primitive interpreter materializes named vararg tables into runtime-owned table storage.

Recommended verification:

```bash
cargo test -p elara-interp varargs
```

Recommended commit:

```text
feat(runtime): support varargs
```

### Completed Step: M7 Exit Criteria Validation

Delivered:

- Function bodies can capture their own local function binding.
- Runtime closure storage is shared across nested executions so closure values
  remain callable across frames.
- Recursive self-reference works through the source compile/eval path.
- Recursive self-reference exit criteria are validated through source eval.

M7 is complete.

### Completed Step: M8.1 Implement conditional branches

Delivered:

- `TEST` and `JMP` execute in the primitive interpreter.
- The bytecode builder can patch forward `JMP` offsets.
- The verifier accepts jumps to the end-of-code boundary.
- The simple compiler lowers `if`, `elseif`, and `else` blocks.
- Source eval executes simple `if/else` chunks.

### Completed Step: M8.2 Implement while and repeat loops

Delivered:

- The simple compiler lowers `while` loops with false-condition exits and back
  jumps.
- The simple compiler lowers `repeat` loops with post-body condition checks.
- `break` statements inside loops patch to the loop exit.
- `break` outside a loop reports a compile diagnostic.
- Source eval executes simple `while`/`break` and `repeat` chunks.

### Completed Step: M8.3 Implement numeric for loops

Delivered:

- The simple compiler lowers numeric `for` loops to `FOR_PREP`/`FOR_LOOP`.
- The bytecode verifier checks the three-register numeric loop control range.
- The primitive interpreter executes integer numeric loops with positive and
  negative steps.
- The primitive interpreter executes float numeric loops.
- Zero numeric-for steps report a runtime error.
- Source eval executes simple numeric `for` chunks with positive and negative
  steps.

### Completed Step: M8.4 Implement generic for loops

Delivered:

- The simple compiler lowers generic `for` loops to `TFOR_PREP`, `TFOR_CALL`,
  and `TFOR_LOOP`.
- The bytecode verifier checks generic-for state and result register ranges.
- The primitive interpreter executes iterator calls with state/control values.
- Generic loops enter the body only when the first iterator result is non-nil.
- Source eval executes simple generic `for` chunks using iterator functions.

M8 is complete.

### Completed Step: M9.1 Compile and execute table constructors

Delivered:

- Bytecode prototypes carry string constants for record-field keys.
- The simple compiler lowers table constructors to `NEW_TABLE`, `SET_TABLE`,
  and `SET_LIST` for final open array fields.
- Array fields use consecutive integer keys starting at 1.
- Named fields use interned short-string keys.
- Keyed fields compile and write their explicit key expressions.
- The primitive interpreter executes `NEW_TABLE`, `LOAD_STRING`, and
  constructor `SET_TABLE` writes into runtime-owned table storage.
- Source eval can return table constructor values.

## Completed Content

### Planning Decisions

- Project name: Elara.
- Main branch target: current stable Lua only.
- Old Lua versions: not implemented through compatibility flags.
- Historical support policy: old versions should be preserved through tags or maintenance branches, not mainline dialect branching.
- Execution model: source -> AST -> HIR -> internal bytecode -> interpreter/JIT.
- JIT backend: Cranelift, optional feature, method JIT first.
- API direction: Rust-first embedding API with typed conversions, native functions, tables, registry, and userdata.
- GC direction: custom tracing GC, starting with stop-the-world mark-sweep, evolving to incremental collection.

### Workspace Bootstrap

- The repository root is a virtual Cargo workspace.
- Workspace uses placeholder member crates for the architecture-defined layers.
- `crates/elara` is the public facade crate and depends only on `elara-api`.
- Each placeholder crate has a minimal manifest and `src/lib.rs`.
- Root README describes project positioning and workspace layout.
- Workspace quality gates are configured through Cargo lints, rustfmt, and GitHub Actions.
- Crate-level module docs describe boundary policies.
- The current Lua target is declared once in `elara-core`.
- Core source span and diagnostic primitives are available.
- Test fixture, conformance, and differential directories are present.
- Snapshot helpers and a `return 42` fixture baseline are present.
- Primitive Lua values are available in `elara-core`.
- GC object headers and typed references are available in `elara-core`.
- Basic GC allocation list, stats, roots, and drop cleanup are available.
- Stop-the-world mark-sweep MVP is available under tests.
- Lua short strings, long strings, and short-string interning are available.
- Table array storage is available in `elara-core`.
- Table hash storage and numeric key canonicalization are available.
- Table metatable metadata, meta flags, and structural versioning are available.
- Lua tokenization with spans and lexical diagnostics is available in `elara-syntax`.
- Lua expression AST and precedence parsing are available in `elara-syntax`.
- Lua statement AST and block parsing are available in `elara-syntax`.
- Parser snapshots and malformed syntax diagnostics are covered.
- Bytecode opcode, instruction encoding, prototype, constant pool, upvalue descriptor, debug placeholder, builder, disassembler, and verifier are available.
- Simple return-expression bytecode codegen is available in `elara-compiler`.
- VM state, Lua thread stack, call frames, and stack helpers are available in `elara-core`.
- Primitive bytecode execution for constants, arithmetic, and return values is available in `elara-interp`.
- Simple source chunks can be compiled and evaluated through `elara-api`.
- Local variable reads/writes and assignment basics are available in the compiler/interpreter path.
- Simple nested Protos and fixed-parameter Lua calls are available.
- Captured outer local reads work through upvalue descriptors and runtime closure captures.
- Anonymous vararg argument passing and first-value `...` lowering work in the compiler/interpreter path.
- Fixed-count multiple call results work in the primitive interpreter.
- Open-ended vararg call/return results work in the primitive interpreter.
- Named vararg tables compile and execute through runtime-owned table storage.
- Recursive self-reference works through the compiler/interpreter/API path.
- Conditional branches work through the compiler/interpreter/API path.
- `while`, `repeat`, and `break` work through the compiler/interpreter/API path.
- Numeric `for` loops work through the compiler/interpreter/API path.
- Generic `for` loops work through the compiler/interpreter/API path.
- Table constructors work through the compiler/interpreter/API path.
- Raw table access works through the parser/compiler/interpreter/API path.
- Primitive runtime table storage has a centralized owner with metatable links.
- Table-valued `__index` and `__newindex` chains work in runtime table helpers.
- Function-valued `__index` and `__newindex` Lua closure calls work in runtime
  table helpers.
- Arithmetic metamethod Lua closure calls work in primitive arithmetic helpers.
- Comparison opcodes and table metamethod closure fallback work in the
  primitive interpreter.
- `LEN` works for runtime strings and runtime tables with `__len` closure
  fallback.
- Long string literals compile and execute through the simple compiler/API path.
- `CALL` works for runtime tables with `__call` closure fallback.
- `CONCAT` works for runtime strings and numeric operands with `__concat`
  closure fallback.
- Declared and implicit global reads/writes compile and execute through runtime
  `_ENV` bytecode.
- Global declaration initialization raises a runtime error when the global is
  already non-nil.
- `global<const> *` blocks assignment to implicit read-only globals in the
  simple compiler.
- Global declarations in nested `if` and loop blocks do not leak out of the
  block in the simple compiler.
- Inner explicit global declarations can shadow an outer collective read-only
  global declaration in the simple compiler.
- Local and captured `_ENV` tables are honored for global reads, global writes,
  and global declaration initialization checks.
- `DECL_GLOBAL` now checks the already-loaded candidate value, matching the Lua
  check-before-store lowering shape.
- `global function` declarations compile and execute through the global
  declaration/store path, including already-defined runtime checks.
- Named and collective read-only global declarations reject direct assignment in
  the simple compiler.
- Explicitly global `_ENV` declarations report a diagnostic when another global
  access would need `_ENV`.
- Default chunk `_ENV` is represented as a root upvalue in the simple compiler.
- Nested simple-compiler functions can capture `_ENV` through parent upvalues.
- Primitive execution seeds a runtime-owned global table and passes it as the
  root `_ENV` upvalue.
- Direct `GET_ENV`/`SET_ENV` bytecode and exposed `_ENV` table access share the
  same runtime global table.
- Core runtime exposes structured `LuaError<K>` and `TraceFrame`.
- Primitive interpreter runtime errors carry `RuntimeErrorKind`, message text,
  and traceback metadata.
- Errors propagating out of primitive Lua closure calls attach child prototype
  traceback frames.
- Core call-frame flags can mark protected-call boundaries.
- Primitive execution exposes `execute_proto_protected`, which returns normal
  output or a caught structured runtime error without propagating it.
- Core thread status helpers model coroutine running, suspended, and dead
  transitions.
- Bytecode includes a `YIELD` opcode with verifier range checks.
- Primitive execution exposes `PrimitiveCoroutine` and `CoroutineResume`.
- Primitive coroutines can yield from the root body or a called Lua frame,
  resume with values, return normally, and report dead-coroutine resume errors.
- One-shot primitive execution rejects `YIELD` outside a coroutine boundary.
- The simple compiler lowers local `<close>` declarations to `TBC` and emits
  `CLOSE` before explicit returns and implicit function end when a close local
  is pending.
- Primitive runtime `TBC` validates non-nil/non-false values for `__close` and
  `CLOSE` calls close metamethods in reverse registration order for normal
  close paths.
- Runtime error paths close pending to-be-closed values before preserving the
  original error when close succeeds.
- Primitive coroutine yield keeps pending to-be-closed values alive, and
  coroutine completion closes them.
- Runtime native registries are shared across cloned handles, and
  `NativeContext` can register callable native functions during execution for
  closure-like stdlib helpers.
- `elara-stdlib` exposes `Library`, `StdLib`, `StdLibSet`,
  `StdLibProfile`, `StdLibRegistry`, `GlobalRegistry`, and `GlobalLibrary`.
- Standard-library profiles expand to deterministic library sets and can
  register selected implementations into a generic global target.
- `elara-stdlib` defines `FunctionSpec`, `FunctionRegistry`,
  `FunctionLibrary`, and descriptor lists for essential base, table, math, and
  string functions.
- `essential_registry` registers the descriptor libraries selected by the
  current standard-library profile.
- `utf8.len` is implemented with strict/lax UTF-8 validation, relative byte
  ranges, invalid-byte reporting, descriptor registration, and runtime module
  registration.
- `utf8.char` encodes zero or more Lua integer code points through Lua's
  31-bit UTF-8 range and returns the concatenated string.
- `utf8.codepoint` returns code points across relative byte ranges, supports
  lax decoding, and reports invalid UTF-8 sequences as Lua errors.
- `utf8.offset` returns Lua 5.5 start/end byte ranges for forward, backward,
  zero-offset, and right-after-end character lookups.
- `utf8.codes` returns strict/lax iterator triplets backed by hidden native
  helpers and works through generic-for execution.
- `utf8.charpattern` is exposed as a runtime-interned string field, backed by
  generic initial global table string-field support in the primitive runtime
  environment.
- `Value` can represent runtime thread placeholders and `coroutine.status`
  reports runnable/suspended/running/dead coroutine status names through a
  stdlib runtime hook.
- `coroutine.create` registers runtime coroutine handles through shared API-side
  coroutine state and returns Lua thread values that `coroutine.status` can
  inspect.
- `coroutine.resume` invokes registered non-yielding Lua functions behind a
  protected-call boundary, prepends the Lua success flag, returns `false` plus
  an error message for protected failures, and marks completed handles dead.
- `coroutine.close` closes registry-backed runnable, suspended, and dead
  coroutine handles, marks closed handles dead, and rejects closing the main
  thread through the Lua error path.
- `coroutine.yield` is registered and delegates through a runtime hook; the
  current API hook reports Lua's outside-coroutine yield error until real
  stdlib coroutine suspension is wired to primitive coroutine execution.
- `coroutine.wrap` returns a dynamically registered callable native wrapper
  around a new registry-backed coroutine handle, resumes non-yielding wrapped
  functions, returns successful values directly, and propagates resume errors.
- `coroutine.running` returns the current main thread handle and main-thread
  flag through the shared coroutine registry.
- `coroutine.isyieldable` reports false for the main thread and true for live
  created coroutine handles.
- Sandboxed profiles exclude `io`, `os`, `package`, and `debug` while keeping
  base, coroutine, table, string, UTF-8, and math libraries selectable, and
  sandbox profile tests verify both disabled-library filtering and allowed
  function registration.
- The public API now exposes `LuaBuilder`, `Lua`, and `Chunk` handles. Builders
  select a standard-library profile, `Lua` allocates source IDs and loads
  source text, and chunks evaluate through the existing stdlib-backed simple
  compiler/interpreter path. The top-level `elara` facade re-exports these safe
  handles.
- The public API exposes owned `LuaValue` conversion helpers plus `IntoLua`,
  `FromLua`, `IntoLuaMulti`, and `FromLuaMulti` traits for nil/unit, booleans,
  integers, floats, UTF-8 strings, options, and one-/two-value tuple basics.
- The public API can create typed native Rust `Function` handles, register them
  as globals for future chunk evaluations, extract typed arguments through
  conversion traits, return multiple Lua values through `IntoLuaMulti`, and
  convert callback/conversion failures into runtime errors.
- The public API exposes safe high-level `Table`, `Function`, `RegistryKey`,
  and `AnyUserData` handles. Tables store and convert API-owned values, Lua
  registry keys round-trip stored values through a `Lua` handle, and userdata
  handles provide typed borrow checks without exposing runtime internals.
- The optional C API crate now packages current-version `lua.h`, `lauxlib.h`,
  and `lualib.h` scaffolding, exposes its include directory from the build
  script, and verifies that the headers target Lua 5.5 only.
- The optional C API crate now has a stack-backed `lua_State` core with
  current-version stack top manipulation, push/copy/rotate behavior, primitive
  type inspection, string/number/integer/boolean conversions, light userdata,
  C-function stack values, and neutral null-state/invalid-index handling for
  the stack API surface.
- The optional C API crate can invoke stack-registered C functions through
  `lua_pcallk`, including active C call-frame stack indexing, fixed-result and
  multiret normalization, runtime-error reporting for invalid calls, and panic
  containment for Rust `extern "C-unwind"` callbacks.
- The optional C API crate now has an integration test that compiles a small C
  module against the packaged current-version headers, exercising stack
  registration and protected-call macros at source level while keeping binary
  compatibility explicitly out of scope.
- `elara-test` exposes configurable official-Lua runner helpers using
  `ELARA_LUA`, captures stdout/stderr and success/error classes, and can compare
  official Lua runs against Elara's public API evaluation path for differential
  testing.
- Conformance fixture subsets now cover language, standard-library, error, and
  coroutine smoke cases through an `elara-test` integration harness that checks
  public API success/error classes.
- Reusable fuzz target entry points now exercise arbitrary bytes through the
  lexer/parser/simple compiler path, bytecode verifier, and raw table
  operations, with deterministic unit tests and workspace verification.
- The benchmark harness now emits release comparison rows for the public API
  interpreter path, the public API JIT path, and official Lua 5.5 when
  available, plus an API overhead workload; `docs/PERFORMANCE.md` records the
  M20 local release report and methodology caveats.
- The M20.3 safety and public API audit is recorded in
  `docs/SAFETY_API_AUDIT.md`; workspace clippy now denies missing unsafe
  function safety docs and undocumented unsafe blocks, the C API unsafe
  entrypoints have explicit `# Safety` sections, the facade docs include a
  native-function quick start doctest, and `crates/elara/examples/basic_embed.rs`
  compile-checks direct evaluation plus typed native callback registration.
- The M20.4 release-candidate pass expanded the README with usage, verification,
  workspace, feature, and limitation details; added
  `crates/elara/examples/jit_embed.rs` for feature-gated JIT embedding;
  documented the version matrix and release/tag plan in `docs/RELEASE.md`; and
  confirmed the workspace version constants still target Elara `0.1.0` and Lua
  5.5.0.
- Bitwise opcode execution now covers raw integer `&`, `|`, `~`, `<<`, `>>`,
  unary `~`, Lua-style wide-shift-to-zero behavior, and `__band`, `__bor`,
  `__bxor`, `__shl`, `__shr`, and `__bnot` metamethod dispatch through the
  interpreter arithmetic path. Compiler snapshots and public API evaluation
  tests cover source-level bitwise expressions.
- Conformance fixtures now cover an expanded release smoke matrix across
  language control flow, bitwise operators, varargs, standard-library
  table/string/utf8/os/package/debug cases, coroutine resume/wrap cases, and
  runtime-error classes. The conformance harness verifies exact portable
  primitive result vectors for success fixtures, and an
  optional differential fixture test compares the same portable fixture set
  against official Lua through `ELARA_LUA`, including stderr-aware error
  classification for stdin-based Lua runs.
- The conformance standard-library smoke matrix now also includes exact-value
  fixtures for base/table helpers, base assertion and protected assertion helpers, base metatable
  and protected metatable mutation helpers, base raw equality helpers, base raw traversal, after-key traversal, and empty traversal helpers, base raw length, metatable-bypass raw length, and empty raw length helpers, base raw table
  access, metatable-bypass access, metatable-bypass setting, and nil-clearing helpers, base iteration including `ipairs` nil-stop behavior, and argument-error helpers,
  base conversion/radix/numeric-input/standard-number/select/type helpers, base multi-result, negative-index, empty-count, and payload-count `select` helpers, base
  scalar and string-preserving `tostring` helpers,
  broader deterministic math functions, integer/float absolute value checks,
  math integer conversion checks,
  integer-preserving rounding, logarithm identity and base-specific behavior,
  zero square-root boundary, zero-angle conversion, deterministic math
  trig/angle helpers, two-argument arctangent,
  math decomposition helpers, negative-exponent recomposition,
  integer/fractional split paths, and zero-decomposition handling,
  math nil-result classification,
  deterministic math RNG edge behavior, common string transform/slicing helpers
  and expanded string argument-error coverage,
  math constants, math subtype and integer-conversion nil-results,
  random result subtypes,
  empty and byte-level string case conversion, byte-string construction range
  errors,
  empty and separator repeat, empty and embedded-NUL reverse, default-end and negative-end substring, and substring-range handling, string byte
  construction/extraction, embedded-NUL length, construction, and byte ranges, empty byte-string construction, default-end and ranged byte helpers,
  and out-of-range byte nil results,
  math functions combined with string pattern
  operations, string pattern find-position/capture/backreference/position-capture helpers, wildcard, bracket-class/negated bracket-class, greedy/optional quantifier, plain, escaped-literal, start/end-anchored, balanced, and frontier string-search helpers, string find miss results,
  literal, wildcard, percent-class, escaped-literal, bracket-class, negated
  bracket-class, quantifier, optional-quantifier, and minimal-quantifier match,
  start-anchored match-init, end-anchored match,
  capture-returning match, balanced-delimiter match, frontier match,
  backreference match, match-init, match-miss helpers, literal, wildcard,
  start-anchored, percent-class, bracket-class, quantifier, capture-replacement,
  balanced-delimiter, frontier, backreference, numeric-replacement, and
  position-capture substitution,
  position-capture and capture-returning iterator helpers, bounded substitution, missing-substitution,
  table-replacement, and function-replacement helpers,
  position-capturing iterator helpers, capture/replacement helpers,
  balanced/frontier string pattern helpers, table mutation, append, first-position, and positioned insertion,
  default, first-position, positioned, after-end, and empty removal, forward/backward
  overlapping, empty, and explicit range/destination moves,
  nil-preserving, trailing-nil, and empty table packing, empty, separator-default,
  nil-separator, numeric-value, numeric-separator, explicit-bound, explicit, and long-string table concatenation,
  table unpack range, default-bound, nil-preserving, non-positive bound, and
  empty-range helpers, and default numeric/string plus
  single-element, long-string, and comparator sorting helpers,
  UTF-8 character construction, empty construction, maximum codepoint
  construction, and upper-bound character errors, iterator/offset helpers,
  multibyte UTF-8 length/codepoint/offset helpers, ranged and multibyte
  ranged UTF-8 codepoint results, empty UTF-8 codepoint
  ranges, missing UTF-8 offset results, multibyte backward and containing
  UTF-8 offsets, long-string UTF-8 operations, empty, bounded, relative,
  invalid-sequence, lax UTF-8 length checks, lax UTF-8 codepoint results, and
  optional UTF-8 index argument errors,
  math subtype classification and expanded numeric function error coverage,
  integral float-to-integer conversion, mixed numeric min/max selection,
  integer/float remainder handling, and standalone unsigned comparison true/false cases,
  base protected-call multiple returns and caught errors, base extended protected-call error
  handling, pre-file-handle `io` stubs, absent-file `io.open` result
  classification, pre-file-handle `io.tmpfile` result classification,
  pre-file-handle `io.write`/`io.flush` result classification,
  pre-file-handle `io.type` nil classification, package configuration
  strings, default package path/cpath state, package preload/require
  caching, nil-loader defaults, custom searcher loader-data propagation, plus
  direct preload searcher hits and Lua searcher misses, direct
  `package.searchpath` miss result classification,
  direct `package.searchpath` readable-file hits and separator replacement in
  miss diagnostics,
  C-searcher miss results, unsupported `package.loadlib` result
  classification, debug frame introspection,
  debug local inspection/mutation, debug uservalue nil classification, debug
  traceback string results and non-string message preservation, raw debug
  metatable access/mutation, mutable
  debug registry access, debug upvalue inspection/mutation/join behavior, no-hook
  `debug.gethook`/`debug.sethook` behavior, deterministic `string.format`
  string width/precision, integer modifiers, precision, signed flags, and
  alternate forms, character and pointer width/left-adjust modifiers, float,
  quoted string/scalar, escaped-percent, and pointer output plus long-string format arguments/results, plus `os.execute` shell availability and success/failure command
  status tuples, `os.clock` result classification,
  `os.tmpname` byte-level and string-result checks, absent `os.getenv` handling,
  absent-file `os.remove`/`os.rename` result classification, UTC
  `os.date` table/string formatting, deterministic table metadata,
  weekday/month name aliases, and ordinal/escaped-percent specifiers,
  UTC `os.time` date-table normalization and default fields, and C-locale
  `os.setlocale` queries plus supported categories, and exact UTC composite
  `os.date` format output.
- The conformance language smoke matrix now also includes exact-value fixtures
  for table field construction/access, zero-argument closure capture, Lua 5.5
  computed table keys,
  conditional branch result selection,
  negative-step numeric `for` loops, custom generic-for iterator results,
  global declarations/functions, arithmetic metamethod dispatch, comparison
  expression lowering, unary `not` truthiness, logical `and`/`or`
  short-circuit values, length-operator string/table/metamethod results, long
  string literal and concatenation byte lengths, numeric concatenation
  coercion, and fixed-parameter function calls with missing-argument nil fill,
  fixed
  parameters combined with named vararg tables, and multiple-return local
  assignment and reassignment.
- The conformance error smoke matrix now includes parser diagnostics, explicit
  base-library errors, bad standard-library arguments, invalid indexing,
  non-callable values, arithmetic type errors, debug uservalue type errors, and
  unsupported `string.format` conversion errors including current-Lua-invalid
  uppercase `%F`, invalid `string.gsub` replacement escapes, and scoped unsupported
  `os.exit` behavior, plus `coroutine.yield` outside a resume boundary,
  attempted main-coroutine close, and invalid `debug.getinfo` option strings.
- The conformance coroutine smoke matrix now also includes wrap, resume success
  and returned values, runnable coroutine close, current-thread
  `coroutine.running`, main-thread and created-coroutine `isyieldable`, and
  created-coroutine lifecycle status coverage.
- The coroutine conformance and differential smoke matrix now covers
  create/wrap/resume/status/close/isyieldable argument error result shapes.
- The base-library conformance and differential smoke matrix now covers `_G`
  identity and the current-version `_VERSION` string.
- The base-library conformance smoke matrix now covers the registered
  unsupported dynamic-loading stubs, unsupported `collectgarbage`, and
  validating `warn`.
- The base-library conformance and differential smoke matrix now covers
  portable `load`/`loadfile`/`dofile` error result shapes without depending on
  host-specific diagnostic text or `load` reader/type ambiguity.
- The base-library conformance and differential smoke matrix now covers
  portable raw access, `select`, `setmetatable`, `tonumber`, and `warn`
  argument error result shapes.
- The base-library conformance and differential smoke matrix now covers
  additional traversal, introspection, conversion, and raw-helper argument
  error result shapes.
- The debug-library conformance and differential smoke matrix now covers
  `debug.sethook`/`debug.gethook` metadata round-tripping without depending on
  callback execution.
- The debug-library conformance and differential smoke matrix now also covers
  `debug.getinfo` empty-option results and invalid-option `pcall` shape.
- The debug-library conformance and differential smoke matrix now covers
  absent and native-function upvalue result shapes plus bad `debug.setupvalue`
  argument `pcall` results.
- The debug-library conformance and differential smoke matrix now covers
  missing-local and bad-argument `debug.setlocal` result shapes.
- The debug-library conformance and differential smoke matrix now covers
  non-userdata `debug.setuservalue` error result shape.
- The debug-library conformance and differential smoke matrix now covers
  absent/native `debug.upvalueid` results plus invalid `debug.upvaluejoin`
  `pcall` shapes.
- The debug-library conformance and differential smoke matrix now covers
  portable `getinfo`, local/upvalue helper, hook, and traceback argument error
  result shapes.
- `debug.getinfo` now reports main chunk stack frames as vararg, matching Lua
  5.5 debug metadata while leaving ordinary Lua function and native function
  `isvararg` reporting unchanged.
- The `debug.getinfo` name/transfer fixture now avoids context-sensitive
  `name`/`namewhat` call-site metadata while still exact-checking portable
  transfer fields and omitted field results.
- The math-library conformance and differential smoke matrix now covers
  `math.fmod`, `math.random`, and `math.ult` argument error result shapes.
- `math.randomseed` now follows Lua 5.5 by validating only the first two seed
  arguments and ignoring extras; the conformance/differential fixture covers
  first/second type errors plus extra-argument success shape.
- The math-library conformance and differential smoke matrix now covers
  generic number argument plus `math.min`/`math.max` error result shapes.
- The math-library conformance and differential smoke matrix now covers
  common numeric function and `math.ldexp` exponent error result shapes.
- The math-library conformance and differential smoke matrix now covers
  optional nil argument defaults for `math.atan` and `math.log`.
- The math-library conformance and differential smoke matrix now covers
  `math.log` with a non-special custom base.
- The math-library conformance and differential smoke matrix now covers
  inverse-trig, angle-conversion, decomposition, and `math.modf` numeric
  function error result shapes.
- Math numeric and integer argument helpers now coerce numeric strings for
  `math.abs`, `math.sqrt`, `math.fmod`, `math.ldexp`, `math.random`,
  `math.randomseed`, and `math.ult` paths, preserving Lua's float subtype for
  string-to-number arguments and rejecting non-integral strings for integer-only
  arguments.
- The table-library conformance and differential smoke matrix now covers
  `table.sort` non-function comparator and incomparable-value error result
  shapes.
- `table.remove` now follows Lua 5.5 by validating only the optional position
  argument and ignoring extras; the table insert/remove fixture covers insert
  wrong-arity plus remove extra-argument success shape.
- The table-library conformance and differential smoke matrix now covers
  `table.insert` and `table.remove` table/position type-error result shapes.
- The table-library conformance and differential smoke matrix now covers
  `table.remove` after-end boundary result preservation.
- The table-library conformance and differential smoke matrix now covers
  `table.concat` non-string value, separator, and bound type-error result
  shapes.
- The table-library conformance and differential smoke matrix now covers
  `table.concat` nil-separator explicit-bound result preservation.
- The table-library conformance and differential smoke matrix now covers
  `table.concat` and `table.sort` first-operand type-error result shapes.
- The table-library conformance and differential smoke matrix now covers
  `table.move` bound argument and destination type-error result shapes.
- The table-library conformance and differential smoke matrix now covers
  backward-overlapping `table.move` result preservation.
- The table-library conformance and differential smoke matrix now covers
  `table.unpack` table, start-bound, and end-bound type-error result shapes.
- The table-library conformance and differential smoke matrix now covers
  explicit non-positive bound `table.unpack` result preservation.
- The utf8-library conformance and differential smoke matrix now covers
  `utf8.char`, `utf8.codepoint`, `utf8.len`, and `utf8.offset` argument error
  result shapes.
- The utf8-library conformance and differential smoke matrix now covers
  `utf8.char` upper-bound and later-argument range error result shapes.
- The utf8-library conformance and differential smoke matrix now covers
  optional-index and subject type-error result shapes for `utf8.codepoint`,
  `utf8.len`, and `utf8.offset`.
- The utf8-library conformance and differential smoke matrix now covers
  `utf8.codepoint`, `utf8.len`, and `utf8.offset` bounds error result shapes.
- The utf8-library conformance and differential smoke matrix now covers
  `utf8.codes` subject type and invalid-leading-byte error result shapes.
- The utf8-library conformance and differential smoke matrix now covers
  invalid UTF-8 `utf8.codepoint` and continuation-byte `utf8.offset` error
  result shapes.
- Named vararg tables now populate Lua 5.5's `n` count field, and the existing
  anonymous/named vararg conformance fixture asserts the count alongside the
  captured argument values.
- The base-library conformance and differential smoke matrix now includes
  `pairs` dispatch through a `__pairs` metamethod that returns a four-value
  iterator tuple.
- The primitive interpreter now executes `SET_UPVALUE`, so closures can assign
  captured locals and sibling closures observe the shared updated cell.
- The simple compiler now accepts bare function-call statements and emits a
  fixed-result call into an ignored temporary register.
- Open upvalue cells are synced back to still-active parent stack slots after
  nested calls, so direct parent reads observe closure mutations.
- The simple compiler now lowers equality, inequality, less-than, less-or-equal,
  greater-than, and greater-or-equal expressions through the primitive
  comparison opcodes.
- The bytecode, primitive interpreter, and simple compiler now support unary
  `not` expressions with Lua truthiness.
- The simple compiler now emits unary operator results into fresh registers so
  locals reused later in a return list are not clobbered.
- The simple compiler now emits binary arithmetic, comparison, and concatenation
  expression results into fresh registers so operand locals are preserved.
- The simple compiler now lowers logical `and` and `or` expressions with Lua
  value-preserving short-circuit behavior.
- The primitive interpreter now executes `LEN` for runtime string byte lengths,
  and the language smoke matrix covers `#` for strings, tables, and `__len`.
- The simple compiler now accepts string literals larger than the short-string
  threshold, and `LOAD_STRING` materializes them through runtime long strings.
- Raw `CONCAT` now reads general runtime string operands and returns long
  string results when the joined bytes exceed the short-string threshold.
- Raw `CONCAT` now also coerces integer and float operands through the runtime
  numeric string representation before falling back to `__concat`.
- The conformance smoke matrix now covers the remaining safe unsupported
  pre-file-handle `io` stubs for input/output/lines/popen/read result shapes.
- The `io` conformance and differential smoke matrix now covers argument error
  result shapes for input/output/lines/open/popen/read/write validation paths.
- The `io` conformance and differential smoke matrix now covers non-file
  `io.type` nil classifications for booleans, tables, functions, and strings.
- The conformance and differential error smoke matrix now includes an
  unsupported string-pattern capture reference case.
- The package conformance and differential smoke matrix now includes
  `package.searchpath` module-separator replacement in path-miss diagnostics.
- The package conformance and differential smoke matrix now covers
  `package.searchpath` name/path/separator argument error result shapes.
- The package conformance and differential smoke matrix now covers `require`
  argument type and non-function searcher error result shapes.
- The package conformance and differential smoke matrix now covers default
  preload searcher missing/name type error result shapes.
- The package conformance and differential smoke matrix now covers default Lua
  searcher missing/name type error result shapes.
- The package conformance and differential smoke matrix now covers default
  searcher `package.path`/`package.cpath` type error result shapes.
- The package conformance and differential smoke matrix now covers
  `package.loadlib` missing/type argument error result shapes.
- The package conformance and differential smoke matrix now covers default C
  searcher missing/name type error result shapes.
- The `os` conformance and differential smoke matrix now covers argument error
  result shapes for supported file, locale, environment, time, and difftime
  paths.
- The `os` conformance and differential smoke matrix now covers `os.date`
  argument and `os.time` date-table field error result shapes.
- The `os.date` UTC formatter now supports portable `strftime` specifiers
  `%C`, `%D`, `%e`, `%I`, `%n`, `%p`, `%r`, `%R`, and `%t`, with matching
  conformance and official-Lua differential fixture coverage.
- The `os.date` UTC formatter now also supports week and ISO week-year
  specifiers `%u`, `%U`, `%W`, `%V`, `%G`, and `%g`, with matching
  conformance and official-Lua differential fixture coverage.
- The `os.date` UTC formatter now supports C-locale composite `strftime`
  specifiers `%c`, `%x`, and `%X`, with matching conformance and official-Lua
  differential fixture coverage.
- The `os.date` UTC formatter now recognizes Lua 5.5's C99 `E`/`O` modified
  `strftime` specifiers for supported C-locale and numeric date/time forms,
  with matching conformance and official-Lua differential fixture coverage.
- The shared `os.time` date-table fixtures now avoid timezone-dependent exact
  timestamps while still checking portable number-result and normalized-field
  shapes; local-only companion fixtures retain exact UTC timestamp coverage for
  Elara's current UTC-normalized `os.time` subset.
- `string.format` now supports Lua-style width and left-adjust modifiers for
  `%c` and `%p`, and the conformance/differential matrix covers portable nil
  pointer formatting.
- `string.format("%q", float)` now emits Lua-style hexadecimal finite float
  literals, so quoted scalar conformance matches official Lua.
- `string.gsub` now rejects invalid `%` escapes in replacement strings, and the
  conformance/differential error matrix covers that runtime error.
- The string-library conformance and differential smoke matrix now covers
  portable core string, slice, search, and substitution argument error result
  shapes.
- The string-library conformance and differential smoke matrix now covers
  additional transform, slice, search, iterator, and substitution argument
  error result shapes.
- The string-library conformance and differential smoke matrix now covers
  `string.char` byte-range error result shapes.
- The `string.format` conformance and differential smoke matrix now covers
  portable missing and wrong-type conversion argument error result shapes.
- The `string.format` conformance and differential smoke matrix now covers
  invalid modifier and width conversion specification error result shapes.
- The `string.format` long-string fixture now avoids implementation-specific
  `%p` result lengths while still exact-checking long literal, `%s`, `%q`, and
  precision-limited `%s` output shape.
- `string.gsub` now accepts numeric replacement arguments, matching Lua's
  replacement argument type set.
- The conformance and differential matrix now exactly covers table/function
  `string.gsub` replacements that return nil or false and preserve original
  matched text.
- The existing table/function `string.gsub` replacement fixtures now assert
  exact byte-level replacement results instead of only string-result classes.
- The existing `string.find`/`string.match` capture and backreference fixtures
  now assert exact captured bytes instead of only string-result classes.
- The existing `string.gmatch` position, capture, percent-class, callable, and
  leading-caret fixtures now assert exact iterator output bytes and positions.
- The existing no-match `string.gsub` fixture now asserts exact original-string
  bytes plus the zero substitution count.
- The simple compiler now preserves return-prefix locals when lowering a final
  open call whose call frame would otherwise overwrite registers still needed
  by nested call arguments.
- The simple compiler now lowers a final `...` in return position as an open
  vararg return, so vararg-forwarding functions preserve all returned values
  instead of truncating to the first.
- The language conformance table-field fixture now avoids a non-portable
  `rawlen` boundary on sparse array contents by using a contiguous array
  segment before checking exact table length.
- The existing base protected-error fixtures for `setmetatable`, `assert`, and
  `pcall` now assert exact string type-byte results instead of only string
  value classes.
- The existing `string.format("%p")` fixture now asserts the exact string
  type-byte result while keeping platform-dependent pointer text out of the
  portable expectation.
- The existing unsupported pre-file-handle `io.open`, `io.tmpfile`,
  `io.write`, and `io.flush` result fixtures now assert exact nil-result and
  string-message type-byte results instead of only message string classes.
- The configured official-Lua differential list now excludes explicitly
  unsupported pre-file-handle `io.tmpfile`, `io.write`, and `io.flush` stub
  result fixtures, while the shared `io_stubs` smoke fixture only covers
  portable missing-file `io.open` and non-file `io.type` nil classifications.
- The existing `package.searchpath` miss and unsupported `package.loadlib`
  fixtures now assert exact nil-result, string-message type-byte, and
  deterministic load stage results.
- The existing long-string `table.concat` fixture now builds its long operands
  from deterministic `string.rep` data instead of host/profile-dependent
  `package.path`, keeping exact-value conformance and differential comparison
  portable across Lua installations.
- The existing nil-loader `require` fixture now uses the portable global
  `require` entry point instead of Elara's exposed `package.require` helper,
  preserving nil-result cache behavior while matching official Lua's package
  surface.
- The existing loaded-cache `require` fixture now also uses the portable
  global `require` entry point for repeated cache reads instead of Elara's
  exposed `package.require` helper.
- The existing `os.execute`, `os.tmpname`, `os.remove`, and `os.rename`
  result fixtures now assert exact status/type booleans and type bytes while
  keeping host-specific labels, names, and errno values out of the expectation.
- The existing `coroutine.running` fixture now asserts exact thread type-byte
  and main-thread flag results, leaving only the generic success-fixture helper
  as the conformance test's custom success assertion path.
- The configurable official-Lua differential runner now has an exact primitive
  success-value comparison path for portable conformance fixtures, while
  keeping error fixtures on success/error class comparison.
- `string.rep` now treats an explicit nil separator like the missing separator
  default, matching Lua's `luaL_optlstring` behavior, and the shared conformance
  and differential fixture matrix covers that optional-argument path.
- The string-library conformance and differential matrix now covers
  `string.rep` with an empty source string, including separator insertion.
- The string-library conformance and differential matrix now covers
  `string.lower` and `string.upper` preserving embedded NUL bytes.
- The string-library conformance and differential smoke matrix now covers
  explicit nil optional-position defaults for `string.sub` and `string.byte`,
  matching Lua's `luaL_optinteger` paths.
- `table.unpack` now treats explicit nil start/end bounds like omitted bounds,
  and the shared conformance/differential matrix covers the Lua `luaL_opt`
  default-length path.
- The table-library conformance and differential smoke matrix now covers
  explicit nil optional defaults for `table.remove` positions and
  `table.concat` bounds.
- The base-library conformance and differential smoke matrix now covers
  `tonumber` with an explicit nil base, matching the standard conversion path.
- `select` now treats any string beginning with `#` as the count form, matching
  Lua's base-library prefix check, and the conformance/differential matrix
  covers that path.
- `assert` now raises Lua's default `"assertion failed!"` message when the
  custom message is omitted and Lua's `"<no error object>"` message when the
  custom message is explicit nil; the shared fixture matrix also covers
  `error()` and `error(nil)` nil-error-object messages.
- String-library receiver arguments now coerce numeric values through Lua-style
  string conversion for common byte-oriented operations, and the shared
  conformance/differential matrix covers numeric receivers for `string.len`,
  `string.byte`, `string.rep`, and `string.sub`.
- The string conformance and differential matrix now covers explicit nil
  required argument errors for core string receivers, `string.char`, and
  `string.rep`.
- `string.gsub` numeric replacement arguments now share the string-library
  numeric conversion path, preserving Lua-style integral-float replacement text
  such as `1.0`; the existing numeric-replacement fixture now covers that
  exact byte shape.
- The base-library conformance and differential matrix now covers `xpcall`
  passing a caught string error to the handler and returning multiple handler
  values after the leading false status.
- The base-library conformance and differential matrix now covers `pcall`
  catching non-callable target errors and returning a string error object.
- The base-library conformance and differential matrix now covers protected
  `rawset` rejection of nil table keys.
- The base-library conformance and differential matrix now covers `rawget`
  ignoring trailing arguments after the table and key.
- The base-library conformance and differential matrix now covers `rawlen`
  ignoring trailing arguments after the table or string operand.
- The base-library conformance and differential matrix now covers explicit nil
  required argument errors for raw table helpers, `next`, `setmetatable`, and
  `select`.
- The base-library conformance and differential matrix now covers
  `getmetatable` and `setmetatable` ignoring trailing arguments.
- The base-library conformance and differential matrix now covers `next`
  ignoring trailing arguments after the optional table key.
- The base-library conformance and differential matrix now covers `type`
  ignoring trailing arguments after the inspected value.
- The base-library conformance and differential matrix now covers `tostring`
  ignoring trailing arguments after the converted value.
- The base-library conformance and differential matrix now covers `tostring`
  preserving embedded NUL bytes in string operands.
- The base-library conformance and differential matrix now covers `rawequal`
  ignoring trailing arguments after the two compared values.
- `string.format` now reports Lua-style modified-`%q` errors instead of the
  generic unsupported-conversion gap, and the shared format-spec error fixture
  covers the `%10q` rejection shape.
- `string.format` now reports Lua-style invalid-conversion errors for
  unsupported alphabetic conversion items such as `%n` and `%F`, while still
  preserving Lua's missing-argument precedence for those invalid items.
- `string.format` integer conversions now require exact integer values for
  float and numeric-string arguments instead of flooring non-integral inputs;
  the shared argument-error fixture covers non-integral `%d` values.
- Standard numeric string conversion now accepts signed hexadecimal integers
  such as `+0x10` and `-0x10`; shared `tonumber` and `string.format`
  fixtures cover that Lua-style parsing path.
- The base-library conformance and differential matrix now covers
  explicit-base `tonumber` trimming surrounding whitespace around signed
  radix integers.
- Standard numeric string conversion now also accepts Lua-style hexadecimal
  floats such as `0x1.8p1` and `0x10.8`, with shared `tonumber` and
  `string.format` fixture coverage.
- String pattern preflight now distinguishes malformed Lua patterns from
  valid-but-unsupported pattern gaps, reporting Lua-style errors for trailing
  `%`, missing bracket/frontier/balanced-pattern delimiters, empty
  bracket/frontier classes, invalid capture indexes, out-of-range capture
  references, too many captures, unmatched close captures, and unfinished
  captures; shared conformance/differential fixtures cover those
  malformed-pattern error classes.
- String pattern preflight now accepts literal `^` and `$` outside their anchor
  positions, and the shared conformance/differential matrix covers those
  literal-anchor byte matches while preserving true terminal anchoring.
- String pattern bracket classes now accept literal `]` as the first class item
  after `[` or `[^`, matching Lua forms such as `[]]` and `[^]]`; the shared
  conformance/differential matrix covers literal and negated `]` classes.
- String-library integer arguments now accept exact floats and numeric strings,
  including hexadecimal numeric strings, across common byte/range/repetition
  and pattern-init paths while rejecting non-integral values.
- The string-library conformance and differential matrix now covers
  `string.sub` ignoring trailing arguments beyond the optional end index.
- The string-library conformance and differential matrix now covers
  `string.byte` ignoring trailing arguments beyond the optional end index.
- The string-library conformance and differential matrix now covers
  `string.rep` ignoring trailing arguments beyond the optional separator.
- The string-library conformance and differential matrix now covers
  `string.find` treating an explicit nil init like the default initial
  position while honoring the optional plain flag.
- The string-library conformance and differential matrix now covers
  `string.match` treating an explicit nil init like the default initial
  position.
- The string-library conformance and differential matrix now covers
  `string.gmatch` treating an explicit nil init like the default initial
  position.
- The string-library conformance and differential matrix now covers
  `string.find` ignoring trailing arguments beyond the optional plain flag.
- The string-library conformance and differential matrix now covers
  `string.match` ignoring trailing arguments beyond the optional init index.
- The string-library conformance and differential matrix now covers
  `string.gmatch` ignoring trailing arguments beyond the optional init index.
- The string-library conformance and differential matrix now covers
  `string.gsub` ignoring trailing arguments beyond the optional replacement
  limit.
- The string-library conformance and differential matrix now covers
  `string.gsub` treating an explicit nil replacement limit like the default.
- UTF-8 library integer arguments now accept exact floats and numeric strings,
  including hexadecimal numeric strings, across character construction, range,
  and offset paths while rejecting non-integral values.
- The UTF-8 conformance and differential matrix now covers `utf8.len`
  ignoring trailing arguments beyond the optional lax flag.
- The UTF-8 conformance and differential matrix now covers `utf8.len`
  defaulting the optional end index when only a start index is supplied.
- The UTF-8 conformance and differential matrix now covers `utf8.len`
  treating explicit nil start and end bounds like their default positions.
- The UTF-8 conformance and differential matrix now covers `utf8.codepoint`
  ignoring trailing arguments beyond the optional lax flag.
- The UTF-8 conformance and differential matrix now covers `utf8.codepoint`
  defaulting the optional end index to the start index.
- The UTF-8 conformance and differential matrix now covers `utf8.codepoint`
  treating explicit nil start and end bounds like their default positions.
- The UTF-8 conformance and differential matrix now covers `utf8.offset`
  ignoring trailing arguments beyond the optional starting position.
- The UTF-8 conformance and differential matrix now covers `utf8.offset`
  treating an explicit nil starting position like the default position.
- The UTF-8 conformance and differential matrix now covers `utf8.offset(0)`
  at an exact character-start position.
- The UTF-8 conformance and differential matrix now covers `utf8.codes`
  ignoring trailing arguments beyond the optional lax flag.
- The UTF-8 conformance and differential matrix now covers explicit nil
  required argument errors for character construction, subject strings, and
  offset counts.
- Table-library integer arguments now accept exact floats and numeric strings,
  including hexadecimal numeric strings, across insertion, removal, move,
  unpack, and concat bounds while rejecting non-integral values.
- The table-library conformance and differential matrix now covers
  `table.sort` comparators that return non-boolean truthy values.
- The table-library conformance and differential matrix now covers
  `table.sort` on an empty table.
- The table-library conformance and differential matrix now covers
  `table.insert` rejecting an explicit nil position in the three-argument
  insertion form.
- The table-library conformance and differential matrix now covers
  `table.remove` ignoring trailing arguments beyond the optional position.
- The table-library conformance and differential matrix now covers
  explicit zero-position `table.remove` behavior for empty and non-empty
  arrays.
- The table-library conformance and differential matrix now covers
  `table.concat` ignoring trailing arguments beyond the optional end bound.
- The table-library conformance and differential matrix now covers
  `table.unpack` ignoring trailing arguments beyond the optional end bound.
- The table-library conformance and differential matrix now covers
  `table.move` ignoring trailing arguments beyond the optional destination.
- The table-library conformance and differential matrix now covers
  `table.move` treating an explicit nil destination table like the source
  table.
- The table-library conformance and differential matrix now covers
  `table.sort` ignoring trailing arguments beyond the optional comparator.
- The package conformance and differential matrix now covers the portable
  newline-delimited shape of `package.config`.
- The package conformance and differential matrix now covers explicit nil
  required string argument errors for `package.searchpath`, global and package
  `require`, and `package.loadlib`.
- The package conformance and differential matrix now covers explicit nil
  required module-name errors for the default package searchers.
- The math-library conformance and differential matrix now covers `math.abs`
  ignoring trailing arguments after the numeric operand.
- The math-library conformance and differential matrix now covers `math.type`
  ignoring trailing arguments after the inspected value.
- The math-library conformance and differential matrix now covers
  `math.tointeger` ignoring trailing arguments after the converted value.
- The math-library conformance and differential matrix now covers `math.ult`
  ignoring trailing arguments after the compared operands.
- The math-library conformance and differential matrix now covers `math.sqrt`
  ignoring trailing arguments after the numeric operand.
- The math-library conformance and differential matrix now covers
  `math.floor` and `math.ceil` ignoring trailing arguments.
- The math-library conformance and differential matrix now covers `math.log`
  ignoring trailing arguments beyond the optional base operand.
- The math-library conformance and differential matrix now covers
  `math.randomseed` ignoring trailing arguments after the optional second seed
  while returning the first two seeds.
- The math-library conformance and differential matrix now covers
  `math.randomseed` treating an explicit nil second seed like the default zero
  seed.
- The math-library conformance and differential matrix now covers
  `math.random` and `math.randomseed` rejecting explicit nil required
  operands rather than treating them like omitted arguments.
- The math-library conformance and differential matrix now covers explicit nil
  required operand errors for representative numeric and integer-only math
  functions.
- The math-library conformance and differential matrix now covers `math.modf`
  ignoring trailing arguments after the numeric operand.
- The math-library conformance and differential matrix now covers `math.modf`
  integer and fractional results for zero.
- The math-library conformance and differential matrix now covers `math.fmod`
  ignoring trailing arguments after the divisor operand.
- The math-library conformance and differential matrix now covers `math.fmod`
  with both dividend and divisor negative.
- The math-library conformance and differential matrix now covers `math.ult`
  unsigned ordering across `math.maxinteger` and `math.mininteger`.
- The math-library conformance and differential matrix now covers `math.ldexp`
  ignoring trailing arguments after the exponent operand.
- The math-library conformance and differential matrix now covers `math.ldexp`
  with a zero mantissa.
- The math-library conformance and differential matrix now covers `math.frexp`
  ignoring trailing arguments after the numeric operand.
- The math-library conformance and differential matrix now covers `math.deg`
  and `math.rad` ignoring trailing arguments.
- The math-library conformance and differential matrix now covers zero-case
  `math.sin`, `math.cos`, `math.tan`, and `math.exp` trailing arguments.
- The math-library conformance and differential matrix now covers zero-case
  inverse-trig and optional-`math.atan` trailing arguments.
- The math-library conformance and differential matrix now covers
  two-argument `math.atan` quadrant handling beyond the positive-axis case.
- The math-library conformance and differential matrix now covers `math.min`
  and `math.max` preserving the numeric subtype of the selected operand,
  including equal-value tie behavior.
- The string library now exposes executable `string.packsize` for fixed-size
  binary packing formats, including Lua-style alignment and variable-length
  format rejection; a shared conformance/differential fixture covers portable
  fixed-size and error-class results against official Lua 5.5.
- The string library now exposes executable `string.pack` for integer, float,
  fixed-string, length-prefixed string, zero-terminated string, padding,
  alignment, and endian-control format items; a shared conformance/differential
  fixture covers portable byte output and value-error classes against official
  Lua 5.5.
- The string library now exposes executable `string.unpack` for integer, float,
  fixed-string, length-prefixed string, zero-terminated string, padding,
  alignment, endian-control, optional-position, and next-position behavior; a
  shared conformance/differential fixture covers portable returned values and
  error classes against official Lua 5.5.
- The coroutine conformance and differential matrix now also covers resuming a
  dead coroutine after normal completion, checking the Lua-style `false`,
  string-error, and `"dead"` status shape against official Lua 5.5.
- The same coroutine matrix now covers `coroutine.wrap` after normal
  completion, checking the second wrapped call's error class via `pcall`
  against official Lua 5.5.
- The package conformance and differential matrix now covers loaders that
  return `false`, checking that `require` returns and stores `false` but reloads
  on a later call because `false` is not a loaded sentinel.
- The package conformance and differential matrix now covers
  `package.searchpath` ignoring trailing arguments beyond the optional directory
  separator.
- The package conformance and differential matrix now covers
  `package.searchpath` treating explicit nil module and directory separators
  like their default separator values.
- The package conformance and differential matrix now covers
  `package.loadlib` ignoring trailing arguments after the init-function name.
- The package conformance and differential matrix now covers cached `require`
  and `package.require` ignoring trailing arguments after the module name.
- The package conformance and differential matrix now covers custom-searcher
  `require` ignoring trailing caller arguments while preserving loader data.
- The language conformance and differential matrix now covers open vararg
  forwarding through call arguments, including `select("#", ...)`.
- The language conformance and differential matrix now covers open
  call-expression result forwarding through later call arguments.
- The language conformance and differential matrix now covers non-final
  call-expression and vararg truncation in table constructor array fields.
- Table constructor lowering and execution now expand final open call-expression
  and vararg array fields through a dedicated `SET_LIST` bytecode operation.
- The language conformance and differential matrix now covers parenthesized
  call-expression and vararg table constructor fields truncating to one value.
- The string conformance and differential matrix now covers `string.format`
  `%a`/`%A` hexadecimal-float conversions and precision markers.
- The string conformance and differential matrix now covers `string.format`
  `%a` width, sign, zero-padding, left-adjust, and alternate-form flags.
- The string conformance and differential matrix now covers `string.format`
  non-hex float conversions for fixed, exponential, and general forms,
  including decimal and hexadecimal numeric-string arguments.
- The string conformance and differential matrix now covers `string.format`
  non-hex float alternate-form output for fixed, exponential, and general
  conversions.
- The string conformance and differential matrix now covers `string.format`
  non-hex float precision for fixed, exponential, and general conversions.
- The string conformance and differential matrix now covers `string.format`
  non-hex float width, sign, space, zero-padding, and left-adjust flags.
- The string conformance and differential matrix now covers malformed
  `string.format` float precision specifications for decimal and hexadecimal
  float conversions.
- The string conformance and differential matrix now covers `string.format`
  unsigned integer rendering of negative values for `%u` and `%x`.
- The string conformance and differential matrix now covers `string.format`
  basic integer-family conversions across `%d`, `%i`, `%u`, `%o`, `%x`, and
  `%X`, including exact-integer numeric-string arguments.
- The string conformance and differential matrix now covers broader
  `string.format` integer width, left-adjust, zero-padding, and hexadecimal
  casing behavior.
- The string conformance and differential matrix now covers malformed
  `string.format` integer width, precision, alternate-form, and unsigned-sign
  conversion specifications.
- The string conformance and differential matrix now covers `string.format`
  `%c` output that includes an embedded NUL byte.
- The string conformance and differential matrix now covers `string.format`
  `%q` control-byte escaping before following digit bytes.
- The string conformance and differential matrix now covers `string.format`
  `%q` escaping for quote, backslash, and newline bytes.
- The string conformance and differential matrix now covers `string.packsize`
  malformed fixed-format errors for missing sizes, invalid alignment,
  oversize integers, invalid options, and invalid `X` next options.
- The string conformance and differential matrix now covers `string.packsize`
  missing-format and non-string-format argument errors.
- The string conformance and differential matrix now covers `string.packsize`
  ignoring trailing arguments after the format string.
- The string conformance and differential matrix now covers `string.unpack`
  missing-format, non-string-format, missing-data, and non-string-data
  argument errors.
- The string conformance and differential matrix now covers `string.unpack`
  malformed fixed-format errors for missing sizes, invalid alignment,
  oversize integers, invalid options, and invalid `X` next options.
- The string conformance and differential matrix now covers `string.unpack`
  data errors for short input data, unfinished zero-terminated strings, and
  out-of-range initial positions.
- The string conformance and differential matrix now covers `string.unpack`
  length-prefixed and zero-terminated string results plus next-position
  reporting.
- The string conformance and differential matrix now covers `string.unpack`
  positive initial-position reads and next-position reporting.
- The string conformance and differential matrix now covers `string.unpack`
  treating an explicit nil initial position like the default first byte while
  ignoring trailing arguments.
- The string conformance and differential matrix now covers `string.pack`
  missing-format, non-string-format, missing-value, and non-integer-value
  argument errors.
- The string conformance and differential matrix now covers `string.pack`
  ignoring extra values beyond those consumed by the format string.
- The string conformance and differential matrix now covers `string.pack`
  malformed fixed-format errors for missing sizes, invalid alignment,
  oversize integers, invalid options, and invalid `X` next options.
- The string conformance and differential matrix now covers `string.pack`
  value errors for integer overflow, unsigned overflow, overlong fixed
  strings, and embedded-zero zero-terminated strings.
- The string conformance and differential matrix now covers `string.pack` and
  `string.unpack` fixed-endian signed integers, unsigned integers,
  fixed-length strings, zero-terminated strings, and next-position reporting.
- The string conformance and differential matrix now covers `string.pack` and
  `string.unpack` one-byte signed and unsigned integer boundary values.
- The string conformance and differential matrix now covers `string.pack` and
  `string.unpack` fixed-endian float and double values.
- The string conformance and differential matrix now covers `string.pack` and
  `string.unpack` fixed-endian native-number values.
- The string conformance and differential matrix now covers `string.pack` and
  `string.unpack` explicit alignment padding, `X` padding, and next-position
  reporting.
- The string conformance and differential matrix now covers `string.pack` and
  `string.unpack` explicit `x` padding bytes and next-position advancement.
- The string conformance and differential matrix now covers `string.pack` and
  `string.unpack` mixed big-endian and little-endian integer fields.
- The string conformance and differential matrix now covers `string.pack` and
  `string.unpack` mixed-endian signed integer fields.
- The string conformance and differential matrix now covers `string.len`
  ignoring trailing arguments beyond the inspected string.
- The string conformance and differential matrix now covers `string.reverse`
  ignoring trailing arguments beyond the inspected string.
- The string conformance and differential matrix now covers `string.reverse`
  numeric receiver coercion.
- The string conformance and differential matrix now covers `string.lower` and
  `string.upper` ignoring trailing arguments beyond the inspected string.
- The string conformance and differential matrix now covers `string.lower` and
  `string.upper` numeric receiver coercion.
- The string conformance and differential matrix now covers `string.rep`
  numeric separator coercion.
- The debug conformance and differential matrix now covers `debug.getregistry`
  ignoring trailing arguments while returning the stable registry table.
- The debug conformance and differential matrix now covers
  `debug.getmetatable` ignoring trailing arguments after the inspected value.
- The debug conformance and differential matrix now covers
  `debug.setmetatable` ignoring trailing arguments after the metatable value.
- The debug conformance and differential matrix now covers no-hook
  `debug.gethook` ignoring trailing non-thread arguments.
- The debug conformance and differential matrix now covers `debug.sethook`
  ignoring trailing arguments after the optional count.
- The debug conformance and differential matrix now covers `debug.getupvalue`
  ignoring trailing arguments after the upvalue index.
- The debug conformance and differential matrix now covers `debug.setupvalue`
  ignoring trailing arguments after the replacement value.
- The debug conformance and differential matrix now covers `debug.upvalueid`
  ignoring trailing arguments after the upvalue index.
- The debug conformance and differential matrix now covers `debug.upvaluejoin`
  ignoring trailing arguments after the source upvalue index.
- The debug conformance and differential matrix now covers function-target
  `debug.getlocal` ignoring trailing arguments after the local index.
- The debug conformance and differential matrix now covers `debug.setlocal`
  ignoring trailing arguments after the replacement value.
- The debug conformance and differential matrix now covers function-target
  `debug.getinfo` ignoring trailing arguments after the options string.
- The debug conformance and differential matrix now covers function-target
  `debug.getinfo` treating explicit nil options like the default option set.
- The debug conformance and differential matrix now covers `debug.traceback`
  ignoring trailing arguments beyond the optional level.
- The debug conformance and differential matrix now covers `debug.traceback`
  treating an explicit nil level like the default level.
- The debug conformance and differential matrix now covers pre-userdata
  `debug.getuservalue` treating an explicit nil uservalue index like the
  default index.
- The debug conformance and differential matrix now covers `debug.sethook`
  treating an explicit nil count like the default zero count.
- The debug conformance and differential matrix now covers explicit nil
  required argument errors for local/upvalue helpers and hook mask validation.
- The io conformance and differential matrix now covers absent-file `io.open`
  ignoring trailing arguments after the optional mode.
- The io conformance and differential matrix now covers absent-file `io.open`
  treating an explicit nil mode like the default read mode before returning
  the safe pre-file-handle result shape.
- The io conformance and differential matrix now covers pre-file-handle
  `io.popen` treating an explicit nil mode like the default read mode before
  returning the safe result shape.
- The io conformance and differential matrix now covers `io.open` and
  `io.popen` rejecting explicit nil required filename/command arguments before
  returning pre-file-handle stub results.
- The io conformance and differential matrix now covers non-file `io.type`
  ignoring trailing arguments after the inspected value.
- The os conformance and differential matrix now covers `os.difftime`
  ignoring trailing arguments after the two time operands.
- The os conformance and differential matrix now covers `os.execute` treating
  an explicit nil command like the no-argument shell-availability query while
  ignoring trailing arguments.
- The os conformance and differential matrix now covers `os.getenv` ignoring
  trailing arguments after the variable name.
- The os conformance and differential matrix now covers explicit nil required
  string argument errors for `os.getenv`, `os.remove`, and `os.rename`.
- The os conformance and differential matrix now covers `os.clock` ignoring
  trailing arguments while returning a numeric elapsed-time value.
- The os conformance and differential matrix now covers `os.tmpname` ignoring
  trailing arguments while returning a string value.
- The os conformance and differential matrix now covers `os.setlocale`
  ignoring trailing arguments after the optional category.
- The os conformance and differential matrix now covers `os.setlocale`
  treating explicit nil locale and category arguments like their default
  query/category values.
- The os conformance and differential matrix now covers `os.remove` ignoring
  trailing arguments after the filename.
- The os conformance and differential matrix now covers `os.rename` ignoring
  trailing arguments after the destination filename.
- The os conformance and differential matrix now covers table-form `os.time`
  ignoring trailing arguments after the date table.
- The os conformance and differential matrix now covers no-table `os.time`
  treating an explicit nil first argument like the current-time default while
  ignoring trailing arguments.
- The os conformance and differential matrix now covers `os.date` ignoring
  trailing arguments after the optional time.
- The os conformance and differential matrix now covers UTC string
  `os.date` treating an explicit nil time argument like the current-time
  default while ignoring trailing arguments.
- The package conformance and differential matrix now covers loaders that set
  `package.loaded` while returning nil, plus `require` searchers that return
  non-string, non-loader miss values before a later searcher succeeds.
- Base `warn` now emits host warnings when enabled, honors single-argument
  `@on`/`@off` control messages, ignores unknown control messages, and keeps
  warnings disabled by default.
- The explicit conformance harness now registers every Lua smoke fixture in
  `tests/conformance`, and the optional official-Lua differential list includes
  the portable recent stdlib additions while leaving intentionally divergent
  host/unsupported cases out of the comparison set.
- The language conformance and differential matrix now covers raw string
  relational comparisons for `<`, `<=`, `>`, and `>=` through the public API.
- The simple compiler now lowers long-bracket string literals in expression
  positions, including Lua's skipped-initial-newline rule, with matching
  language conformance and differential coverage.
- The simple compiler now decodes quoted string literal escapes for named
  control escapes, escaped quotes/backslashes, hexadecimal and decimal byte
  escapes, UTF-8 codepoint escapes, escaped newlines, and `\z` whitespace
  skipping, with matching language conformance and differential coverage.
- Raw equality comparison now compares runtime string byte contents before
  `__eq` fallback, so independently allocated long strings with the same bytes
  compare equal; language conformance and differential fixtures cover that
  public behavior.
- Table hash keys now accept long strings and compare all string keys by byte
  value while still tracing the stored GC reference; language conformance and
  differential fixtures cover long-string key lookup.
- The base-library conformance and differential matrix now covers `rawset` and
  `rawget` with independently allocated long-string keys.
- The string-library conformance and differential matrix now covers
  `string.match` with negative init positions, including clamping before the
  start of the subject.
- The string-library conformance and differential matrix now covers empty
  patterns across `string.find`, `string.match`, and `string.gsub`.
- `string.find` and `string.match` now allow empty matches at `#s + 1`;
  shared conformance and differential fixtures cover the final-init boundary.
- `string.gmatch` now also allows a final empty match at `#s + 1` while still
  stopping for larger init positions.
- The string-library conformance and differential matrix now covers empty
  subject handling across `find`, `match`, `gsub`, and `gmatch`.
- The string-library conformance and differential matrix now covers
  `string.gmatch` negative, zero, and past-end init positions.
- The string-library conformance and differential matrix now covers
  `string.gsub` empty-pattern replacements with explicit limits.
- The string-library conformance and differential matrix now covers
  `string.gsub` table and function replacements for empty-pattern matches.
- The string-library conformance and differential matrix now covers
  `string.gsub` empty-pattern replacement fallbacks for missing table entries
  and false or nil function results.
- The table-library conformance and differential matrix now covers
  `table.move` over negative and zero integer source indices.
- The table-library conformance and differential matrix now covers
  `table.move` into negative and zero integer destination indices.
- The table-library conformance and differential matrix now covers
  `table.concat` error behavior for nil holes inside the requested range.
- The table-library conformance and differential matrix now covers
  `table.concat` ignoring nil holes outside explicit bounds.
- The table-library conformance and differential matrix now covers
  `table.sort` ordering mixed integer and float values.
- The table-library conformance and differential matrix now covers
  `table.sort` comparator error propagation through `pcall`.
- The math-library conformance and differential matrix now covers
  `math.floor` and `math.ceil` returning integer-typed results for rounded
  float inputs.

## Remaining Gaps

### Release Conformance Dashboard

- `tests/conformance` currently contains seven hundred nine smoke fixtures across
  language, standard-library, runtime-error, and coroutine cases. Success
  fixtures check exact portable primitive result vectors through the public API.
- `crates/elara-api/tests` provides broader public-API coverage for `debug`,
  `io`, `os`, and `package` behavior, and crate-local unit tests cover the
  bulk of base/table/math/string/utf8 native behavior, but these are not a
  substitute for a broad official Lua conformance corpus.
- Differential test utilities exist and can invoke an `ELARA_LUA` reference
  interpreter. The portable conformance smoke fixtures can now compare exact
  primitive success values and success/error classes against official Lua, but
  there is not yet a release-sized differential fixture set.
### Explicitly Scoped Unsupported Behavior

- Implement coroutine suspension from `coroutine.yield`, yielding resume and
  wrap behavior, and full primitive-backed close semantics.
- File-handle-backed `io` behavior is intentionally represented by safe
  unsupported stubs until runtime file handles are implemented.
- Dynamic Lua file loading and dynamic C library loading remain unsupported;
  `load`, `loadfile`, `dofile`, `package.loadlib`, and C searchers report
  explicit unsupported-loader behavior until dynamic chunk and host file
  loading are implemented.
- Base-library `collectgarbage` is registered as an explicit unsupported stub.
- `os.exit` validates arguments but does not terminate the host process.
- C API source compatibility is tested for core stack/call usage, but binary
  compatibility with existing Lua modules is not promised.
- Remaining unsupported `string.format` conversion forms and unsupported or
  malformed string-pattern forms report explicit runtime errors.

### Product Gaps

Major implementation work is still pending:

- Broader full-profile standard-library conformance.
- Release-sized conformance and differential fixture coverage.

M9 is complete.
M10.1 is complete.
M10.2 is complete.
M10.3 is complete.
M10.4 is complete.
M10 is complete.
M11.1 is complete.
M11.2 is complete.
M11.3 is complete with explicit full-profile coroutine gaps.
M11.4 is complete.
M11 is complete.
M12.1 is complete.
M12.2 is complete.
M12.3 is complete.
M12.4 is complete.
M12 is complete.
M13.1 is complete.
M13.2 is complete.
M13.3 is complete.
M13 is complete.
M14.1 is complete.
M14.2 is complete.
M14.3 is complete.
M14.4 is complete.
M14 is complete.
M15.1 is complete.
M15.2 is complete.
M15.3 is complete.
M15.4 is complete.
M15 is complete.
M16.1 is complete.
M16.2 is complete.
M16.3 is complete.
M16.4 is complete.
M16 is complete.
M17.1 is complete.
M17.2 is complete.
M17.3 is complete.
M17.4 is complete.
M17 is complete.
M18.1 is complete.
M18.2 is complete.
M18.3 is complete.
M18.4 is complete.
M18 is complete.
M19.1 is complete.
M19.2 is complete.
M19.3 is complete.
M19.4 is complete.
M19 is complete.
M20.1 is complete.
M20.2 is complete.
M20.3 is complete.
M20.4 is complete.

## Last Verification

Latest focused verification passed:

```bash
cargo test -p elara-test conformance_standard_library_fixtures
cargo test -p elara-test --test differential_fixtures
```

`cargo fmt -p elara-test -- --check` currently reports pre-existing formatting
drift in committed Rust files outside the current conformance fixture expansion.

`cargo test -p elara-test --test differential_fixtures` passed in this
environment with no `ELARA_LUA` configured, so the optional official-Lua
comparison paths skipped after building the fixture matrix successfully.

## Next Recommended Action

Continue reducing remaining release gaps by adding more full-profile
standard-library fixtures and growing the optional official-Lua differential
fixture set beyond the current smoke matrix.

## Current Risk Notes

- Do not introduce old Lua dialect infrastructure during bootstrap.
- Do not over-design JIT before bytecode and interpreter semantics exist.
- Keep `elara-core` independent from parser, compiler, stdlib, and JIT.
- Keep crate files small and focused as implementation begins.
- Keep `docs/PROGRESS.md` concise and current; do not append historical diary entries.

## Status Table

| Area | Status | Notes |
|---|---|---|
| Project naming | Complete | Elara selected. |
| Language target | Complete | Latest stable Lua only. |
| Architecture docs | Drafted | Present in `docs/`. |
| Milestone plan | Drafted | Present in `docs/`. |
| Codex goal | Drafted | Present in `docs/`. |
| Workspace | Complete | Virtual workspace and placeholder member crates exist. |
| CI and lints | Complete | Cargo lints, rustfmt, and GitHub Actions are configured. |
| Crate boundary docs | Complete | Module docs describe crate responsibilities and dependencies. |
| Lua spec | Complete | `elara-core` declares Lua 5.5 / 5.5.0 as the only current target. |
| Diagnostics | Complete | Source spans and structured diagnostics are in `elara-core`. |
| Test fixtures | Complete | Fixture, conformance, and differential directories are present. |
| Snapshots | Complete | Snapshot helper and `return 42` diagnostics baseline exist. |
| Core runtime | Initial foundation complete | Value, light userdata, GC, string, and table foundations are implemented. |
| Value primitives | Complete | Nil, bool, integer, and float values are implemented. |
| GC headers | Complete | Headers, colors, kinds, and typed refs are implemented. |
| GC allocation | Complete | Arena allocation list, stats, roots, and drop cleanup are implemented. |
| GC | M14 complete | Root marking, transitive tracing, weak table cleanup, ephemeron marking, finalizer queueing, incremental mode state, write barriers, and allocation-list sweeping are implemented for tests. |
| Strings | Complete | Short strings, long strings, and interning are implemented. |
| Table array | Complete | Raw 1-based array get/set and nil clearing are implemented. |
| Table hash | Complete | Hash storage and numeric key canonicalization are implemented. |
| Table metadata | Complete | Metatable pointer, meta flags, and structural versioning are implemented. |
| Lexer | Complete | Lua 5.5 tokens, literals, comments, and lexical diagnostics are implemented. |
| Expression parser | Complete | Expression AST, precedence parsing, calls, table constructors, and varargs are implemented. |
| Statement parser | Complete | Declarations, assignments, control flow, function declarations, labels, and returns are implemented. |
| Parser snapshots | Complete | Representative AST and malformed syntax diagnostic snapshots are implemented. |
| Bytecode model | M18 complete | Proto, instruction encoding, opcode set including boolean `NOT`, constants, upvalues, source/line/local debug metadata, builder, disassembler, verifier, internal dump/load format with magic/version/header validation, and explicit unsupported official Lua chunk policy are implemented. |
| Compiler | Initial MVP complete | Simple return-expression codegen emits verified bytecode, including long string literals, non-clobbering unary and binary operators, logical `and`/`or`, comparison expression lowering, and fixed-result lowering for bare call statements. |
| VM/thread stack | Complete | VM state, Lua thread stack, call frames, and stack helpers are implemented. |
| Interpreter | M15 complete | Primitive bytecode execution includes structured errors, coroutines, close variables, native calls, bitwise operators, boolean `NOT`, hot stack helpers, table/global inline caches, and an `ADD_INT` superinstruction. |
| Variables/scopes | Complete | Local variables, assignment basics, fixed-parameter calls, captured outer local reads and writes through shared runtime upvalue cells with parent stack-slot synchronization after nested calls, anonymous and named varargs, multiple call results, and recursive self-reference are implemented. |
| Control flow | Complete | Conditional branches, `while`, `repeat`, `break`, numeric `for`, and generic `for` execute through bytecode. |
| Tables/globals/metamethods | Complete for M9 | Table constructors, raw table access, table/function-valued `__index`/`__newindex`, arithmetic/bitwise/comparison metamethods, `__len`, `__call`, `__concat`, global declarations, and default `_ENV` execute. |
| Standard library | M18.2 complete | Base, coroutine, table, math, string, utf8, safe unsupported pre-file-handle `io.close`, `io.flush`, `io.input`, `io.lines`, `io.open`, `io.output`, `io.popen`, `io.read`, `io.tmpfile`, and `io.write`, pre-file-handle `io.type`, `os.clock`, UTC table and string-format `os.date`, `os.difftime`, `os.execute`, safe unsupported `os.exit`, `os.getenv`, `os.remove`, `os.rename`, C-locale subset `os.setlocale`, `os.tmpname`, no-argument and UTC date-table `os.time`, global `require`, `package.config`, `package.cpath`, `package.loadlib` unsupported-C-loader behavior, `package.loaded`, `package.path`, `package.preload`, preloaded-module `package.require`, `package.require` searcher miss aggregation, custom `package.searchers` entries for `require`, default preload `package.searchers[1]`, default Lua path `package.searchers[2]`, default C path searchers in `package.searchers[3]` and `[4]`, `package.searchpath`, `debug.gethook`/`debug.sethook` hook metadata installation and clearing plus call/return/line/count hook callback dispatch, `debug.getinfo` runtime-hook validation and current-thread frame materialization, read-only stack-level `debug.getlocal`, function-target `debug.getlocal` parameter names, stack-level `debug.setlocal` for current-thread Lua frames, and primitive coroutine debug frames for native debug calls, read-only `debug.getupvalue`, `debug.setupvalue` over shared runtime upvalue cells, `debug.upvalueid`, `debug.upvaluejoin`, raw `debug.getmetatable`, `debug.getregistry`, pre-userdata `debug.getuservalue`, raw `debug.setmetatable`, pre-userdata `debug.setuservalue`, and `debug.traceback` message handling plus stack-frame formatting are implemented; base string-facing paths, `math.tointeger`, common byte-oriented `string` primitives, `string.format` including `%c`/`%p` width and left-adjust modifiers plus `%q` finite float hex literals, `string.pack` binary packing, `string.packsize` fixed-format sizing, `string.unpack` binary unpacking, string pattern results and replacements, `table.concat`, `table.sort` default string comparisons, executable `utf8` primitives, and executable `os` string paths handle runtime long strings; full-profile descriptors include `io`, `os`, `package`, and `debug` while host-sensitive executable registration remains gated. |
| Rust API | M20.3 audited | Builder/chunk evaluation, conversions, native functions, tables, registry keys, and userdata handles are implemented; native Rust callback string arguments and results handle runtime long strings; facade docs and `basic_embed` example compile against the safe public surface. |
| Conformance | Expanded post-M20.4 | Six hundred sixty-eight smoke fixtures run through the public API with exact portable primitive return-value checks and error-class checks for failure cases; broader API/unit coverage exists, but release-sized conformance remains a product gap. |
| Differential testing | Expanded post-M20.4 | Configurable official-Lua runner compares exact primitive success values and success/error classes with Elara, including the portable conformance smoke fixture set when `ELARA_LUA` is configured. |
| Fuzz targets | Initial M13 targets complete | Parser, bytecode verifier, and table-operation target entry points are test-covered. |
| JIT | M17 complete; M18.2 debug interaction complete | Optional Cranelift dependencies, feature plumbing, baseline ABI, helper registry, arithmetic lowering, hot counters, cached JIT entries, interpreter fallback, debug-hook forced interpretation, API JIT selection for environment-independent chunks with debug/runtime-environment chunks kept on the interpreter, deopt metadata/stack sync, table array fast-path guards, call trampoline statuses, and interpreter equivalence tests are implemented. |
| C API | M19 complete | Current-version `lua.h`, `lauxlib.h`, and `lualib.h` scaffolding is packaged by `elara-capi`; stack-backed `lua_State` top manipulation, push/copy/rotate behavior, primitive type inspection, basic conversions, stack-registered C function calls, protected-call result normalization, Rust callback panic containment, and source-level C module compilation against packaged headers are implemented. |
| Benchmarks | M20.2 complete | Stable custom `cargo bench` runner covers API overhead, arithmetic, table access, calls, strings, and representative macro workloads across interpreter API, JIT API, and official Lua 5.5 reference rows; `docs/PERFORMANCE.md` records the M20 release report and remaining methodology gaps. |
| Release candidate | M20.4 prepared | README usage and limitations are complete for the candidate; `docs/RELEASE.md` records version constants, gates, and tag plan; default and JIT embedding examples compile under all-features verification. |
