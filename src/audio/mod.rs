//! 音频资源模块
//!
//! 使用 `include_dir!` 将内置音效编译进二进制，保持 single-binary 特性。
//! 目录结构：
//! ```text
//! audio/
//! ├── pain/   (10 个 MP3)
//! ├── sexy/   (3 个 MP3)
//! ├── halo/   (9 个 MP3)
//! └── lizard/ (1 个 MP3)
//! ```

use include_dir::{include_dir, Dir};

mod pack;
mod player;
mod tracker;

// 嵌入整个 audio/ 目录到二进制中
// $CARGO_MANIFEST_DIR 是包含 Cargo.toml 的目录
pub static AUDIO_DIR: Dir<'static> = include_dir!("$CARGO_MANIFEST_DIR/audio");

/// 音频错误类型
#[derive(Debug)]
pub enum AudioError {
    NoAudioDevice,
    DecodeError,
    PlayError,
    BuiltinNotFound(String),
    BuiltinPack(SoundPackId),
    CustomDirNotFound(String),
    CustomReadError(String),
    CustomEmpty,
    NoCustomPath,
}

impl std::fmt::Display for AudioError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoAudioDevice => write!(f, "no audio output device found"),
            Self::DecodeError => write!(f, "failed to decode MP3 data"),
            Self::PlayError => write!(f, "failed to play audio"),
            Self::BuiltinNotFound(name) => {
                write!(f, "builtin audio pack '{}' not found in binary", name)
            }
            Self::BuiltinPack(id) => write!(f, "cannot load builtin pack '{:?}' via from_dir", id),
            Self::CustomDirNotFound(path) => {
                write!(f, "custom audio directory not found: {}", path)
            }
            Self::CustomReadError(path) => write!(f, "failed to read audio file: {}", path),
            Self::CustomEmpty => write!(f, "no MP3 files found in the specified path"),
            Self::NoCustomPath => write!(f, "custom mode requires --custom-path or --custom-files"),
        }
    }
}

impl std::error::Error for AudioError {}

pub use pack::*;
pub use player::AudioPlayer;
pub use tracker::SlapTracker;

// ──────────────────────────────────────────────
//  音频线程：接收播放指令，调度解码和播放
// ──────────────────────────────────────────────

use std::borrow::Cow;
use std::sync::mpsc;

/// 发给音频线程的指令
pub enum AudioCommand {
    /// 按幅度触发播放（幅度影响音量缩放）
    Play { amplitude: f64 },
    /// 切换音效包
    SetPack(SoundPackId),
    /// 开关音量缩放
    SetVolumeScaling(bool),
    /// 调整播放速度
    SetSpeed(f64),
    /// 列出音效包文件（通过 oneshot 回复）
    ListAudio {
        pack_id: SoundPackId,
        tx: mpsc::Sender<Vec<String>>,
    },
    /// 停止线程
    Stop,
}

fn load_pack(
    id: SoundPackId,
    custom_path: &Option<String>,
    custom_files: &Option<Vec<String>>,
) -> Result<SoundPack, AudioError> {
    match id {
        SoundPackId::Custom => {
            if let Some(ref path) = custom_path {
                SoundPack::from_dir(path)
            } else if let Some(ref files) = custom_files {
                SoundPack::from_files(files)
            } else {
                Err(AudioError::NoCustomPath)
            }
        }
        _ => SoundPack::builtin(id),
    }
}

/// 启动音频播放线程
///
/// 返回 `mpsc::Sender<AudioCommand>`，调用方通过它向音频线程发送指令。
/// 如果 `no_audio` 为 true，返回 `None`（不分配线程和音频设备）。
pub fn spawn_audio_thread(
    volume_scaling: bool,
    speed_ratio: f64,
    cooldown_ms: u64,
    custom_path: Option<String>,
    custom_files: Option<Vec<String>>,
    initial_pack_id: SoundPackId,
) -> mpsc::Sender<AudioCommand> {
    let (tx, rx) = mpsc::channel();

    std::thread::spawn(move || {
        audio_loop(
            rx,
            volume_scaling,
            speed_ratio,
            cooldown_ms,
            custom_path,
            custom_files,
            initial_pack_id,
        );
    });

    tx
}

fn audio_loop(
    rx: mpsc::Receiver<AudioCommand>,
    volume_scaling: bool,
    speed_ratio: f64,
    cooldown_ms: u64,
    custom_path: Option<String>,
    custom_files: Option<Vec<String>>,
    initial_pack_id: SoundPackId,
) {
    // ── 加载初始音效包 ──
    let mut current_pack = match load_pack(initial_pack_id, &custom_path, &custom_files) {
        Ok(pack) => {
            eprintln!(
                "audio: loaded pack '{}' ({} files, mode={:?})",
                pack.name,
                pack.files.len(),
                pack.mode
            );
            Some(pack)
        }
        Err(e) => {
            eprintln!("audio: failed to load initial pack: {e}");
            None
        }
    };

    // ── 初始化追踪器（半衰期与事件冷却对齐） ──
    let cooldown = std::time::Duration::from_millis(cooldown_ms);
    let file_count = current_pack.as_ref().map(|p| p.files.len()).unwrap_or(1);
    let mut tracker = SlapTracker::new(file_count.max(1), cooldown);

    // ── 创建播放器 ──
    let mut player: Option<AudioPlayer> = match AudioPlayer::new(volume_scaling, speed_ratio) {
        Ok(p) => {
            eprintln!("audio: playback engine ready (volume_scaling={volume_scaling}, speed={speed_ratio})");
            Some(p)
        }
        Err(e) => {
            eprintln!("audio: {e}");
            None
        }
    };

    // ── 主循环 ──
    while let Ok(cmd) = rx.recv() {
        match cmd {
            AudioCommand::Play { amplitude } => {
                if let Some(pack) = &current_pack {
                    let (_, score) = tracker.record(std::time::Instant::now());

                    let idx = match pack.mode {
                        crate::audio::PlayMode::Random => tracker.random_index(pack.files.len()),
                        crate::audio::PlayMode::Escalation => {
                            tracker.escalation_index(score, pack.files.len())
                        }
                    };

                    let file = &pack.files[idx];
                    if let Some(p) = &player {
                        let data: Vec<u8> = match &file.data {
                            Cow::Borrowed(b) => b.to_vec(),
                            Cow::Owned(v) => v.clone(),
                        };
                        if let Err(e) = p.play(data, amplitude) {
                            eprintln!("audio: play error: {e}");
                        } else {
                            eprintln!(
                                "audio: ▶ [{}/{}] {} (amp={:.4}g, score={:.2})",
                                idx + 1,
                                pack.files.len(),
                                file.name,
                                amplitude,
                                score,
                            );
                        }
                    } else {
                        // --no-audio dry-run
                        eprintln!(
                            "audio: [dry-run] [{}/{}] {} (amp={:.4}g, score={:.2})",
                            idx + 1,
                            pack.files.len(),
                            file.name,
                            amplitude,
                            score,
                        );
                    }
                }
            }

            AudioCommand::SetPack(id) => match load_pack(id, &custom_path, &custom_files) {
                Ok(pack) => {
                    eprintln!(
                        "audio: switched to pack '{}' ({} files)",
                        pack.name,
                        pack.files.len()
                    );
                    current_pack = Some(pack);
                    tracker.reset();
                }
                Err(e) => eprintln!("audio: {e}"),
            },

            AudioCommand::SetVolumeScaling(vs) => {
                if let Some(p) = &mut player {
                    p.set_volume_scaling(vs);
                }
                eprintln!("audio: volume_scaling={vs}");
            }

            AudioCommand::SetSpeed(sr) => {
                if let Some(p) = &mut player {
                    p.set_speed(sr);
                }
                eprintln!("audio: speed={sr}");
            }

            AudioCommand::ListAudio { pack_id, tx } => {
                let files = match load_pack(pack_id, &custom_path, &custom_files) {
                    Ok(p) => p.list_files(),
                    Err(e) => vec![format!("error: {e}")],
                };
                let _ = tx.send(files);
            }

            AudioCommand::Stop => {
                eprintln!("audio: stopping playback engine");
                break;
            }
        }
    }
}
