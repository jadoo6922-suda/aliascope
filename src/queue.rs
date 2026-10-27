use futures::stream::{FuturesUnordered, StreamExt};
use std::future::Future;
use std::sync::Arc;
use tokio::sync::Semaphore;

/// Run async tasks with a concurrency limit (simple worker queue).
pub async fn map_limited<I, T, F, Fut>(items: I, limit: usize, f: F) -> Vec<T>
where
    I: IntoIterator,
    F: Fn(I::Item) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = T> + Send + 'static,
    T: Send + 'static,
    I::Item: Send + 'static,
{
    let sem = Arc::new(Semaphore::new(limit.max(1)));
    let f = Arc::new(f);
    let mut futs = FuturesUnordered::new();

    for item in items {
        let sem = Arc::clone(&sem);
        let f = Arc::clone(&f);
        futs.push(tokio::spawn(async move {
            let _permit = sem.acquire().await.expect("semaphore");
            f(item).await
        }));
    }

    let mut out = Vec::new();
    while let Some(joined) = futs.next().await {
        if let Ok(v) = joined {
            out.push(v);
        }
    }
    out
}
