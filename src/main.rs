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

    // Check root privileges
    if unsafe { libc::geteuid() } != 0 {
        eprintln!("error: slap-your-laptop requires root privileges for accelerometer access");
        eprintln!("run with: sudo slap-your-laptop");
        std::process::exit(1);
    }

    if matches!(cli.command, Some(Command::Mcp)) {
        eprintln!(
            "slap-your-laptop: starting MCP server (min_level={}, cooldown={}ms)",
            cli.min_level, cli.cooldown_ms
        );
    } else {
        eprintln!(
            "slap-your-laptop: starting standalone (min_level={}, cooldown={}ms, min_slap_amp={:.4}g, min_shake_amp={:.4}g)",
            cli.min_level,
            cli.cooldown_ms,
            cli.min_slap_amp,
            cli.min_shake_amp
        );
    }

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
    let state = Arc::new(SharedState::new(DetectorConfig {
        cooldown_ms: cli.cooldown_ms,
        min_level: cli.min_level,
        min_slap_amp: cli.min_slap_amp,
        min_shake_amp: cli.min_shake_amp,
    }));

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
