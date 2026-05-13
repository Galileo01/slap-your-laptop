use clap::{Parser, Subcommand};

use crate::audio::SoundPackId;

#[derive(Parser, Debug, Clone)]
#[command(name = "slap-your-laptop")]
#[command(about = "Detect laptop slaps via Apple Silicon accelerometer and play audio feedback")]
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

    /// Sound pack to play on detection: pain, sexy, halo, lizard, or custom
    #[arg(long, env = "SLAP_SOUND", default_value = "pain")]
    pub sound: String,

    /// Enable volume scaling based on impact amplitude
    #[arg(long, env = "SLAP_VOLUME_SCALING", default_value_t = true)]
    pub volume_scaling: bool,

    /// Playback speed ratio (1.0 = normal)
    #[arg(long, env = "SLAP_SPEED", default_value_t = 1.0)]
    pub speed: f64,

    /// Custom audio directory path (requires --sound custom)
    #[arg(long, env = "SLAP_CUSTOM_PATH")]
    pub custom_path: Option<String>,

    /// Custom audio file paths (comma-separated, requires --sound custom)
    #[arg(long, env = "SLAP_CUSTOM_FILES")]
    pub custom_files: Option<String>,

    /// List all audio files in a sound pack and exit
    #[arg(long)]
    pub list_audio: Option<String>,

    /// Disable audio playback entirely
    #[arg(long, env = "SLAP_NO_AUDIO")]
    pub no_audio: bool,

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

impl Cli {
    /// Resolve the sound pack ID from CLI args
    pub fn sound_pack_id(&self) -> Result<SoundPackId, String> {
        SoundPackId::from_str(&self.sound).ok_or_else(|| {
            format!(
                "unknown sound pack '{}'. valid: pain, sexy, halo, lizard, custom",
                self.sound
            )
        })
    }

    /// Resolve custom files from comma-separated string
    pub fn custom_files_list(&self) -> Option<Vec<String>> {
        self.custom_files.as_ref().map(|s| {
            s.split(',')
                .map(|p| p.trim().to_string())
                .filter(|p| !p.is_empty())
                .collect()
        })
    }
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
        assert_eq!(cli.sound, "pain");
        assert!(cli.volume_scaling);
        assert!((cli.speed - 1.0).abs() < f64::EPSILON);
        assert!(!cli.no_audio);
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

    #[test]
    fn test_sound_pack_id_parsing() {
        let cli = Cli::try_parse_from(["slap-your-laptop", "--sound", "sexy"]).unwrap();
        assert_eq!(cli.sound_pack_id().unwrap(), SoundPackId::Sexy);

        let cli = Cli::try_parse_from(["slap-your-laptop", "--sound", "halo"]).unwrap();
        assert_eq!(cli.sound_pack_id().unwrap(), SoundPackId::Halo);

        let cli = Cli::try_parse_from(["slap-your-laptop", "--sound", "lizard"]).unwrap();
        assert_eq!(cli.sound_pack_id().unwrap(), SoundPackId::Lizard);

        let cli = Cli::try_parse_from(["slap-your-laptop", "--sound", "custom"]).unwrap();
        assert_eq!(cli.sound_pack_id().unwrap(), SoundPackId::Custom);

        // clap accepts any string; validation happens in sound_pack_id()
        let cli = Cli::try_parse_from(["slap-your-laptop", "--sound", "bogus"]).unwrap();
        assert!(cli.sound_pack_id().is_err());
    }

    #[test]
    fn test_custom_files_parsing() {
        let cli = Cli::try_parse_from([
            "slap-your-laptop",
            "--sound",
            "custom",
            "--custom-files",
            "a.mp3,b.mp3,c.mp3",
        ])
        .unwrap();
        let files = cli.custom_files_list().unwrap();
        assert_eq!(files, vec!["a.mp3", "b.mp3", "c.mp3"]);
    }
}
