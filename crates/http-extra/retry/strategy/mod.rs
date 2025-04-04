mod exponential_backoff;
mod fibonacci_backoff;
mod fixed_interval;

pub use self::{
    exponential_backoff::ExponentialBackoff, fibonacci_backoff::FibonacciBackoff,
    fixed_interval::FixedInterval,
};

use std::time::Duration;

/// 为重试时间添加一个抖动因子
pub fn jitter(duration: Duration) -> Duration {
    duration.mul_f64(rand::random::<f64>())
}

/// 为重试时间添加一个抖动因子，提供因子上限和下限
pub fn jitter_range(min: f64, max: f64) -> impl Fn(Duration) -> Duration {
    move |x| x.mul_f64(rand::random::<f64>() * (max - min) + min)
}

#[cfg(test)]
mod tests {
    use super::{jitter, jitter_range};
    use std::time::Duration;

    #[test]
    fn test_jitter() {
        let jitter = jitter(Duration::from_millis(100));
        assert!(jitter.as_millis() <= 100);
        assert_ne!(jitter.as_millis(), 100);
    }

    #[test]
    fn test_jitter_range() {
        let jitter = jitter_range(0.01, 0.1)(Duration::from_millis(100));
        assert!(jitter.as_millis() >= 1);
        assert!(jitter.as_millis() <= 10);
        assert_ne!(jitter.as_millis(), 100);

        let jitter = jitter_range(0.1, 0.2)(Duration::from_millis(100));
        assert!(jitter.as_millis() >= 10);
        assert!(jitter.as_millis() <= 20);
        assert_ne!(jitter.as_millis(), 100);

        let jitter = jitter_range(0.5, 0.6)(Duration::from_millis(100));
        assert!(jitter.as_millis() >= 50);
        assert!(jitter.as_millis() <= 60);
        assert_ne!(jitter.as_millis(), 100);
    }
}
