mod audio;
mod config;
mod detector;
mod mcp;
mod sensor;
mod shared;

use std::sync::Arc;
use std::time::Duration;

use clap::Parser;
use config::{Cli, Command};
use shared::{DetectorConfig, SharedState};

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    // --list-audio: print pack contents and exit (before root check since it doesn't need sensor)
    if let Some(ref _pack_name) = cli.list_audio {
        let pack_id = cli.sound_pack_id().unwrap_or_else(|e| {
            eprintln!("{e}");
            std::process::exit(1);
        });
        let pack = match pack_id {
            audio::SoundPackId::Custom => {
                if let Some(ref path) = cli.custom_path {
                    match audio::SoundPack::from_dir(path) {
                        Ok(p) => p,
                        Err(e) => {
                            eprintln!("{e}");
                            std::process::exit(1);
                        }
                    }
                } else if let Some(ref files) = cli.custom_files {
                    let file_list: Vec<String> = files
                        .split(',')
                        .map(|s| s.trim().to_string())
                        .filter(|s| !s.is_empty())
                        .collect();
                    match audio::SoundPack::from_files(&file_list) {
                        Ok(p) => p,
                        Err(e) => {
                            eprintln!("{e}");
                            std::process::exit(1);
                        }
                    }
                } else {
                    eprintln!("listing custom pack requires --custom-path or --custom-files");
                    std::process::exit(1);
                }
            }
            _ => match audio::SoundPack::builtin(pack_id) {
                Ok(p) => p,
                Err(e) => {
                    eprintln!("{e}");
                    std::process::exit(1);
                }
            },
        };
        eprintln!(
            "Sound pack '{}' ({} files, mode={:?}):",
            pack.name,
            pack.files.len(),
            pack.mode
        );
        for (i, f) in pack.list_files().iter().enumerate() {
            eprintln!("  {:>3}. {}", i + 1, f);
        }
        std::process::exit(0);
    }

    if matches!(cli.command, Some(Command::Mcp)) {
        eprintln!(
            "slap-your-laptop: starting MCP server (min_level={}, cooldown={}ms, sound={})",
            cli.min_level, cli.cooldown_ms, cli.sound
        );
    } else {
        eprintln!(
            "slap-your-laptop: starting standalone (min_level={}, cooldown={}ms, min_slap_amp={:.4}g, min_shake_amp={:.4}g, sound={}, volume_scaling={}, speed={})",
            cli.min_level,
            cli.cooldown_ms,
            cli.min_slap_amp,
            cli.min_shake_amp,
            cli.sound,
            cli.volume_scaling,
            cli.speed,
        );
    }

    // Resolve sound pack ID
    let pack_id = cli.sound_pack_id().unwrap_or_else(|e| {
        eprintln!("error: {e}");
        std::process::exit(1);
    });

    // Start audio thread (unless disabled)
    let audio_tx = if !cli.no_audio {
        let tx = crate::audio::spawn_audio_thread(
            cli.volume_scaling,
            cli.speed,
            cli.cooldown_ms,
            cli.custom_path.clone(),
            cli.custom_files_list(),
            pack_id,
        );
        eprintln!("audio: playback enabled (pack={:?})", pack_id);
        Some(tx)
    } else {
        eprintln!("audio: playback disabled (--no-audio)");
        None
    };

    // Start accelerometer sensor
    let ring = match sensor::start_sensor() {
        Ok(r) => r,
        Err(e) => {
            eprintln!("error: {e}");
            std::process::exit(1);
        }
    };

    // Wait for sensor warmup
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Create shared state
    let state = Arc::new(SharedState::new(
        DetectorConfig {
            cooldown_ms: cli.cooldown_ms,
            min_level: cli.min_level,
            min_slap_amp: cli.min_slap_amp,
            min_shake_amp: cli.min_shake_amp,
            volume_scaling: cli.volume_scaling,
            speed_ratio: cli.speed,
            sound_pack_id: pack_id,
            custom_path: cli.custom_path.clone(),
            custom_files: cli.custom_files_list(),
        },
        audio_tx.unwrap_or_else(|| std::sync::mpsc::channel().0),
    ));

    // Spawn detection loop
    let detection_state = state.clone();
    tokio::spawn(async move {
        shared::run_detection_loop(ring, &detection_state).await;
    });

    // Dispatch to mode
    match cli.command {
        Some(Command::Mcp) => {
            if let Err(e) = mcp::server::run(state).await {
                eprintln!("MCP server error: {e}");
                std::process::exit(1);
            }
        }
        _ => {
            run_standalone(state).await;
        }
    }
}

async fn run_standalone(state: Arc<SharedState>) {
    eprintln!("slap-your-laptop: listening for slaps... (ctrl+c to quit)");

    // Subscribe to events from the shared detection loop
    let mut rx = state.event_tx.subscribe();

    loop {
        match rx.recv().await {
            Ok(ts_event) => {
                let event = &ts_event.event;
                println!(
                    "{{\"senderId\":\"slap\",\"text\":\"{} #{} {}\",\"correlationId\":\"\"}}",
                    event.kind.as_str(),
                    event.severity.level(),
                    event.severity.as_str()
                );
            }
            Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                eprintln!("warning: dropped {n} events (consumer too slow)");
            }
            Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                eprintln!("error: detection loop stopped");
                std::process::exit(1);
            }
        }
    }
}
