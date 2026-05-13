# slap-your-laptop

## What This Is

A Rust CLI that detects physical slaps/impacts on Apple Silicon MacBooks via the built-in accelerometer (Bosch BMI286 IMU accessed via IOKit HID). Runs in two modes: standalone (prints JSON events to stdout) or MCP server (exposes tools over stdio for AI agent integration).

## Architecture

```
slap-your-laptop           # standalone mode (default, backwards-compatible)
slap-your-laptop mcp       # MCP server mode (stdio)
```

Both modes share the same sensor thread + detection loop:

```
Sensor Thread (OS)           Tokio Runtime
─────────────────           ─────────────────────────────
IOKit HID → ring buf   →   Detection Loop Task
                                ↓ updates
                            Arc<SharedState>
                                ↓ broadcast
                        ┌───────┴──────────┐
                   Standalone          MCP Server
                   (stdout JSON)      (rmcp stdio)
```

```
src/
├── main.rs          # CLI (clap) + mode dispatch
├── config.rs        # Cli struct + Command subcommands
├── shared.rs        # SharedState, DetectorConfig, run_detection_loop()
├── sensor/
│   ├── mod.rs       # Sensor module: start_sensor() → SensorRing
│   ├── iokit.rs     # Rust FFI bindings to C shim
│   └── iokit.c      # C shim: IOKit HID accelerometer via AppleSPUHIDDriver
├── detector/
│   ├── mod.rs       # Vibration detector: 4 algorithms + event classifier
│   └── ring.rs      # Fixed-capacity ring buffer (RingFloat)
└── mcp/
    ├── mod.rs       # MCP module declaration
    └── server.rs    # SlapServer: 5 MCP tools via rmcp
(moved to repo root as README section)
```

## Audio Feedback

When a slap or shake is detected, the system can play an MP3 sound as feedback:

### 4 Built-in Sound Packs

| Pack | Mode | Files | Description |
|------|------|-------|-------------|
| **pain** | Random | 10 | Reactions to being slapped (Ow, Ouch, Yowch, etc.) |
| **sexy** | Escalation | 3 | Playful sounds, escalating with rapid hits |
| **halo** | Random | 9 | Halo-themed weapon sounds |
| **lizard** | Escalation | 1 | Lizard sound (Escalation mode, more files TBD) |

### Play Modes

- **Random**: Each event picks a random file from the pack
- **Escalation**: Consecutive rapid hits select progressively more intense files (exponential decay score → S-curve mapping)

### Custom Sound Packs

Load your own MP3 files via `--sound custom` with either:
- `--custom-path <dir>` — directory of MP3 files
- `--custom-files <a.mp3,b.mp3,...>` — comma-separated file list

## CLI Options

```
slap-your-laptop [OPTIONS] [COMMAND]

Commands:
  mcp         Run as MCP server over stdio (for AI agent integration)
  standalone  Run in standalone mode (default if no subcommand)

Options:
      --cooldown <COOLDOWN_MS>         Cooldown between events in milliseconds [env: SLAP_COOLDOWN=] [default: 500]
      --min-level <MIN_LEVEL>          Minimum severity level to publish (1-6) [env: SLAP_MIN_LEVEL=] [default: 4]
      --min-slap-amp <MIN_SLAP_AMP>    Minimum SLAP amplitude (g) to publish [env: SLAP_MIN_SLAP_AMP=] [default: 0.01]
      --min-shake-amp <MIN_SHAKE_AMP>  Minimum SHAKE amplitude (g) to publish [env: SLAP_MIN_SHAKE_AMP=] [default: 0.03]
      --sound <SOUND>                  Sound pack: pain, sexy, halo, lizard, custom [env: SLAP_SOUND=] [default: pain]
      --volume-scaling                 Enable volume scaling based on impact amplitude [env: SLAP_VOLUME_SCALING=] [default: true]
      --speed <SPEED>                  Playback speed ratio, 1.0 = normal [env: SLAP_SPEED=] [default: 1]
      --custom-path <CUSTOM_PATH>      Custom audio directory path (requires --sound custom) [env: SLAP_CUSTOM_PATH=]
      --custom-files <CUSTOM_FILES>    Custom audio file paths, comma-separated (requires --sound custom) [env: SLAP_CUSTOM_FILES=]
      --list-audio <LIST_AUDIO>        List all audio files in a sound pack and exit
      --no-audio                       Disable audio playback entirely [env: SLAP_NO_AUDIO=]
  -h, --help                           Print help
  -V, --version                        Print version
```

### Examples

```bash
# Default: pain pack with volume scaling
sudo ./target/release/slap-your-laptop

# Sexy pack, faster playback
sudo ./target/release/slap-your-laptop --sound sexy --speed 1.2

# List available sounds in the halo pack
./target/release/slap-your-laptop --list-audio halo

# Custom sound pack from a directory
sudo ./target/release/slap-your-laptop --sound custom --custom-path /path/to/mp3s/

# Custom sound pack from specific files
sudo ./target/release/slap-your-laptop --sound custom --custom-files hit1.mp3,hit2.mp3,hit3.mp3

# Disable audio (back to original silent behavior)
sudo ./target/release/slap-your-laptop --no-audio

# More sensitive (lower min level)
sudo ./target/release/slap-your-laptop --min-level 2
```

## MCP Tools

| Tool | Description |
|------|-------------|
| `slap_status` | Detector phase, samples processed, sensor health, uptime |
| `slap_get_events` | Recent event history (filterable by limit, min_level) |
| `slap_wait_for_event` | Block until event occurs or timeout |
| `slap_get_config` | Current runtime configuration |
| `slap_set_config` | Update config at runtime (cooldown, thresholds) |

## Key Constants

- Sample rate: 100Hz (decimated from ~800Hz raw)
- IMU: Bosch BMI286, report length 22 bytes, data offset 6
- Scale: Q16 fixed-point → g-force (divide by 65536.0)
- Cooldown: 500ms between events
- 6 severity levels: MICRO_VIB → CHOC_MAJEUR

## How to Build & Run

```bash
cargo build --release

# Standalone mode (default) — prints JSON events to stdout
sudo ./target/release/slap-your-laptop
sudo ./target/release/slap-your-laptop --min-level 3         # more sensitive

# MCP server mode
sudo ./target/release/slap-your-laptop mcp
```

## How to Test

```bash
cargo test           # Unit tests (detector, ring buffer, config)
cargo clippy         # Lint
cargo fmt --check    # Format check
```

## Conventions

- C shim handles all macOS framework calls (IOKit, CoreFoundation)
- Rust FFI in sensor/iokit.rs wraps C functions
- Detector is pure Rust, no unsafe, fully unit-testable with synthetic data
- MCP server uses rmcp crate with derive macros (same pattern as miniflux/mcp)
- Shared detector args (cooldown, min_level, amplitudes) on top-level Cli
