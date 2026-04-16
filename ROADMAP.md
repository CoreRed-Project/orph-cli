# ROADMAP — orph

## Status: Priority 4 (orphd v1) — Complete ✅

---

## Implemented

- `orph sys status` — CPU, memory, and disk usage (daemon-aware)
- `orph sys info` — hostname, OS, kernel, uptime, CPU count
- `orph core status` — real daemon liveness check via socket ping
- `orph core start` — spawn orphd in background
- `orph core stop` — graceful shutdown via IPC
- `orph run list` — list scripts with descriptions from `~/.orph/scripts/`
- `orph run <script> [args] [--timeout <s>]` — execute script with captured output, exit code handling, timeout, safety
- `orph logs [--tail] [--level <lvl>]` — view `~/.orph/orph.log` (daemon-aware)
- `orph pet status/feed/play/rename` — pet companion (cutecore style) with time-based decay; daemon-aware
- `orph cfg list/get/set` — SQLite-backed config (daemon-aware)
- `orph telemetry` — list recent command executions
- `orph telemetry top` — most used commands
- `orph completions <bash|zsh|fish>` — generate shell completion scripts
- **`orphd`** — background daemon (Unix socket, sequential IPC, JSON protocol)
  - `sys.status`, `pet.status`, `pet.feed`, `pet.play`
  - `cfg.list`, `cfg.get`, `cfg.set`
  - `logs.read` (with `tail` and `level` filter params)
  - `ping` / `shutdown` lifecycle commands
  - clean socket removal on SIGTERM / SIGINT / shutdown command
  - prevents multiple instances
  - invalid JSON → structured error response (no crash)
  - unknown commands → structured error response (no crash)
- **CLI IPC fallback** — all state commands try daemon first, fall back to local if offline
- `--json` / `--quiet` / `--verbose` global flags
- SQLite storage: pet state, config, telemetry
- Logging system: every command + errors + pet actions written to `~/.orph/orph.log`
- Timestamp consistency: all timestamps are RFC3339 (ISO 8601) everywhere

> **Design boundary:** script execution (`orph run`) remains CLI-side only.
> The daemon handles state access and read operations. It does not execute processes.

---

## MoSCoW Prioritization

### MUST
- [x] All core commands
- [x] SQLite persistence
- [x] JSON output mode
- [x] Offline-first design

### SHOULD — PRIORITY 1 (Hardening) ✅
- [x] Timestamp consistency — RFC3339 everywhere (pet, logs, telemetry, JSON)
- [x] Logging system — `~/.orph/orph.log` auto-created, all commands + errors logged
- [x] Telemetry visibility — `orph telemetry` and `orph telemetry top`

### SHOULD — PRIORITY 2 (Stability) ✅
- [x] Runner improvements:
  - stdout/stderr captured via background threads (no deadlock)
  - structured JSON output: `{script, exit_code, timed_out, elapsed_ms, stdout, stderr}`
  - `--timeout <secs>` flag: kills process, reaps to avoid zombies, accurate elapsed
  - exit code propagated correctly (non-zero → error in human mode, `exit_code` field in JSON)
  - safety: path traversal rejected (no `/`, `\`, leading `.` in script names)
  - scripts restricted to `~/.orph/scripts/` only
  - all events logged (start, exit code, elapsed, timeout)
- [x] Pet time-based decay:
  - `last_updated` timestamp column added (with migration for existing DBs)
  - hunger += 10/hour, happiness -= 5/hour (deterministic, no randomness)
  - values clamped to 0–100
  - decay applied on every `pet status`, `pet feed`, `pet play`
  - decay delta logged to `orph.log`

### COULD — PRIORITY 3 (Enhancement) ✅
- [x] `orph sys status` — disk usage (total, used, percent; root FS preferred, graceful fallback)
- [x] `orph run list` — script descriptions extracted from first `#` comment after shebang
- [x] Shell completions — `orph completions <bash|zsh|fish>` via `clap_complete`

### COULD — PRIORITY 4 (orphd v1) ✅
- [x] `orphd` daemon — Unix socket, JSON IPC, sequential request handling, no crash on bad input
- [x] `orph core start/stop/status` — full lifecycle management
- [x] CLI IPC fallback — transparent daemon-or-local for `sys status`, `pet *`, `cfg *`, `logs`
- [x] Daemon handles: `sys.status`, `pet.*`, `cfg.*`, `logs.read`
- [x] Script execution intentionally stays CLI-side (daemon does not run processes)
- [ ] `orph update` — self-update mechanism
- [ ] Cross-compile Makefile target for ARM64 (aarch64-unknown-linux-gnu)
- [ ] Config profiles (`orph cfg --profile <name>`)

### WON'T (for now)
- GUI or web dashboard
- External APIs or cloud sync
- Plugin system
- Async runtime
- Multi-threaded daemon
- Script execution via daemon

---

## Next Steps

1. Cross-compile for ARM64 via Makefile (`aarch64-unknown-linux-gnu`) for Raspberry Pi 5 deployment
2. Config profiles (`orph cfg --profile <name>`)
3. `orph update` self-update mechanism
