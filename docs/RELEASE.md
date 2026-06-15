# Release Candidate Plan

Status: M20.4 release-candidate plan  
Last updated: 2026-06-15

This document records the version constants, release gates, and tag plan for the
first Elara release candidate.

## Version Matrix

| Item | Value | Source |
|---|---:|---|
| Elara crate/runtime version | `0.1.0` | Workspace `Cargo.toml` |
| Rust edition | `2024` | Workspace `Cargo.toml` |
| License | `MIT OR Apache-2.0` | Workspace `Cargo.toml` |
| Lua language target | `Lua 5.5 / Lua 5.5.0` | `elara-core::LUA_SPEC` |
| Lua C header version | `LUA_VERSION_NUM 505`, `LUA_RELEASE "Lua 5.5.0"` | `crates/elara-capi/include/lua.h` |
| Optional JIT backend | Cranelift `0.132.1` | Workspace dependencies |

Elara's crate version and Lua's language version are separate. Rust API
stability follows Rust semver. Language conformance follows the current Lua
target declared in `elara-core::LUA_SPEC`.

## Release Gates

Run these commands before tagging:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --all-features --all-targets
cargo doc --workspace --all-features --no-deps
cargo bench -p elara-bench
git diff --check
```

The release candidate also expects:

- README documents usage, features, verification, and limitations.
- `docs/PROGRESS.md` lists all known unsupported behavior and product gaps.
- `docs/PERFORMANCE.md` contains the latest local release benchmark report.
- `docs/SAFETY_API_AUDIT.md` records the unsafe and public API audit.
- Examples under `crates/elara/examples` compile with all targets.

## Tag Plan

After the release gates pass on `main`, create:

```text
v0.1.0-rc.1
```

When Lua 5.5 support is considered complete enough to preserve independently of
future latest-Lua tracking, create:

```text
lua-5.5-complete
```

After release-candidate validation, promote the runtime release tag to:

```text
v0.1.0
```

If a later Lua release becomes the main target, create an optional maintenance
branch before retargeting `main`:

```text
branch/lua-5.5
```
