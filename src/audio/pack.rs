use std::borrow::Cow;

use serde::{Deserialize, Serialize};

use crate::audio::{AudioError, AUDIO_DIR};

/// 音效包标识符
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SoundPackId {
    Pain,
    Sexy,
    Halo,
    Lizard,
    Custom,
}

impl SoundPackId {
    /// 从字符串解析音效包 ID
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "pain" => Some(Self::Pain),
            "sexy" => Some(Self::Sexy),
            "halo" => Some(Self::Halo),
            "lizard" => Some(Self::Lizard),
            "custom" => Some(Self::Custom),
            _ => None,
        }
    }

    /// 获取目录名称
    pub fn dir_name(&self) -> Option<&'static str> {
        match self {
            Self::Pain => Some("pain"),
            Self::Sexy => Some("sexy"),
            Self::Halo => Some("halo"),
            Self::Lizard => Some("lizard"),
            Self::Custom => None,
        }
    }
}

/// 播放模式
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayMode {
    /// 随机选择
    Random,
    /// 递进升级（连打越多，音效越激烈）
    Escalation,
}

/// 单个音效文件
pub struct SoundFile {
    pub name: String,
    pub data: Cow<'static, [u8]>,
}

/// 音效包定义
pub struct SoundPack {
    pub name: &'static str,
    pub mode: PlayMode,
    pub files: Vec<SoundFile>,
}

impl SoundPack {
    /// 加载内置音效包
    pub fn builtin(id: SoundPackId) -> Result<Self, AudioError> {
        let dir_name = match id.dir_name() {
            Some(d) => d,
            None => return Err(AudioError::BuiltinPack(id)),
        };

        let dir = AUDIO_DIR
            .get_dir(dir_name)
            .ok_or_else(|| AudioError::BuiltinNotFound(dir_name.to_string()))?;

        let mut files: Vec<SoundFile> = dir
            .files()
            .filter(|f| f.path().extension().is_some_and(|e| e == "mp3"))
            .map(|f| SoundFile {
                name: f.path().file_name().unwrap().to_string_lossy().into_owned(),
                data: Cow::Borrowed(f.contents()),
            })
            .collect();

        files.sort_by(|a, b| a.name.cmp(&b.name));

        let (name, mode) = match id {
            SoundPackId::Pain => ("pain", PlayMode::Random),
            SoundPackId::Sexy => ("sexy", PlayMode::Escalation),
            SoundPackId::Halo => ("halo", PlayMode::Random),
            SoundPackId::Lizard => ("lizard", PlayMode::Escalation),
            SoundPackId::Custom => unreachable!(),
        };

        Ok(Self { name, mode, files })
    }

    /// 从目录加载自定义音效包
    pub fn from_dir(path: &str) -> Result<Self, AudioError> {
        let entries =
            std::fs::read_dir(path).map_err(|_| AudioError::CustomDirNotFound(path.to_string()))?;
        let mut files = Vec::new();
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() && path.extension().is_some_and(|e| e == "mp3") {
                let name = path.file_name().unwrap().to_string_lossy().into_owned();
                let data = std::fs::read(&path)
                    .map_err(|_| AudioError::CustomReadError(path.display().to_string()))?;
                files.push(SoundFile {
                    name,
                    data: Cow::Owned(data),
                });
            }
        }
        files.sort_by(|a, b| a.name.cmp(&b.name));
        if files.is_empty() {
            return Err(AudioError::CustomEmpty);
        }
        Ok(Self {
            name: "custom",
            mode: PlayMode::Random,
            files,
        })
    }

    /// 从文件列表加载自定义音效包（--custom-files）
    pub fn from_files(paths: &[String]) -> Result<Self, AudioError> {
        let mut files = Vec::new();
        for path in paths {
            let name = std::path::Path::new(path)
                .file_name()
                .unwrap()
                .to_string_lossy()
                .into_owned();
            let data =
                std::fs::read(path).map_err(|_| AudioError::CustomReadError(path.clone()))?;
            files.push(SoundFile {
                name,
                data: Cow::Owned(data),
            });
        }
        if files.is_empty() {
            return Err(AudioError::CustomEmpty);
        }
        Ok(Self {
            name: "custom",
            mode: PlayMode::Random,
            files,
        })
    }

    /// 获取文件名列表（用于 --list-audio 显示）
    pub fn list_files(&self) -> Vec<String> {
        self.files.iter().map(|f| f.name.clone()).collect()
    }
}
