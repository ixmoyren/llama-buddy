use std::{iter::Iterator, time::Duration};

/// 指数回退策略，由重试次数决定指数
#[derive(Debug, Clone)]
pub struct ExponentialBackoff {
    // 当前延迟时间
    current: u64,
    // 基本持续时间
    base: u64,
    // 时间因子
    factor: u64,
    // 最大延迟时间
    max_delay: Option<Duration>,
}

impl ExponentialBackoff {
    /// 通过给定的持续时间，构建一个指数回退策略，时间单位为毫秒
    ///
    /// 所得到的延迟时间是通过 base 的 n 次方来计算的， 其中 n 表示尝试的次数
    pub fn from_millis(base: u64) -> ExponentialBackoff {
        ExponentialBackoff {
            current: base,
            base,
            factor: 1u64,
            max_delay: None,
        }
    }

    /// 用于延迟时间的乘法因子
    ///
    /// 例如，使用因子“1000”将使每次延迟以秒为单位。
    ///
    /// 默认因子为 `1`
    pub fn factor(mut self, factor: u64) -> ExponentialBackoff {
        self.factor = factor;
        self
    }

    /// 最大的延迟时间，每次重试时，等待时间不能大于这个最大的延迟时间
    pub fn max_delay(mut self, duration: Duration) -> ExponentialBackoff {
        self.max_delay = Some(duration);
        self
    }
}

impl Iterator for ExponentialBackoff {
    type Item = Duration;

    fn next(&mut self) -> Option<Duration> {
        // 应用因子
        let duration = if let Some(duration) = self.current.checked_mul(self.factor) {
            Duration::from_millis(duration)
        } else {
            Duration::from_millis(u64::MAX)
        };

        // 检查是否超过了设定的最大延迟时间
        if let Some(ref max_delay) = self.max_delay
            && duration > *max_delay
        {
            return Some(*max_delay);
        }

        let current = if let Some(next) = self.current.checked_mul(self.base) {
            next
        } else {
            u64::MAX
        };

        self.current = current;

        Some(duration)
    }
}

#[cfg(test)]
mod tests {
    use crate::retry::strategy::ExponentialBackoff;
    use std::time::Duration;

    #[test]
    fn returns_some_exponential_base_10() {
        let mut s = ExponentialBackoff::from_millis(10);

        assert_eq!(s.next(), Some(Duration::from_millis(10)));
        assert_eq!(s.next(), Some(Duration::from_millis(100)));
        assert_eq!(s.next(), Some(Duration::from_millis(1000)));
    }

    #[test]
    fn returns_some_exponential_base_2() {
        let mut s = ExponentialBackoff::from_millis(2);

        assert_eq!(s.next(), Some(Duration::from_millis(2)));
        assert_eq!(s.next(), Some(Duration::from_millis(4)));
        assert_eq!(s.next(), Some(Duration::from_millis(8)));
    }

    #[test]
    fn saturates_at_maximum_value() {
        let mut s = ExponentialBackoff::from_millis(u64::MAX - 1);

        assert_eq!(s.next(), Some(Duration::from_millis(u64::MAX - 1)));
        assert_eq!(s.next(), Some(Duration::from_millis(u64::MAX)));
        assert_eq!(s.next(), Some(Duration::from_millis(u64::MAX)));
    }

    #[test]
    fn can_use_factor_to_get_seconds() {
        let factor = 1000;
        let mut s = ExponentialBackoff::from_millis(2).factor(factor);

        assert_eq!(s.next(), Some(Duration::from_secs(2)));
        assert_eq!(s.next(), Some(Duration::from_secs(4)));
        assert_eq!(s.next(), Some(Duration::from_secs(8)));
    }

    #[test]
    fn stops_increasing_at_max_delay() {
        let mut s = ExponentialBackoff::from_millis(2).max_delay(Duration::from_millis(4));

        assert_eq!(s.next(), Some(Duration::from_millis(2)));
        assert_eq!(s.next(), Some(Duration::from_millis(4)));
        assert_eq!(s.next(), Some(Duration::from_millis(4)));
    }

    #[test]
    fn returns_max_when_max_less_than_base() {
        let mut s = ExponentialBackoff::from_millis(20).max_delay(Duration::from_millis(10));

        assert_eq!(s.next(), Some(Duration::from_millis(10)));
        assert_eq!(s.next(), Some(Duration::from_millis(10)));
    }
}
