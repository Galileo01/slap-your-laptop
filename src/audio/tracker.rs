use std::time::{Duration, Instant};

use rand::Rng;

/// 打击追踪器 —— 根据连打频率计算当前音效索引
///
/// - **Random 模式**：每次都等概率随机选一个文件
/// - **Escalation 模式**：连打越多，越靠后的文件（更激烈的声音）
///
/// Escalation 算法：
/// 1. 每次打击 score += 1.0
/// 2. 两次打击之间按指数衰减（半衰期 30 秒）
/// 3. score 通过 S 形曲线 (1 - e^(-x)) 映射到文件索引
///    使得持续最大频率打击时恰好到达最后一个文件
pub struct SlapTracker {
    score: f64,
    last_time: Option<Instant>,
    total: u64,
    half_life: f64,
    scale: f64,
    rng: rand::rngs::ThreadRng,
}

impl SlapTracker {
    /// 创建新的追踪器
    ///
    /// `file_count` — 当前音效包的文件数量
    /// `cooldown` — 事件冷却时间（保留用于未来扩展）
    pub fn new(file_count: usize, cooldown: Duration) -> Self {
        let _ = cooldown; // 保留参数，scale 由文件数量决定
        // scale: S 曲线缩放因子，约 N 次连续打击到达最后一个文件
        // 公式推导：idx = N*(1-exp(-(score-1)/scale))，令 score=N 时 idx≈N-1
        // 解得 scale = (N-1)/ln(N)
        //
        // TODO: 当前 scale 仅由文件数 N 决定，假设 score 可累积到 N。
        //       但实际 score 受半衰期衰减限制，存在上界。当 N 较大时，
        //       S 曲线尾部对应的 score 超过实际可达范围，导致最后几个文件无法到达。
        let scale = if file_count > 1 {
            let n = file_count as f64;
            (n - 1.0) / f64::ln(n)
        } else {
            1.0
        };
        Self {
            score: 0.0,
            last_time: None,
            total: 0,
            half_life: 30.0,
            scale,
            rng: rand::thread_rng(),
        }
    }

    /// 记录一次打击，返回 (总打击次数, 当前 score)
    pub fn record(&mut self, now: Instant) -> (u64, f64) {
        if let Some(last) = self.last_time {
            let elapsed = now.duration_since(last).as_secs_f64();
            self.score *= f64::powf(0.5, elapsed / self.half_life);
        }
        self.score += 1.0;
        self.last_time = Some(now);
        self.total += 1;
        (self.total, self.score)
    }

    /// Random 模式：等概率随机
    pub fn random_index(&mut self, file_count: usize) -> usize {
        self.rng.gen_range(0..file_count)
    }

    /// Escalation 模式：score 越高 → 索引越大 → 更激烈的声音
    pub fn escalation_index(&self, score: f64, file_count: usize) -> usize {
        if file_count <= 1 {
            return 0;
        }
        let max_idx = file_count - 1;
        // 1-exp(-(score-1)/scale) 产生 S 形曲线
        let idx = (file_count as f64 * (1.0 - f64::exp(-(score - 1.0) / self.scale))) as usize;
        idx.min(max_idx)
    }

    pub fn total_hits(&self) -> u64 {
        self.total
    }

    pub fn current_score(&self) -> f64 {
        self.score
    }

    /// 切换音效包时重置
    pub fn reset(&mut self) {
        self.score = 0.0;
        self.last_time = None;
        self.total = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_escalation_increases_with_rapid_hits() {
        let mut tracker = SlapTracker::new(10, Duration::from_millis(500));
        let base = Instant::now();

        // 快速连打
        let mut last_idx = 0;
        for i in 0..20 {
            let t = base + Duration::from_millis(i * 100);
            tracker.record(t);
            let idx = tracker.escalation_index(tracker.current_score(), 10);
            if i > 5 {
                // 连打后索引应该增长
                assert!(
                    idx >= last_idx,
                    "escalation should not decrease: idx={} last={}",
                    idx,
                    last_idx
                );
            }
            last_idx = idx;
        }
    }

    #[test]
    fn test_escalation_decays_over_time() {
        let mut tracker = SlapTracker::new(10, Duration::from_millis(500));
        let base = Instant::now();

        // 快速连打10次
        for i in 0..10 {
            let t = base + Duration::from_millis(i * 100);
            tracker.record(t);
        }
        let high_idx = tracker.escalation_index(tracker.current_score(), 10);

        // 等待衰减（模拟60秒静默）
        let t_after = base + Duration::from_secs(60);
        // 不记录新打击，score 会因为下次 record 时的衰减而下降
        tracker.record(t_after);
        let low_idx = tracker.escalation_index(tracker.current_score(), 10);

        // 长时间静默后索引应该降低
        assert!(low_idx <= high_idx, "index should decay over time");
    }

    #[test]
    fn test_single_file_always_returns_zero() {
        let tracker = SlapTracker::new(1, Duration::from_millis(500));
        // 只有1个文件时，无论 score 多高都返回 0
        assert_eq!(tracker.escalation_index(100.0, 1), 0);
    }

    #[test]
    fn test_random_index_in_bounds() {
        let mut tracker = SlapTracker::new(100, Duration::from_millis(500));
        let base = Instant::now();
        tracker.record(base);

        for _ in 0..100 {
            let idx = tracker.random_index(50);
            assert!(idx < 50, "random index out of bounds: {}", idx);
        }
    }
}
