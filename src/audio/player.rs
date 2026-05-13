use rodio::{Decoder, OutputStream, OutputStreamHandle, Sink, Source};
use std::io::Cursor;
use std::sync::{Arc, Mutex};

use crate::audio::AudioError;

/// 音频播放引擎
///
/// 在独立的 std 线程中持有 rodio OutputStream 和 Sink。
/// 通过 channel 接收播放指令，支持非阻塞异步播放。
pub struct AudioPlayer {
    /// 保持 stream 存活（drop 后所有声音停止）
    _stream: OutputStream,
    /// 用于创建新 Sink
    handle: OutputStreamHandle,
    /// 当前活跃的 Sink（用于停止之前的播放）
    sink: Arc<Mutex<Option<Sink>>>,
    volume_scaling: bool,
    speed_ratio: f64,
}

impl AudioPlayer {
    /// 创建新的音频播放器
    pub fn new(volume_scaling: bool, speed_ratio: f64) -> Result<Self, AudioError> {
        let (stream, handle) =
            OutputStream::try_default().map_err(|_| AudioError::NoAudioDevice)?;
        Ok(Self {
            _stream: stream,
            handle,
            sink: Arc::new(Mutex::new(None)),
            volume_scaling,
            speed_ratio,
        })
    }

    /// 播放音频数据（非阻塞）
    ///
    /// 内部 spawn 一个新线程来处理解码和播放，立即返回。
    pub fn play(&self, data: Vec<u8>, amplitude: f64) -> Result<(), AudioError> {
        let handle = self.handle.clone();
        let sink_ref = self.sink.clone();
        let vs = self.volume_scaling;
        let sr = self.speed_ratio;

        std::thread::spawn(move || {
            // 停止之前的播放
            {
                let mut sink = sink_ref.lock().unwrap();
                if let Some(s) = sink.take() {
                    s.stop();
                }
            }

            // 解码 MP3
            let cursor = Cursor::new(data);
            let source = match Decoder::new(cursor) {
                Ok(s) => s,
                Err(_) => return,
            };

            // 转换为 f32 并统一转为 Box<dyn Source<Item = f32> + Send> 以便链式调用
            let source: Box<dyn Source<Item = f32> + Send> =
                Box::new(source.convert_samples::<f32>());

            // 速度调整（通过 resampling 改变播放速率）
            let source: Box<dyn Source<Item = f32> + Send> =
                if (sr - 1.0).abs() > f64::EPSILON && sr > 0.0 {
                    Box::new(source.speed(sr as f32))
                } else {
                    source
                };

            // 音量缩放（幅度越大音量越高，对数曲线）
            let source: Box<dyn Source<Item = f32> + Send> = if vs {
                Box::new(source.amplify(amplitude_to_volume(amplitude)))
            } else {
                source
            };

            // 创建新 Sink 并播放
            match Sink::try_new(&handle) {
                Ok(new_sink) => {
                    new_sink.append(source);
                    let mut sink = sink_ref.lock().unwrap();
                    *sink = Some(new_sink);
                }
                Err(_) => {
                    eprintln!("audio: failed to create sink");
                }
            }
        });

        Ok(())
    }

    /// 设置音量缩放开关
    pub fn set_volume_scaling(&mut self, enabled: bool) {
        self.volume_scaling = enabled;
    }

    /// 设置播放速度倍率
    pub fn set_speed(&mut self, ratio: f64) {
        self.speed_ratio = ratio;
    }
}

/// 幅度 → 音量映射（对数曲线）
///
/// - 0.05g  → -3.0 dB (约 1/8 音量)
/// - 0.80g+ →  0.0 dB (全音量)
/// - 中间值按对数曲线平滑过渡
fn amplitude_to_volume(amplitude: f64) -> f32 {
    const MIN_AMP: f64 = 0.05;
    const MAX_AMP: f64 = 0.80;
    const MIN_VOL_LINEAR: f32 = 0.125; // 约 -3dB

    let t = if amplitude <= MIN_AMP {
        0.0
    } else if amplitude >= MAX_AMP {
        1.0
    } else {
        (amplitude - MIN_AMP) / (MAX_AMP - MIN_AMP)
    };

    // 对数映射：让轻敲安静、重拍大声，中间有自然过渡
    let log_t = f64::ln(1.0 + t * 99.0) / f64::ln(100.0);
    let min_vol = MIN_VOL_LINEAR as f64;
    (min_vol + log_t * (1.0 - min_vol)) as f32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_amplitude_to_volume_below_min() {
        assert!((amplitude_to_volume(0.01) - 0.125).abs() < 0.01);
    }

    #[test]
    fn test_amplitude_to_volume_above_max() {
        assert!((amplitude_to_volume(2.0) - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_amplitude_to_volume_monotonic() {
        let mut prev = amplitude_to_volume(0.05);
        for amp in (10..=80).map(|a| a as f64 / 100.0) {
            let cur = amplitude_to_volume(amp);
            assert!(
                cur >= prev - 1e-6,
                "non-monotonic at amp={}: prev={}, cur={}",
                amp,
                prev,
                cur
            );
            prev = cur;
        }
    }

    #[test]
    fn test_amplitude_to_volume_mid_range() {
        let v = amplitude_to_volume(0.4);
        assert!(
            v > 0.125 && v < 1.0,
            "mid-range volume should be between min and max, got {}",
            v
        );
    }
}
