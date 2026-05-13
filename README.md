# slap-your-laptop

> English | [中文](README.zh.md)

[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/Rust-1.75%2B-orange.svg)](https://www.rust-lang.org/)
[![Platform](https://img.shields.io/badge/Platform-Apple%20Silicon-black.svg)](https://support.apple.com/en-us/116943)

> Slap your MacBook. Your AI agent slaps back (verbally).

**slap-your-laptop** is a Rust CLI that detects physical slaps and shakes on Apple Silicon MacBooks via the built-in accelerometer and prints events to stdout.

```
you: *slaps laptop*
slap-your-laptop: {"senderId":"slap","text":"SLAP #5 CHOC_MOYEN","correlationId":""}
```

## Table of Contents

- [Why Does This Exist?](#why-does-this-exist)
- [How It Works](#how-it-works)
- [Requirements](#requirements)
- [Quick Start](#quick-start)
- [Modes](#modes)
- [Severity Levels](#severity-levels)
- [Event Types](#event-types)
- [CLI Reference](#cli-reference)
- [Event Payload](#event-payload)
- [Detection Algorithms](#detection-algorithms)
- [Architecture](#architecture)
- [Startup Sequence](#startup-sequence)
- [Anti-False-Positive Measures](#anti-false-positive-measures)
- [Tuning Tips](#tuning-tips)
- [Testing](#testing)
- [Troubleshooting](#troubleshooting)
- [Contributing](#contributing)
- [Credits](#credits)
- [License](#license)

## Why Does This Exist?

Because someone looked at the Bosch BMI286 accelerometer inside every Apple Silicon MacBook and thought: "what if my laptop could feel pain?"

This tool reads raw IMU data at 800Hz, runs it through seismology-grade detection algorithms (originally designed for earthquake detection, now repurposed for detecting workplace laptop abuse), classifies impacts into 6 severity levels from "was that a butterfly?" to "YOU MONSTER", and prints a structured event to stdout.

Your MacBook already judges you silently. Now it can do it out loud.

## How It Works

```
                    Your Hand
                        |
                        | (violence)
                        v
┌─────────────────────────────────────┐
│  Apple Silicon MacBook              │
│  ┌───────────────────────────────┐  │
│  │ Bosch BMI286 IMU              │  │
│  │ (accelerometer, ~800Hz raw)   │  │
│  └──────────────┬────────────────┘  │
└─────────────────┼───────────────────┘
                  │
                  │ IOKit HID (requires sudo, because
                  │ Apple doesn't trust you either)
                  v
    ┌─────────────────────────────┐
    │ C Shim (iokit.c)            │
    │ - wakes SPU sensor drivers  │
    │ - auto-locks to accel HID   │
    │ - decimates 800Hz → 100Hz   │
    │ - lock-free ring buffer     │
    └──────────────┬──────────────┘
                   │
                   │ Q16 fixed-point → g-force
                   v
    ┌─────────────────────────────┐
    │ Detector (pure Rust)        │
    │ ┌─────────┐ ┌────────────┐ │
    │ │ STA/LTA │ │   CUSUM    │ │
    │ │(3 scale)│ │ (drift)    │ │
    │ ├─────────┤ ├────────────┤ │
    │ │Kurtosis │ │ Peak/MAD   │ │
    │ │(impulse)│ │ (outlier)  │ │
    │ └─────────┘ └────────────┘ │
    │                             │
    │ High-pass filter removes    │
    │ gravity (your laptop is     │
    │ not falling, probably)      │
    └──────────────┬──────────────┘
                   │
                   │ Event: kind + severity + amplitude
                   v
    ┌─────────────────────────────┐
    │ Classification              │
    │                             │
    │ SLAP  = short spike (<100ms)│
    │ SHAKE = sustained (>200ms)  │
    │                             │
    │ 6 severity levels           │
    │ (see table below)           │
    └──────────────┬──────────────┘
                   │
                   │ cooldown + amplitude filter
                   v
    ┌─────────────────────────────┐
    │ stdout JSON output  │
    │ {"senderId":"slap",       │
    │  "text":"SLAP #5 CHOC"}  │
    └─────────────────────────────┘
```

## Requirements

- **Apple Silicon Mac** (M1, M2, M3, M4 — any variant)
- **Root privileges** (`sudo`) — IOKit HID accelerometer access needs it
- **Rust toolchain** — `rustup` recommended

## Quick Start

### 1. Build

```bash
git clone https://github.com/Galileo01/slap-your-laptop
cd slap-your-laptop
cargo build --release
```

### 2. Test Locally

```bash
sudo ./target/release/slap-your-laptop standalone
```

You'll see a warmup progress bar, then an arming phase. Once `detector: ready` appears — slap your laptop and watch events print to stdout.

```
warmup: [#########################] 0.0s remaining
arming: [#########################] 0.0s remaining
detector: [#########################] ready
>>> SLAP #5 [CHOC_MOYEN  amp=0.04231g] sources=["STA/LTA", "CUSUM", "PEAK"]
```

If you see nothing: slap harder. This isn't a touchscreen.

### 3. MCP Server Mode

```bash
sudo ./target/release/slap-your-laptop mcp
```

Starts a stdio MCP server. AI agents can call `slap_status`, `slap_wait_for_event`, and other tools via the standard MCP protocol to monitor slap events in real time.

## Modes

This tool runs in two modes:

| Mode | Command | Description |
|------|---------|-------------|
| **Standalone** (default) | `sudo slap-your-laptop` | Detects events and prints JSON to stdout |
| **MCP Server** | `sudo slap-your-laptop mcp` | Exposes tools over stdio for AI agent integration |

Both modes share the same sensor thread and detection loop — only the output differs.

### MCP Tools

| Tool | Description |
|------|-------------|
| `slap_status` | Detector phase, samples processed, sensor health, uptime |
| `slap_get_events` | Recent event history (filterable by limit, min_level) |
| `slap_wait_for_event` | Block until event occurs or timeout |
| `slap_get_config` | Current runtime configuration |
| `slap_set_config` | Update config at runtime (cooldown, thresholds) |

## Severity Levels

Your laptop is a drama queen. It classifies impacts into 6 levels:

| Level | Name | What Happened | Your Laptop's Mood |
|-------|------|---------------|-------------------|
| 1 | MICRO_VIB | You breathed near it | "Did something happen?" |
| 2 | VIB_LEGERE | Typing a bit too aggressively | "I felt that, you know" |
| 3 | VIBRATION | Table bump, nearby door slam | "Excuse me??" |
| 4 | MICRO_CHOC | Light slap, firm tap | "Oh no you didn't" |
| 5 | CHOC_MOYEN | Solid slap | "ASSAULT! ASSAULT!" |
| 6 | CHOC_MAJEUR | Full force, multiple algorithms screaming | "I'm calling AppleCare" |

Classification is based on how many detection algorithms agree something happened and how large the amplitude was. When all 4 detectors go off simultaneously, your laptop knows you meant it.

## Event Types

| Type | Duration | Example |
|------|----------|---------|
| **SLAP** | < 100ms STA/LTA activation | Quick smack, tap |
| **SHAKE** | > 200ms sustained oscillation | Picking up laptop angrily, desk vibration |

Events between 100-200ms are classified as UNKNOWN and silently dropped — your laptop is confused and chooses not to comment.

## CLI Reference

```
slap-your-laptop [OPTIONS] [COMMAND]
```

Commands: `standalone` (default), `mcp`

### Detection Tuning

| Flag | Env Var | Default | Description |
|------|---------|---------|-------------|
| `--cooldown <MS>` | `SLAP_COOLDOWN` | `500` | Minimum ms between published events |
| `--min-level <1-6>` | `SLAP_MIN_LEVEL` | `4` | Ignore events below this severity |
| `--min-slap-amp <G>` | `SLAP_MIN_SLAP_AMP` | `0.010` | Minimum SLAP amplitude in g-force |
| `--min-shake-amp <G>` | `SLAP_MIN_SHAKE_AMP` | `0.030` | Minimum SHAKE amplitude in g-force |

### Audio Feedback

| Flag | Env Var | Default | Description |
|------|---------|---------|-------------|
| `--sound <SOUND>` | `SLAP_SOUND` | `pain` | Sound pack: `pain`, `sexy`, `halo`, `lizard`, `custom` |
| `--volume-scaling` | `SLAP_VOLUME_SCALING` | `true` | Scale volume by impact amplitude |
| `--speed <SPEED>` | `SLAP_SPEED` | `1` | Playback speed ratio |
| `--custom-path <DIR>` | `SLAP_CUSTOM_PATH` | — | Custom audio directory (requires `--sound custom`) |
| `--custom-files <FILES>` | `SLAP_CUSTOM_FILES` | — | Comma-separated MP3 paths (requires `--sound custom`) |
| `--list-audio <PACK>` | — | — | List files in a sound pack and exit |
| `--no-audio` | `SLAP_NO_AUDIO` | — | Disable audio playback entirely |

## Event Payload

Each event is printed as a structured JSON line to stdout:

```json
{"senderId":"slap","text":"SLAP #5 CHOC_MOYEN","correlationId":""}
```

or for shakes:

```json
{"senderId":"slap","text":"SHAKE #4 MICRO_CHOC","correlationId":""}
```

## Detection Algorithms

Four algorithms run in parallel on every sample. This is genuinely overkill for "someone slapped a laptop", but we're here to have fun with signal processing.

### STA/LTA (Short-Term Average / Long-Term Average)

Borrowed from seismology. Compares recent energy to background energy across 3 timescales:

| Scale | Short Window | Long Window | Sensitivity |
|-------|-------------|-------------|-------------|
| Fast | 3 samples (30ms) | 100 samples (1s) | Catches sharp spikes |
| Medium | 15 samples (150ms) | 500 samples (5s) | Catches moderate impacts |
| Slow | 50 samples (500ms) | 2000 samples (20s) | Catches sustained disturbance |

When the ratio exceeds the ON threshold, the channel activates. How long it stays active determines SLAP vs SHAKE.

### CUSUM (Cumulative Sum)

Drift detection — accumulates deviations from the running mean. Like a grudge. Small offenses pile up until the threshold is breached.

### Kurtosis

Measures "peakedness" of the signal distribution over a 100-sample window. Normal noise has kurtosis ~3. An impulsive slap spikes it above 6. Basically: "how much does this look like someone hit something?"

### Peak/MAD (Median Absolute Deviation)

Robust outlier detection over a 200-sample window. If the current sample is more than 4 sigma from the median (MAD-estimated), something abnormal just happened.

## Architecture

```
src/
├── main.rs            # CLI + warmup/arming UX + main loop + mode dispatch + audio thread
├── config.rs          # clap derive CLI args + env vars + subcommands
├── shared.rs          # SharedState, DetectorConfig, run_detection_loop()
├── sensor/
│   ├── mod.rs         # Module exports
│   ├── iokit.rs       # Rust FFI: ring buffer reader, Q16→g conversion
│   └── iokit.c        # C shim: IOKit HID, SPU driver wake, device auto-lock
├── detector/
│   ├── mod.rs         # 4 detection algorithms + severity classifier
│   └── ring.rs        # Fixed-capacity ring buffer (RingFloat)
├── audio/
│   ├── mod.rs         # AudioError, AudioCommand, spawn_audio_thread(), type re-exports
│   ├── pack.rs        # SoundPackId, PlayMode, SoundPack (builtin + custom loading)
│   ├── player.rs      # AudioPlayer (rodio-based, non-blocking, volume scaling)
│   └── tracker.rs     # SlapTracker (Random/Escalation index selection)
└── mcp/
    ├── mod.rs         # MCP module declaration
    └── server.rs      # SlapServer: 5 MCP tools via rmcp
```

### Why a C Shim?

IOKit and CoreFoundation are C frameworks. You _can_ call them from Rust via raw FFI, but it involves 200+ lines of `extern "C"` declarations, opaque type casts, and `CFRelease` choreography. The C shim is ~240 lines, handles all the macOS framework calls, and exposes 3 functions to Rust:

```c
int iokit_sensor_init(void);    // zero the ring buffer
void iokit_sensor_run(void);    // wake sensors + run CFRunLoop (blocking)
const uint8_t* iokit_ring_ptr(void);  // shared ring buffer pointer
```

### Device Auto-Lock

Apple Silicon Macs expose 4-8 HID devices through `AppleSPUHIDDevice`. Only one of them is the accelerometer. The C shim uses a voting system to auto-detect the right device:

1. Open all vendor-page (`0xFF00`) HID devices
2. Filter reports to 22-byte IMU format
3. Validate raw L1 norm is in plausible gravity range (0.5g–4g)
4. After 3 consecutive good reports from the same device → lock
5. After 6 consecutive good reports with the same report ID → lock report

This means the same binary works across M1, M2, M3, M4 without hardcoding device indices.

## Startup Sequence

When you run the tool, you'll see:

```
iokit: woke 8 SPU drivers
iokit: device 1: UsagePage=0xff00 Usage=255
iokit: registered accel callback on idx=0 usage=255
...
iokit: locked accelerometer device idx=0 usage=255
iokit: locked accelerometer reportID=0
warmup: [################---------] 0.9s remaining
arming: [#########################] 0.0s remaining
detector: [#########################] ready
```

**Phase 1 — Warmup (2s):** The high-pass filter and running averages need time to settle. Events during warmup are suppressed.

**Phase 2 — Arming (1s):** An extra quiet period after warmup to let statistics stabilize. Still suppressed.

**Phase 3 — Ready:** The detector is live. Your laptop is now emotionally available.

## Anti-False-Positive Measures

Because nobody wants their laptop screaming "ASSAULT" while they're just typing an email:

1. **Warmup gate** — first 200 samples (2s) suppressed entirely
2. **Arming gate** — additional 100 samples (1s) of quiet settling
3. **UNKNOWN events dropped** — only SLAP and SHAKE are published
4. **Anti-typing guard** — SLAP events without a PEAK detection source AND amplitude < 0.03g are silently dropped (keyboards produce low-amplitude micro-vibrations that look like soft slaps)
5. **Amplitude floor** — separate configurable minimums for SLAP (0.01g) and SHAKE (0.03g)
6. **Severity filter** — default `--min-level 4` ignores levels 1-3 entirely
7. **Cooldown** — 500ms minimum between published events

## Tuning Tips

**Too sensitive?** (fires on typing, table bumps)

```bash
sudo ./target/release/slap-your-laptop --min-level 5 --min-slap-amp 0.025
```

**Not sensitive enough?** (need to punch the laptop to trigger it)

```bash
sudo ./target/release/slap-your-laptop --min-level 3 --min-slap-amp 0.005 --min-shake-amp 0.010
```

**Getting spammed?** (too many events in a row)

```bash
sudo ./target/release/slap-your-laptop --cooldown 3000  # 3 second cooldown
```

## Usage

```
slap-your-laptop [OPTIONS] [COMMAND]
```

Commands: `standalone` (default), `mcp`

## Testing

```bash
cargo test        # Unit tests (detector, ring buffer, config, MCP, integration paths)
cargo clippy      # Lint
cargo fmt --check # Format check
```

Tests use synthetic accelerometer data — no actual laptop violence required during CI.

## Troubleshooting

**"requires root privileges"**
→ Run with `sudo`. IOKit HID needs it. No way around this.

**"Failed to initialize IOKit HID sensor"**
→ Not Apple Silicon, or your Mac doesn't have the BMI286 IMU. Only M-series chips are supported.

**No events detected**
→ Wait for the "detector: ready" message. Slap the palmrest area firmly (not the screen, please). Check `--min-level` isn't set too high.

**Events fire on typing**
→ Raise `--min-slap-amp` (try `0.020` or `0.025`). The anti-typing guard catches most cases but heavy typists on certain MacBook models may need higher thresholds.

**Progress bar stuck**
→ Sensor thread may have failed. Check the iokit log lines above for errors. On some M4 Macs, the sensor usage page differs — the auto-lock system should handle this, but file an issue if it doesn't.

## Contributing

Contributions are welcome! This project is in early development and there's plenty to improve.

### Development Setup

```bash
git clone https://github.com/Galileo01/slap-your-laptop
cd slap-your-laptop
cargo build
```

### Running Tests

```bash
cargo test
cargo clippy
cargo fmt --check
```

### Areas for Contribution

- **Hardware testing** — Try it on different MacBook models (M1/M2/M3/M4) and report how it behaves
- **Detection tuning** — Improve false-positive filtering or propose new algorithms
- **New output modes** — Additional integrations beyond MCP
- **Documentation** — Translations, tutorials, or improved troubleshooting guides

Please open an issue before starting large changes so we can discuss the approach.

## Credits

Detection algorithms ported from:

- [taigrr/spank](https://github.com/taigrr/spank) — the OG Go implementation
- [taigrr/apple-silicon-accelerometer](https://github.com/taigrr/apple-silicon-accelerometer)

Built with:

- [clap](https://docs.rs/clap) for CLI
- [tokio](https://tokio.rs) for async runtime
- [rmcp](https://docs.rs/rmcp) for MCP server
- [cc](https://docs.rs/cc) for C shim compilation

## License

This project is licensed under the MIT License — see the [LICENSE](LICENSE) file for details.

Slap responsibly.
