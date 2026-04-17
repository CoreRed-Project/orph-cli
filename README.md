# Orph

![Project Banner](Orph.png)

![MSRV](https://img.shields.io/badge/MSRV-1.85-blue)
![License](https://img.shields.io/badge/License-MIT-green)
[![CI](https://github.com/CoreRed-Project/orph-cli/workflows/CI/badge.svg)](https://github.com/CoreRed-Project/orph-cli/actions)

<p align="center">
  <strong>Offline-first ✦ Zero dependencies ✦ Built for constrained hardware</strong><br>
  <em>On-device Raspberry Pi Harness. One binary. No network. No noise.</em>
</p>

<p align="center">
  <a href="#about">About</a> ✦
  <a href="#features">Features</a> ✦
  <a href="#installation">Installation</a> ✦
  <a href="#usage">Usage</a> ✦
  <a href="#contributing">Contributing</a>
</p>

---

## About

**Orph** is a local harness for Raspberry Pi and other offline, resource-constrained devices. It standardizes your on-device workflows — script execution, config management, system inspection — without requiring a network, a package manager, or anything beyond the binary itself.

Fresh Pi. No network. No packages. You need to inspect the system, run scripts, persist state — without installing anything first. That's where Orph lives. Drop the binary, run it. Zero runtime dependencies means an entire class of problems just doesn't exist.

It ships as two binaries: `orph` (the CLI) and `orphd` (an optional background daemon). Everything lives in `~/.orph/`. No external calls. No cloud. No surprises.

Orph isn't competing with modern CLI toolkits. It's competing with the friction that comes with running headless, offline, or on hardware where things break quietly and nothing is guaranteed.

### Philosophy

> *"Local-first. No external services. No dependencies beyond the binary."*

The typical Pi setup is full of implicit assumptions: network available, packages installed, paths correct. Orph removes as many of those assumptions as possible. The daemon is optional — if it's not running, the CLI falls back silently to local execution. If a script fails, you know why. If telemetry runs, it stays on-device.

This is a **Core Red** project, part of the Sxnnyside Project's experimental branch.

## Features

- **Consistent script runner**: Standardize workflows across environments from `~/.orph/scripts/` — with timeout, captured output, and path safety. Same interface whether you're SSHed in, running headless, or on a fresh flash.
- **Local config store**: SQLite-backed key/value store via `orph cfg`. Persistent, inspectable, no daemon required.
- **Optional daemon (`orphd`)**: Unix socket server for persistent state across rapid invocations. CLI works fully without it, falls back gracefully.
- **System inspection**: Quick CPU, memory, disk, and host info via `orph sys` — useful when you don't have a dashboard or htop handy.
- **Shell completions**: Bash, Zsh, Fish via `orph completions`.
- **Local telemetry**: Command usage tracking stored in `~/.orph/orph.db`. Never transmitted. Opt-out in one command.
- **Virtual pet** ฅ^•ﻌ•^ฅ: A time-decaying companion that lives in your terminal — `orph pet`. It fits the vibe, stays out of your way.

## Installation

### Prerequisites

- [Rust](https://rustup.rs/) (1.85+)

### From Source

```bash
git clone https://github.com/CoreRed-Project/orph-cli.git
cd orph-cli

# Build and install both binaries
make build
make install
```

Both `orph` and `orphd` need to be on `$PATH` for `orph core start` to work. `make install` handles that.

### Cross-compilation (Raspberry Pi 5)

This is the primary target. Cross-compile on your dev machine, copy the binaries over, done.

```bash
make cross
# copy target/aarch64-unknown-linux-gnu/release/{orph,orphd} to your Pi
```

No runtime dependencies on the Pi side. Drop the binary, run it.

## Usage

```bash
# system inspection
orph sys status
orph sys info

# script runner — scripts live in ~/.orph/scripts/
# first `orph run list` creates the directory and drops a sample script
orph run list
orph run my-script --timeout 30

# add your own:
#   echo '#!/bin/sh' > ~/.orph/scripts/hello
#   echo 'echo "hello!"' >> ~/.orph/scripts/hello
#   chmod +x ~/.orph/scripts/hello
#   orph run hello

# config
orph cfg list
orph cfg get <key>
orph cfg set <key> <value>

# logs
orph logs                   # view all
orph logs -n 20             # last 20 lines
orph logs -f                # follow (like tail -f)
orph logs --level warn

# daemon lifecycle (optional — CLI works without it)
orph core start             # start orphd in background
orph core status            # check if running
orph core stop

# telemetry (local only)
orph telemetry
orph telemetry top

# pet (no subcommand defaults to status)
orph pet
orph pet feed
orph pet play
orph pet rename HAL

# shell completions (add to your shell profile)
orph completions zsh >> ~/.zshrc
```

Global flags: `--json` ✦ `--quiet` ✦ `--verbose`

### Daemon (`orphd`)

`orphd` is optional. The CLI works fully without it — all state falls back to local SQLite. The daemon is useful if you're running many `orph` commands in quick succession and want persistent state across invocations.

When offline, commands that support the daemon will note `[daemon offline — running in local fallback mode]`. Nothing breaks silently.

### Telemetry

Command usage is logged locally to `~/.orph/orph.db`. Nothing leaves the device.

```bash
orph cfg set telemetry disabled

# verify:
orph telemetry
# telemetry is disabled
```

## Contributing

Contributions are accepted. See [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

Before contributing, read the [Code of Conduct](CODE_OF_CONDUCT.md).

## License

This project is licensed under the MIT License — see the [LICENSE](LICENSE) file for details.

---

<p align="center">
  <strong>Orph</strong> — A Core Red Project<br>
  <em>&copy; 2026 Sxnnyside Project</em>
</p>