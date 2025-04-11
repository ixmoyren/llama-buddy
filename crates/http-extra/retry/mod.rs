use std::{fmt::Debug, iter::Iterator, time::Duration};

pub mod strategy;

pub async fn spawn<T, E: Debug>(
    strategy: impl IntoIterator<Item = Duration>,
    action: impl AsyncFn() -> Result<T, E>,
) -> Result<T, E> {
    let mut strategy = strategy.into_iter();
    loop {
        match action().await {
            Ok(t) => return Ok(t),
            Err(err) => {
                if let Some(duration) = strategy.next() {
                    tokio::time::sleep(duration).await;
                    tracing::warn!("Future execution failed, starting retry! Error: {err:?}");
                } else {
                    tracing::warn!(
                        "Future execution failed, the maximum number of retries was reached! Error: {err:?}"
                    );
                    return Err(err);
                }
            }
        }
    }
}

pub async fn spawn_if<T, E: Clone + Debug>(
    strategy: impl IntoIterator<Item = Duration>,
    action: impl AsyncFn() -> Result<T, E>,
    condition: impl Fn(E) -> bool,
) -> Result<T, E> {
    let mut strategy = strategy.into_iter();
    loop {
        match action().await {
            Ok(t) => return Ok(t),
            Err(err) => {
                if let Some(duration) = strategy.next()
                    && condition(err.clone())
                {
                    tracing::warn!("Future execution failed, starting retry! Error: {err:?}");
                    tokio::time::sleep(duration).await;
                } else {
                    tracing::warn!(
                        "Future execution failed, the maximum number of retries was reached! Error: {err:?}"
                    );
                    return Err(err);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{
        future,
        sync::{
            atomic::{AtomicUsize, Ordering},
            Arc,
        },
    };

    #[tokio::test]
    async fn attempts_just_once() {
        use std::iter::empty;
        let counter = Arc::new(AtomicUsize::new(0));
        let cloned_counter = counter.clone();
        let future = super::spawn(empty(), async move || {
            cloned_counter.fetch_add(1, Ordering::SeqCst);
            Err::<(), u64>(42)
        });
        let res = future.await;

        assert_eq!(res, Err(42));
        assert_eq!(counter.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn attempts_just_once_with_async_fn() {
        use std::iter::empty;
        let counter = Arc::new(AtomicUsize::new(0));
        let cloned_counter = counter.clone();
        let future = super::spawn(empty(), async move || {
            cloned_counter.fetch_add(1, Ordering::SeqCst);
            future::ready(Err::<(), u64>(42)).await
        });
        let res = future.await;

        assert_eq!(res, Err(42));
        assert_eq!(counter.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn attempts_until_max_retries_exceeded() {
        use super::strategy::FixedInterval;
        let s = FixedInterval::from_millis(100).take(2);
        let counter = Arc::new(AtomicUsize::new(0));
        let cloned_counter = counter.clone();
        let future = super::spawn(s, async move || {
            cloned_counter.fetch_add(1, Ordering::SeqCst);
            future::ready(Err::<(), u64>(42)).await
        });
        let res = future.await;

        assert_eq!(res, Err(42));
        assert_eq!(counter.load(Ordering::SeqCst), 3);
    }

    #[tokio::test]
    async fn attempts_until_success() {
        use super::strategy::FixedInterval;
        let s = FixedInterval::from_millis(100);
        let counter = Arc::new(AtomicUsize::new(0));
        let cloned_counter = counter.clone();
        let future = super::spawn(s, async move || {
            let previous = cloned_counter.fetch_add(1, Ordering::SeqCst);
            if previous < 3 {
                future::ready(Err::<(), u64>(42)).await
            } else {
                future::ready(Ok::<(), u64>(())).await
            }
        });
        let res = future.await;

        assert_eq!(res, Ok(()));
        assert_eq!(counter.load(Ordering::SeqCst), 4);
    }

    #[tokio::test]
    async fn compatible_with_tokio_core() {
        use super::strategy::FixedInterval;
        let s = FixedInterval::from_millis(100);
        let counter = Arc::new(AtomicUsize::new(0));
        let cloned_counter = counter.clone();
        let future = super::spawn(s, move || {
            let previous = cloned_counter.fetch_add(1, Ordering::SeqCst);
            if previous < 3 {
                future::ready(Err::<(), u64>(42))
            } else {
                future::ready(Ok::<(), u64>(()))
            }
        });
        let res = future.await;

        assert_eq!(res, Ok(()));
        assert_eq!(counter.load(Ordering::SeqCst), 4);
    }

    #[tokio::test]
    async fn attempts_retry_only_if_given_condition_is_true() {
        use super::strategy::FixedInterval;
        let s = FixedInterval::from_millis(100).take(5);
        let counter = Arc::new(AtomicUsize::new(0));
        let cloned_counter = counter.clone();
        let future = super::spawn_if(
            s,
            async move || {
                let previous = cloned_counter.fetch_add(1, Ordering::SeqCst);
                future::ready(Err::<(), usize>(previous + 1)).await
            },
            |e: usize| e < 3,
        );
        let res = future.await;

        assert_eq!(res, Err(3));
        assert_eq!(counter.load(Ordering::SeqCst), 3);
    }
}
