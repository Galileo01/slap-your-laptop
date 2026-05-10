use clap::{Parser, Subcommand};

#[derive(Parser, Debug, Clone)]
#[command(name = "slap-your-laptop")]
#[command(about = "Detect laptop slaps via Apple Silicon accelerometer and print events to stdout")]
#[command(version)]
pub struct Cli {
    /// Cooldown between events in milliseconds
    #[arg(long = "cooldown", env = "SLAP_COOLDOWN", default_value_t = 500)]
    pub cooldown_ms: u64,

    /// Minimum severity level to publish (1-6)
    #[arg(long, env = "SLAP_MIN_LEVEL", default_value_t = 4, value_parser = clap::value_parser!(u8).range(1..=6))]
    pub min_level: u8,

    /// Minimum SLAP amplitude (g) to publish
    #[arg(long, env = "SLAP_MIN_SLAP_AMP", default_value_t = 0.010)]
    pub min_slap_amp: f64,

    /// Minimum SHAKE amplitude (g) to publish
    #[arg(long, env = "SLAP_MIN_SHAKE_AMP", default_value_t = 0.030)]
    pub min_shake_amp: f64,

    #[command(subcommand)]
    pub command: Option<Command>,
}

#[derive(Subcommand, Debug, Clone)]
pub enum Command {
    /// Run as MCP server over stdio (for AI agent integration)
    Mcp,

    /// Run in standalone mode (default if no subcommand)
    Standalone,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_no_subcommand() {
        let cli = Cli::try_parse_from(["slap-your-laptop"]).unwrap();
        assert!(cli.command.is_none());
        assert_eq!(cli.cooldown_ms, 500);
        assert_eq!(cli.min_level, 4);
        assert!((cli.min_slap_amp - 0.010).abs() < f64::EPSILON);
        assert!((cli.min_shake_amp - 0.030).abs() < f64::EPSILON);
    }

    #[test]
    fn test_mcp_subcommand() {
        let cli = Cli::try_parse_from(["slap-your-laptop", "--min-level", "3", "mcp"]).unwrap();
        assert!(matches!(cli.command, Some(Command::Mcp)));
        assert_eq!(cli.min_level, 3);
    }

    #[test]
    fn test_standalone_subcommand() {
        let cli = Cli::try_parse_from(["slap-your-laptop", "standalone"]).unwrap();
        assert!(matches!(cli.command, Some(Command::Standalone)));
    }

    #[test]
    fn test_detector_args_with_standalone() {
        let cli = Cli::try_parse_from([
            "slap-your-laptop",
            "--cooldown",
            "1000",
            "--min-level",
            "5",
            "--min-slap-amp",
            "0.02",
            "--min-shake-amp",
            "0.05",
            "standalone",
        ])
        .unwrap();
        assert_eq!(cli.cooldown_ms, 1000);
        assert_eq!(cli.min_level, 5);
        assert!((cli.min_slap_amp - 0.02).abs() < 1e-9);
        assert!((cli.min_shake_amp - 0.05).abs() < 1e-9);
        assert!(matches!(cli.command, Some(Command::Standalone)));
    }

    #[test]
    fn test_invalid_min_level() {
        let result = Cli::try_parse_from(["slap-your-laptop", "--min-level", "7"]);
        assert!(result.is_err());
    }
}
