# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/), and this project adheres to [Semantic Versioning](https://semver.org/).

---

## [Unreleased]

---

## [v0.1.1] — 2026-04-16

### Changed

- **Scope redefinition:** `orph` is now explicitly scoped as a *local-first cyberdeck CLI tooling system*; daemon (`orphd`) is positioned as an optional state accelerator, not a required component
- **UX — output clarity:** human-readable output reviewed across all commands; labels, spacing, and wording made consistent
- **UX — daemon offline banner:** `[daemon offline — running in local fallback mode]` message is now printed only once per invocation (removed duplicate lines in `sys` + `pet` combined flows)
- **UX — error messages:** actionable error hints added for `orph core start` (binary not found), `orph run` (script not found / bad name), and `orph cfg set` (missing value)

### Fixed

- ARM64 cross-compilation linker configuration in CI: linker now declared statically in `.cargo/config.toml` instead of being appended dynamically, preventing duplicate-key errors
- Clippy warnings resolved: `checked_div()` used for `mem_pct` and `disk_pct`; `RunFlags` type alias introduced to reduce type complexity

---

## [v0.1.0] — 2026-04-16

### Added
- `orph pet` defaults to `status` when called with no subcommand
- Auto-creates `~/.orph/scripts/` with a sample `hello` script on first `orph run list`
- Daemon transparency: shows `[daemon offline — running in local fallback mode]` in `sys` and `pet`
- `orph telemetry` shows disabled state when opted out
- `orph logs --follow` / `-f` flag for streaming; `--tail -n N` for last N lines
- Shell completions example and daemon explanation added to README

**CLI (`orph`)**
- `orph sys status` — CPU, memory, disk usage
- `orph sys info` — hostname, OS, kernel, uptime, CPU count
- `orph core start/stop/status` — daemon lifecycle management
- `orph run list` — list scripts with descriptions (from `#` comments)
- `orph run <script> [--timeout <s>]` — script execution with captured stdout/stderr, exit codes, timeout, path safety
- `orph logs [--tail] [--level]` — structured log viewer
- `orph pet status/feed/play/rename` — virtual pet with time-based decay
- `orph cfg list/get/set` — SQLite-backed configuration
- `orph telemetry` / `orph telemetry top` — local usage tracking
- `orph completions <bash|zsh|fish>` — shell completion generation
- `--json` / `--quiet` / `--verbose` global flags
- Daemon-aware IPC fallback for `sys`, `pet`, `cfg`, `logs`

**Daemon (`orphd`)**
- Unix socket server at `/tmp/orphd.sock`
- Sequential JSON IPC (newline-delimited)
- Handles: `sys.status`, `pet.*`, `cfg.*`, `logs.read`, `ping`, `shutdown`
- Clean socket cleanup on SIGTERM/SIGINT
- Structured error responses for invalid/unknown requests

**Storage & Infrastructure**
- SQLite database at `~/.orph/orph.db`
- Structured log file at `~/.orph/orph.log` (RFC3339 timestamps)
- Local telemetry (command + timestamp, no external transmission)
- Pet state time-based decay (hunger +10/h, happiness −5/h)
- DB migration support for schema evolution

### Fixed
- `orph run --timeout abc` now fails with a clear error instead of silently ignoring the value
- `orph core start` now falls back to `which orphd` and prints an actionable hint if not found
- `orph sys info` no longer sleeps unnecessarily (saves ~100ms per call)
- `print_pet` serialization errors now propagate correctly instead of producing empty output
- `--help` descriptions are now actionable for all subcommands

---

[Unreleased]: https://github.com/CoreRed-Project/orph-cli/compare/v0.1.1...HEAD
[v0.1.1]: https://github.com/CoreRed-Project/orph-cli/compare/v0.1.0...v0.1.1
[v0.1.0]: https://github.com/CoreRed-Project/orph-cli/releases/tag/v0.1.0
