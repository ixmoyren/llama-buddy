use std::{iter::Iterator, time::Duration};

/// 固定延迟时间策略
#[derive(Debug, Clone)]
pub struct FixedInterval {
    // 延迟时间
    duration: Duration,
}

impl FixedInterval {
    pub fn new(duration: Duration) -> FixedInterval {
        FixedInterval { duration }
    }

    pub fn from_millis(millis: u64) -> FixedInterval {
        FixedInterval {
            duration: Duration::from_millis(millis),
        }
    }
}

impl Iterator for FixedInterval {
    type Item = Duration;

    fn next(&mut self) -> Option<Duration> {
        Some(self.duration)
    }
}

#[cfg(test)]
mod tests {
    use crate::retry::strategy::FixedInterval;
    use std::time::Duration;

    #[test]
    fn returns_some_fixed() {
        let mut s = FixedInterval::new(Duration::from_millis(123));

        assert_eq!(s.next(), Some(Duration::from_millis(123)));
        assert_eq!(s.next(), Some(Duration::from_millis(123)));
        assert_eq!(s.next(), Some(Duration::from_millis(123)));
    }
}
