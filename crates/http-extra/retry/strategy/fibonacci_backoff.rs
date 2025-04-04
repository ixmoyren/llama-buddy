use std::{iter::Iterator, time::Duration};

/// 斐波那契回退策略，每次重试等待的延迟时间，都是前两次的延迟时间的和
///
/// 在一些特定情况下，这个策略的性能要比指数回退策略要好
///
/// 详情请看论文 ["A Performance Comparison of Different Backoff Algorithms under Different Rebroadcast Probabilities for MANETs."](https://www.researchgate.net/profile/Saher-Manaseer/publication/255672213_A_Performance_Comparison_of_Different_Backoff_Algorithms_under_Different_Rebroadcast_Probabilities_for_MANET's/links/542d40220cf29bbc126d2378/A-Performance-Comparison-of-Different-Backoff-Algorithms-under-Different-Rebroadcast-Probabilities-for-MANETs.pdf)
#[derive(Debug, Clone)]
pub struct FibonacciBackoff {
    // 当前延迟时间
    current: u64,
    // 下一次延迟时间
    next: u64,
    // 时间因子
    factor: u64,
    // 最大延迟时间
    max_delay: Option<Duration>,
}

impl FibonacciBackoff {
    /// 通过给定基本持续时间，构造了一个斐波那契回退策略，时间单位为毫秒
    pub fn from_millis(millis: u64) -> FibonacciBackoff {
        FibonacciBackoff {
            current: millis,
            next: millis,
            factor: 1u64,
            max_delay: None,
        }
    }

    /// 用于延迟时间的乘法因子
    ///
    /// 例如，使用因子“1000”将使每次延迟以秒为单位。
    ///
    /// 默认因子为 `1`
    pub fn factor(mut self, factor: u64) -> FibonacciBackoff {
        self.factor = factor;
        self
    }

    /// 最大的延迟时间，每次重试时，等待时间不能大于这个最大的延迟时间
    pub fn max_delay(mut self, duration: Duration) -> FibonacciBackoff {
        self.max_delay = Some(duration);
        self
    }
}

impl Iterator for FibonacciBackoff {
    type Item = Duration;

    fn next(&mut self) -> Option<Duration> {
        // 应用因子
        let duration = if let Some(duration) = self.current.checked_mul(self.factor) {
            Duration::from_millis(duration)
        } else {
            Duration::from_millis(u64::MAX)
        };

        // 检查是否超过了设置的最大延迟时间
        if let Some(ref max_delay) = self.max_delay
            && duration > *max_delay
        {
            return Some(*max_delay);
        }

        let (current, next) = if let Some(next_next) = self.current.checked_add(self.next) {
            (self.next, next_next)
        } else {
            (self.next, u64::MAX)
        };
        self.current = current;
        self.next = next;

        Some(duration)
    }
}

#[cfg(test)]
mod tests {
    use crate::retry::strategy::FibonacciBackoff;
    use std::time::Duration;

    #[test]
    fn returns_the_fibonacci_series_starting_at_10() {
        let mut iter = FibonacciBackoff::from_millis(10);
        assert_eq!(iter.next(), Some(Duration::from_millis(10)));
        assert_eq!(iter.next(), Some(Duration::from_millis(10)));
        assert_eq!(iter.next(), Some(Duration::from_millis(20)));
        assert_eq!(iter.next(), Some(Duration::from_millis(30)));
        assert_eq!(iter.next(), Some(Duration::from_millis(50)));
        assert_eq!(iter.next(), Some(Duration::from_millis(80)));
    }

    #[test]
    fn saturates_at_maximum_value() {
        let mut iter = FibonacciBackoff::from_millis(u64::MAX);
        assert_eq!(iter.next(), Some(Duration::from_millis(u64::MAX)));
        assert_eq!(iter.next(), Some(Duration::from_millis(u64::MAX)));
    }

    #[test]
    fn stops_increasing_at_max_delay() {
        let mut iter = FibonacciBackoff::from_millis(10).max_delay(Duration::from_millis(50));
        assert_eq!(iter.next(), Some(Duration::from_millis(10)));
        assert_eq!(iter.next(), Some(Duration::from_millis(10)));
        assert_eq!(iter.next(), Some(Duration::from_millis(20)));
        assert_eq!(iter.next(), Some(Duration::from_millis(30)));
        assert_eq!(iter.next(), Some(Duration::from_millis(50)));
        assert_eq!(iter.next(), Some(Duration::from_millis(50)));
    }

    #[test]
    fn returns_max_when_max_less_than_base() {
        let mut iter = FibonacciBackoff::from_millis(20).max_delay(Duration::from_millis(10));

        assert_eq!(iter.next(), Some(Duration::from_millis(10)));
        assert_eq!(iter.next(), Some(Duration::from_millis(10)));
    }

    #[test]
    fn can_use_factor_to_get_seconds() {
        let factor = 1000;
        let mut s = FibonacciBackoff::from_millis(1).factor(factor);

        assert_eq!(s.next(), Some(Duration::from_secs(1)));
        assert_eq!(s.next(), Some(Duration::from_secs(1)));
        assert_eq!(s.next(), Some(Duration::from_secs(2)));
    }
}
