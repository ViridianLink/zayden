use futures::StreamExt;

pub(crate) const CONCURRENCY: usize = 5;

pub(crate) async fn run<T, F, Fut>(rows: Vec<T>, f: F)
where
    F: Fn(T) -> Fut,
    Fut: Future<Output = ()>,
{
    futures::stream::iter(rows.into_iter().map(f))
        .buffer_unordered(CONCURRENCY)
        .for_each(|()| async {})
        .await;
}
