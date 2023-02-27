use futures::{task::SpawnExt, Future};
use once_cell::sync::OnceCell;

pub fn run_background<Fut>(fut: Fut) -> anyhow::Result<()>
where
    Fut: Future<Output = ()> + Send + 'static,
    Fut::Output: Send + 'static,
{
    use futures::executor::ThreadPool;

    static THREAD_POOL: OnceCell<ThreadPool> = OnceCell::new();

    Ok(THREAD_POOL
        .get_or_init(|| ThreadPool::new().unwrap())
        .spawn(fut)?)
}
