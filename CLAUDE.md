# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Build & Development Commands

This project uses [just](https://github.com/casey/just) as the task runner.

```bash
just check             # cargo check on workspace
just build             # debug build
just build-release     # release build
just test              # Rust tests with coverage (requires cargo-tarpaulin)
just verify            # full check: cargo check + tests + Angular build
just schema            # regenerate JSON schema for scenario config
just bump-version x.y.z  # update version across all packages
```

**Run individual interfaces (dev mode):**
```bash
just cli [args]        # CLI
just tui [args]        # TUI
just gui               # Tauri GUI (requires npm deps: just npm-install)
```

**Run a single Rust test:**
```bash
cargo test -p scenario-rs-core <test_name>
```

**Integration tests (Docker-managed):**
```bash
just test-scenarios           # spin up Docker, run tests, tear down
just test-scenarios --keep    # keep Docker container after tests
just test-scenarios --no-docker  # skip Docker management
```

## Architecture

**Workspace crates:**
- `core/` (`scenario-rs-core`) — all business logic; the other crates are thin UI shells
- `cli/` — clap-based CLI
- `tui/` — ratatui/crossterm terminal UI
- `gui/src-tauri/` — Tauri backend; `gui/src/` — Angular frontend

**Core module layout:**
```
core/src/
  config/    # TOML deserialization: PartialScenarioConfig (mergeable) → ScenarioConfig
  scenario/  # Runtime execution: Scenario, Steps, Tasks (RemoteSudo, SftpCopy), Variables
  session/   # SSH/SFTP abstraction over russh; also Mock and DryRun implementations
  state/     # Thread-safe ExecutionStateManager — polled by TUI/GUI
  trace/     # ScenarioEvent enum + custom tracing layers consumed by each UI
  utils/     # Shared utilities
```

**Execution flow:**
1. TOML config is loaded with optional parent-config inheritance into `ScenarioConfig`
2. `Scenario::new()` converts config into runtime types (`Steps`, `Tasks`, `Variables`, etc.)
3. SSH `Session` is established (real via russh, or Mock/DryRun)
4. Steps execute sequentially; each step runs one or more tasks; `OnFail` sequences trigger on failure
5. All events are emitted via `tracing` as `ScenarioEvent` variants and consumed by UI-specific layers

**Key design decisions:**

- **Trait-based SSH abstraction** (`Channel`, `Sftp`, `Write` traits in `session.rs`): real, mock, and dry-run sessions are interchangeable; this is how tests avoid live SSH.
- **Two-layer config**: `PartialScenarioConfig` supports `Option` fields for merging from parent configs; `ScenarioConfig` is the fully-resolved type used at runtime.
- **Structured tracing for UI decoupling**: each UI registers its own `tracing` layer (`ScenarioEventLayer` for CLI, `FrontendLayer` for GUI). Avoid printing/logging directly in core — emit `ScenarioEvent`s instead.
- **Async runtime guard**: Tauri already owns a Tokio runtime. `session.rs` detects this via `try_block_on` and spawns a dedicated thread to avoid nested-runtime panics.
- **Variable resolution** (`scenario/variables.rs`): regex-based placeholder expansion with a modifier pipeline (path, system/env, time, string ops). Variables are resolved lazily at execution time, not at config parse time.
- **`schemars` integration**: all config types derive `JsonSchema`; run `just schema` after any config type change.

## Config Schema

```toml
parent = "path/to/parent.toml"   # Optional: inherited and overridden

[credentials]
username = "string"               # REQUIRED (also injected as a variable)
password = "string"               # Optional — omit to use SSH agent
private_key = "./path/to/key"     # Optional — resolved relative to config dir

[server]
host = "string"                   # REQUIRED
port = 22                         # Optional, defaults to 22

[steps.step_name]
task = "task_name"                # Reference a single task  — OR —
sequence = "sequence_name"        # Reference a named sequence
on-fail = "sequence_name"         # Optional: recovery sequence on failure

[sequences]
sequence_name = ["task_1", "task_2"]

[variables.required]
var_name = { type = "String|Path|Timestamp", label = "...", read_only = false }

[variables.defined]
var_name = "value with {placeholders}"

[tasks.remote_sudo.task_name]
command = "shell command with {placeholders}"  # REQUIRED
description = "..."                            # Optional
error_message = "..."                          # Optional

[tasks.sftp_copy.task_name]
source = "local/path"                          # REQUIRED
destination = "remote/path"                    # REQUIRED
description = "..."                            # Optional
error_message = "..."                          # Optional
```

**Config validation (load-time):** `credentials.username`, `server.host`, `[execute]` (steps), and `[tasks]` are required. Circular parent chains are rejected. Task IDs referenced in steps that don't exist in the tasks map are caught at scenario creation time, not config load time.

## State Machine

**`ExecutionStatus`** (top-level): `Idle → Running → Completed | Failed { error }`

**`StepStatus`** (per step): `Pending → Running → Completed | Failed`

**Critical behavior — `on-fail` does NOT recover a step.** If a step's task fails and its `on-fail` sequence succeeds, the step is still marked `Failed` and scenario execution still stops after that step. On-fail is for cleanup/rollback, not recovery.

`ExecutionStateManager` maintains an `Arc<RwLock<ExecutionState>>` and emits `StateDiff` events over an mpsc channel. UIs consume `StateDiff` variants:
- `ExecutionStatusChanged`, `StepStatusChanged`, `StepProgressUpdated`
- `StepOutputAppended`, `StepErrorAdded`
- Equivalent `OnFailStep*` variants for recovery steps

## Error Hierarchy

| Layer | Key Error Types |
|---|---|
| Config load | `ScenarioConfigError`: `MissingCredentials`, `MissingUsername`, `MissingServer`, `MissingHost`, `MissingExecute`, `MissingTasks`, `CircularDependency` |
| Scenario creation | `ScenarioError::CannotCreateScenarioFromConfig`, `CannotCreateTaskFromConfig` (bad task ref in step) |
| Remote command | `RemoteSudoError`: `RemoteCommandFailedWithStatusCode(i32)` is the normal failure; also channel/exec/read/exit-status errors |
| SFTP | `SftpCopyError`: open source, init subsystem, create dest, read/write loop |
| On-fail | `OnFailError` — failures logged but execution continues to propagate the original step error |
| Variables | `PlaceholderResolutionError::CannotResolveVariablesPlaceholders(Vec<String>)` — lists all missing vars |

## Variable / Placeholder System

Format: `{variable_name}` or `{modifier:variable_name}`

**Modifier categories** (applied in pipeline order):
- **Path**: `basename`, `stem`, `dir`, `ext`, `abspath`
- **System**: `env:VAR_NAME`, `hostname`, `os`
- **Time/uniqueness**: `uuid`, `now`, `now:FORMAT`, `now:epoch`
- **String**: `uppercase`, `lowercase`, `base64`, `trim`

Variable merge order (later wins): built-ins (`username`) → `defined` → `required` (runtime-provided). Timestamp-typed required variables are auto-generated from their `format` string.
