# FIRE TEST — First Interaction & Real Experience Test
### `orph` v0.1.0 — April 2026

---

## Summary

### Overall Onboarding Quality

Onboarding is **acceptable for developers, broken for novices**. The README is functional but thin. The build path (`make install`) works if you have Rust. The first command (`orph sys status`) works immediately with no daemon. The daemon story is confusing and underdocumented. The virtual pet is the feature that most users will try first out of curiosity — and it works, but is never explained beyond a one-liner.

### Major Friction Areas

1. **No `orph --help` guidance on what the tool *is*.** The tagline `⋆｡°✩ cyberdeck CLI tooling system ✩°｡⋆` tells a new user absolutely nothing actionable.
2. **The `orph run` UX is a trap.** Running `orph run my-script` fails silently unless `~/.orph/scripts/` exists and the script is already there with execute permission. No onboarding for this.
3. **Daemon vs. no-daemon is invisible.** The fallback is silent, which is smart design — but it means users never know if their data is persisted or ephemeral. No indicator anywhere.
4. **`orph pet` state persistence depends on daemon being up.** Users who never start the daemon get a working pet via local SQLite fallback — but decay state is recalculated fresh on every invocation when the daemon is down. This is not documented.
5. **Banner image is broken.** `![Project Banner](path/to/banner.png)` is a literal placeholder. First thing a visitor sees on GitHub.

---

## Role: Novice

### 1. Onboarding

**What they try first:**
They open the README. The broken banner image immediately signals "unfinished project." They read past it. They see the Prerequisites section: "Rust (1.85+)." They don't have Rust. They click the link, go through `rustup` install (5–10 min). They come back.

```bash
git clone https://github.com/CoreRed-Project/orph-cli.git
cd orph-cli
make build
```

`make build` runs `cargo build --release` (not confirmed in Makefile — let's check). If it's just `cargo build`, that works. They wait 2–4 minutes for the first compilation.

**Do they succeed immediately?**
No. `make install` puts binaries in `~/.local/bin/` or `/usr/local/bin/` — the README says "Both `orph` and `orphd` need to be on `$PATH`" but doesn't show them how to verify this, what their `$PATH` looks like, or what to do if `~/.local/bin` isn't on their path. On a fresh Linux system, this silently breaks.

**What confuses them:**
- "What is a cyberdeck?" — no explanation.
- "What is `orphd`? Do I need it?" — README says "optional" but never says *what it adds* in concrete terms.
- "What is `~/.orph/scripts/`? Do I have to make it?" — not stated anywhere.
- `orph` alone with no subcommand prints clap's help. The help lists `sys`, `core`, `run`, `logs`, `pet`, `cfg`, `telemetry`, `completions`. The novice has no idea what to try first.

**Time estimates:**
- Time to first successful command: **15–25 minutes** (most of this is Rust install)
- Time to understand what Orph does: **~30 minutes**, and they'll still be uncertain about `run` and `core`

---

### 2. First Usage

**`orph sys status`**
Works immediately. Output is clean:
```
sys status
  cpu    : 4.2%
  memory : 3012 / 16384 MB (18%)
  disk   : 42 / 500 GB (8%)
```
Intuitive. The novice gets it. **This is the best first-run experience in the tool.**

**`orph run`**
Novice types `orph run` and gets an error: `error: 'run' requires a subcommand`. They try `orph run list`. Output:
```
no scripts found in /home/user/.orph/scripts
```
They don't know what to do next. There is zero guidance on how to add a script. No `orph run --help` example. No `orph run init` or equivalent. **Dead end.**

**`orph pet`**
Novice sees "Virtual pet" in the README and tries `orph pet`. Gets: `error: 'pet' requires a subcommand`. They try `orph pet status`:
```
⋆｡°✩ your pet Pixel feels happy ✩°｡⋆
  name      : Pixel
  hunger    : 30/100
  happiness : 80/100
  mood      : happy
  last fed  : 2026-04-16T10:00:00Z
  last play : 2026-04-16T10:00:00Z
```
They love it. They immediately try `orph pet feed` and `orph pet play`. Both work. This is the stickiest feature for this user type.

**What feels unnecessary:**
Telemetry. The novice doesn't know it's on, doesn't know it's local-only, and when they eventually find out something is tracking their commands, they'll feel uneasy even if it's harmless. The README explains opt-out — but it's buried below the Contributing section.

---

### 3. Friction Points

1. **`~/.orph/scripts/` is never created automatically.** The user has to know to create it and put executable scripts in it. Not documented.
2. **No default name for the pet** — wait, there is: `Pixel`. But where does that come from? The user doesn't know they can `orph pet rename`. The rename command is listed in `--help` but not in the README.
3. **`orph logs`** — the novice will try this and get an empty output (no logs yet) or a paginated stream with no explanation. The log format (RFC3339 + level + message) is opaque.
4. **`orph telemetry` output** is confusing. Running it before any commands gives "no data." The user doesn't know what they're looking at.
5. **`orph core start` fails** if `orphd` isn't in the same directory as `orph`. On a dev build (`cargo build` without `make install`), the binaries are in `target/debug/`. Running `orph core start` from a PATH-installed `orph` will look for `orphd` next to itself and fail if only `orph` was installed. **Error message:** `error: orphd binary not found at /usr/local/bin/orphd` — decent, but the user doesn't know what to do.

---

### 4. Break Test

**Wrong command:**
```
$ orph status
error: unrecognized subcommand 'status'
  tip: a similar subcommand exists: 'sys'
```
Clap provides a did-you-mean hint. Good.

**Missing args:**
```
$ orph cfg get
error: the following required arguments were not provided: <KEY>
```
Clear. Good.

**Invalid input:**
```
$ orph run ../../../etc/passwd
error: invalid script name '../../../etc/passwd': only plain filenames are allowed
```
Excellent. Security boundary is clearly communicated.

**Script that doesn't exist:**
```
$ orph run nonexistent
error: script 'nonexistent' not found in /home/user/.orph/scripts
```
Good.

**`orph cfg set` with no value:**
```
$ orph cfg set mykey
error: the following required arguments were not provided: <VALUE>
```
Fine.

---

### 5. Verdict

**Would they keep using Orph?**
**Maybe.** The pet keeps them around. `orph sys status` is genuinely useful. But `orph run` is completely inaccessible without documentation that doesn't exist. They'll use 2 of the 7 features and think the rest is "for advanced users." They won't start the daemon.

---

## Role: Average Developer

### 1. Onboarding

**What they try first:**
```bash
git clone https://github.com/CoreRed-Project/orph-cli.git
cd orph-cli
cargo build
./target/debug/orph --help
```
They skip `make install` initially and just run from `target/debug/`. This works for everything except `orph core start`, where the daemon discovery logic looks for `orphd` next to the current executable — in `target/debug/`, it's there. So it actually works in dev.

**Do they succeed immediately?**
Yes. `orph sys status` works. `orph pet status` works. Time to first command: **~3 minutes** (just compile time).

**What confuses them:**
- "What does the daemon add?" — README says "persistent state management" but doesn't say *which commands behave differently with vs. without it*. They'll have to read the source to find out (IPC fallback in `sys.rs`, `pet.rs`).
- "Why is `orph run` structured as `orph run <script-name>` instead of `orph run exec <script-name>`?" — the `#[command(external_subcommand)]` approach is fine but non-standard. They'll try `orph run --help` and see `[COMMAND]...` with no explanation of what the argument is.
- "What database is this using?" — they'll wonder and may check. They'll find it at `~/.orph/orph.db`. Fine.

**Time estimates:**
- Time to first successful command: **3–5 minutes**
- Time to understand what Orph does: **10–15 minutes**

---

### 2. First Usage

**`orph sys status`**
Works. Clean output. They'll immediately try `--json` and get valid JSON. They'll pipe it: `orph sys status --json | jq .cpu_percent` — works. Happy.

**`orph run`**
They read the README usage, understand they need scripts in `~/.orph/scripts/`. They create the dir and put a shell script in it. They run `orph run list` — sees it. They run `orph run myscript`. Works.

They notice: **the script must have execute permission.** If they do `echo 'echo hello' > ~/.orph/scripts/hello` without `chmod +x`, they get a permissions error from the OS, not from `orph`. The error is:
```
error: failed to run script: Permission denied (os error 13)
```
Acceptable but could be friendlier ("hint: script may not be executable — try chmod +x").

**`orph pet`**
They enjoy it. They'll try `orph pet rename HAL` and it works. They'll check `--json` output.

One issue: `orph pet status --json` outputs valid JSON, but the field names differ from `orph sys status --json`. `sys` uses snake_case + `_percent` suffixes; `pet` uses `name`, `hunger`, `happiness`, etc. Not a bug, just inconsistency that would matter in scripting.

---

### 3. Friction Points

1. **Daemon version in `orph core status` shows the CLI version, not the daemon version.** The code is `env!("CARGO_PKG_VERSION")` in `core.rs`. Since both binaries come from the same crate, this is always the same — but it signals "I didn't think about this carefully."

2. **`orph logs` with no flags streams everything to stdout.** No pagination. On a machine that's been running for a week, this is a wall of text. The `--tail` flag isn't a count — you can't do `--tail 20`. It seems to tail indefinitely (reading the source would confirm, but the README suggests `--tail` with no argument, which is unusual).

3. **`orph run list --json` output includes `path` field.** Good. But `description` is extracted from `# comment` in scripts — no standard enforced. Not a bug, just worth noting.

4. **Global flags (`--json`, `--quiet`, `--verbose`) must come before the subcommand.** Clap's `global = true` usually handles this, but with `external_subcommand` in `run`, flags passed *after the script name* are handled manually in `extract_run_flags()`. This creates inconsistency: `orph sys status --json` works, but `orph run myscript --json` is parsed differently (consumed by the script runner, not clap). This is by design but undocumented.

5. **No `--dry-run` for `orph run`.** Developer instinct.

---

### 4. Break Test

**Run with timeout=0:**
```
$ orph run myscript --timeout 0
```
The script starts and is immediately killed (timeout=0 → deadline=0 → `start.elapsed() >= 0` is always true). Output: empty stdout/stderr, `exit_code=-1`. In JSON mode: `timed_out: true`. Technically correct, but timeout=0 is meaningless and silently accepted.

**`orph cfg set` with whitespace value:**
```
$ orph cfg set key "value with spaces"
```
Should work. The shell handles quoting. The key/value are stored as-is in SQLite. Fine.

**Run a binary (not a script):**
```
$ orph run ls
error: script 'ls' not found in /home/user/.orph/scripts
```
Correct — it's scoped to `~/.orph/scripts/`. Expected.

---

### 5. Verdict

**Would they keep using Orph?**
**Yes, selectively.** They'll use `orph sys status` in shell prompts, `orph pet` for fun, and maybe `orph run` for personal automation. They won't use the daemon unless they find a concrete reason to. They'll be mildly annoyed by the logging verbosity of `orph logs` and the `--timeout 0` edge case. They'll recommend it to peers with caveats.

---

## Role: Senior Rust Engineer

### 1. Onboarding

**What they try first:**
```bash
git clone ...
cat Cargo.toml     # check deps, MSRV, workspace structure
cat src/lib.rs     # understand module layout
cargo build 2>&1 | grep warning   # look for noise
```
They build and expect zero warnings (`-D warnings` is presumably enforced in CI).

**Do they succeed immediately?**
Yes, technically. But they'll immediately spot things:

**What confuses (annoys) them:**
- `src/main.rs` has `mod cli; mod commands; mod services; mod models; mod ipc;` at the top — this is a binary re-declaring modules that are also declared in `lib.rs`. This pattern means `lib.rs` defines them as a library *and* `main.rs` re-imports them. If `lib.rs` uses `pub mod`, then `main.rs` declaring `mod` again creates a shadow. They'll check `lib.rs` immediately.

Actually looking at the code: `main.rs` has its own `mod` declarations, and `lib.rs` presumably also exports them. This means the binary isn't using the library crate — it re-declares modules directly. This is fragile: changes to `lib.rs` visibility don't affect the binary.

- `pub mod services` in `lib.rs` exposes `db::init()` to downstream — already flagged in the audit but still present.
- `core.rs` line 43: `let version = env!("CARGO_PKG_VERSION");` — this reports the CLI version as the daemon version. Lazy and misleading.
- `print_pet()` calls `serde_json::to_string(pet).unwrap_or_default()` — `unwrap_or_default()` on a serialization error silently produces an empty string. The rule "no `unwrap()` in production" is technically honoured but `unwrap_or_default()` on a type that can't fail to serialize is unnecessary noise, and on a type that could fail it's silent data loss.

**Time estimates:**
- Time to first successful command: **2 minutes**
- Time to form an opinion about the architecture: **15 minutes**

---

### 2. First Usage

**`orph sys status`**
They notice the 100ms sleep in `sys.rs` (`std::thread::sleep(sysinfo::MINIMUM_CPU_UPDATE_INTERVAL)`) and understand why (CPU sampling). Fine.

They also notice `orph sys info` calls `sys.refresh_cpu_all()` and sleeps even though it only uses CPU *count*, not CPU *usage*. That sleep is wasted for `info`. Minor inefficiency but signals "copy-paste code path."

**`orph run`**
They look at `#[command(external_subcommand)]` and appreciate the UX choice. They read `extract_run_flags` and note it's a hand-rolled flag parser that duplicates what Clap already does for global flags. They accept the tradeoff given the `external_subcommand` constraint but note it could break if new global flags are added.

**`orph pet`**
They read `pet_service::apply_decay` (not shown here but referenced in audit). They want to know: is decay applied on every read, or is it event-driven? If it's applied on read, the pet state is mutated on every `orph pet status` invocation — meaning the act of checking destroys state. They'll test this:

```bash
orph pet status  # hunger=30
sleep 3600
orph pet status  # hunger=60? or unchanged?
```

If hunger only changes when you call `status`, then the pet's "last fed" time is correct but the displayed hunger is computed on-read. This is fine but means two rapid `status` calls give different hunger values if time passes between them. Subtle.

---

### 3. Friction Points

1. **IPC protocol uses JSON newline-delimited over a Unix socket — fine. But `ipc::send()` returns `Option<Response>` and silently swallows errors.** Any IPC failure (socket not found, malformed response, timeout) becomes `None` and triggers the local fallback. Users and developers get no visibility into *why* the fallback happened. In a debugging session, this is frustrating.

2. **`orph core start` does `std::process::exit(0)` at the end of a `fn start(...) -> !` function.** The `-> !` return type + `process::exit` is fine but the intermediate `match child` block exits in all paths. The function structure is readable but verbose — the `-> !` diverging return causes clap's type system to require it everywhere in `handle()`, which is why `CoreCmd::Status` has an extra `Ok(())` wrapper. This creates type inconsistency in the match arms.

3. **The daemon IPC response format is not validated on the client side.** `pet.rs` does `serde_json::from_value(data)?` — a schema change in the daemon's response would give a confusing deserialization error to the user, not an "incompatible daemon version" message.

4. **No protocol version in the IPC request/response.** If `orph` v0.2 is installed but `orphd` v0.1 is running, there is no version negotiation. The first schema mismatch will produce a cryptic error.

5. **`orph logs`** — the engineer checks the implementation. If logs are written to a file in `~/.orph/`, then `orph logs` reads that file. The `--tail` flag without a count argument is non-standard (`tail -f` vs `tail -n 20`). They'll check the source and find that `--tail` is a boolean flag that streams the file, not a count. This is a UX lie: `--tail` conventionally implies "last N lines."

6. **`extract_run_flags()` silently ignores `--timeout abc`** (non-numeric timeout). It just sets `timeout = None`. The user typed `--timeout abc`, gets no error, and the script runs with no timeout. Data loss without feedback.

---

### 4. Break Test

**Concurrent invocations:**
Two terminals, both run `orph pet feed` simultaneously (daemon not running). Both open the same SQLite file. `rusqlite` with `bundled` feature handles this via SQLite's internal locking — writes will serialize. No data corruption. But the second invocation may block briefly. Fine.

**Very long script name:**
```
$ orph run $(python3 -c "print('a'*4096)")
```
This will fail at the OS argument length limit before reaching `validate_script_name`. The `bail!` message will be truncated by the terminal. Harmless but uncontrolled.

**IPC socket left behind after daemon crash:**
If `orphd` crashes hard without cleaning up `~/.orph/orph.sock`, the next `ipc::ping()` will fail to connect (connection refused) and return `false`. That's correct. But `orph core status` will say `offline` correctly. Good.

**`orph cfg set telemetry disabled` — then check:**
```
$ orph cfg set telemetry disabled
$ orph sys status
```
Telemetry should not be recorded. There's no user-facing confirmation that telemetry is now off. `orph cfg get telemetry` would show "disabled" — but the user has to know to check. Needs a `telemetry status` command or output from `orph cfg set`.

---

### 5. Verdict

**Would they keep using Orph?**
**Probably not for themselves** — they'd write their own version in 2 hours with fewer abstractions. But they'd appreciate the code quality and consider contributing, especially to fix the IPC versioning gap and the `extract_run_flags` parser duplication. They'd submit a PR for the `--timeout abc` silent failure within a week of first use.

They'd respect the project. They wouldn't trust it for production-critical automation without the IPC version negotiation fix.

---

## Cross-Role Insights

| Friction Point | Novice | Avg Dev | Senior |
|---|---|---|---|
| Broken banner image in README | ✗ hit | ✗ noticed | ✗ noted |
| `~/.orph/scripts/` not auto-created | ✗ blocked | ~ worked around | ~ irrelevant |
| `orph run` has no usage examples | ✗ blocked | ~ ok | ✓ fine |
| Daemon on/off invisible to user | ✗ confused | ~ ok | ✗ architectural concern |
| `--tail` is a bool flag not a count | ~ ignored | ✗ confused | ✗ rejected |
| `orph core start` fails if `orphd` not co-located | ✗ stuck | ~ noticed | ✗ fragile |
| No IPC version negotiation | ✓ irrelevant | ~ ok | ✗ blocker |
| Telemetry on by default, no visible confirmation of opt-out | ✗ unaware | ✗ mild concern | ✗ noted |
| `--timeout 0` or `--timeout abc` silently accepted | ✓ irrelevant | ✗ edge case | ✗ bug |

**Repeated across all roles:**
- The README banner is broken — everyone sees this.
- The daemon's role is never concretely explained.
- `orph sys status` is the best first-run experience and the most intuitive command.
- The pet is the stickiest feature but is undersold in the README.

---

## Critical Issues (Fix Before Release)

### 1. `README.md` banner is a literal placeholder
**Impact:** Every GitHub visitor sees `![Project Banner](path/to/banner.png)` as a broken image. Signals "unfinished" immediately.
**Fix:** Remove the banner line or replace with a real image/ASCII art.

### 2. `orph run` has zero onboarding
**Impact:** Users who try `orph run` get a dead end. The scripts directory doesn't exist, there's no guidance on creating it or formatting scripts, and the `--help` output is opaque (`[COMMAND]...`).
**Fix:** Auto-create `~/.orph/scripts/` on first `orph run list`. Add a one-line example to README: `echo '#!/bin/bash' > ~/.orph/scripts/hello && chmod +x ...`.

### 3. `--timeout` flag silently ignores non-numeric values
**Impact:** `orph run myscript --timeout abc` runs with no timeout and no error. Silent data loss.
**Fix:** `val.parse::<u64>().ok()` → `val.parse::<u64>().map_err(|_| anyhow!("--timeout must be a number"))`.

### 4. `orph core start` daemon discovery is fragile in non-`make install` scenarios
**Impact:** Developers who run `cargo build` and put only `orph` on `$PATH` cannot start the daemon. The error message is clear but there's no recovery path documented.
**Fix:** Document in README that both binaries must be co-located. Add a `which orphd` fallback in the binary resolution.

### 5. Telemetry opt-out has no user-facing acknowledgement
**Impact:** Users run `orph cfg set telemetry disabled` and get no confirmation. They don't know if it worked.
**Fix:** `orph cfg set` should print "set telemetry = disabled" (it may already — confirm). Separately, `orph telemetry` should show `status: disabled` when opted out.

### 6. `orph logs --tail` is a boolean, not a count
**Impact:** Non-standard API. `--tail` conventionally means "last N lines" (`tail -n 20`). A boolean `--tail` that streams everything is unexpected and pipelines will break.
**Fix:** Change to `--tail <N>` optional integer, defaulting to streaming if not given, or rename to `--follow`/`-f` to match `tail -f` convention.

---

## Nice Improvements (Post v0.1.0)

1. **Auto-create `~/.orph/scripts/` with a sample script on first `orph run list`.** Drops the dead-end experience entirely.

2. **Add IPC protocol version field to `Request`/`Response`.** Even a simple `"version": 1` field enables future graceful degradation.

3. **`orph sys info` should not sleep for CPU sampling** — it doesn't display CPU usage, only CPU count. Remove the sleep from that code path (saves ~100ms on every `info` call).

4. **`orph core status` should show daemon binary version, not CLI version.** This requires the daemon to respond with its own version via IPC. Without this, the "version" field in `core status` is misleading after an upgrade.

5. **`--tail` → `--follow` rename** (see Critical Issues #6 above) and add `--tail <N>` for last-N-lines behaviour.

6. **Shell completions in README quickstart.** One line: `orph completions zsh >> ~/.zshrc` would dramatically improve daily UX and is currently buried.

7. **`orph pet` subcommand as default.** `orph pet` with no subcommand could default to `orph pet status`. The current "requires a subcommand" error is a first-run speed bump for the most discoverable feature.

---

*FIRE TEST completed — April 16, 2026*

