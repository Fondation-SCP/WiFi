//! Contains miscellaneous tools functions and traits that may be used anywhere in the code.

use std::ops::Deref;
use futures_util::future::{join_all, try_join_all, JoinAll, TryJoinAll};
use futures_util::{FutureExt, TryFuture};
use reqwest::header::USER_AGENT;
use std::time::Duration;
use tokio_stream::Iter;

/// Downloads a webpage into a [String]. Retries `max_retries` times using [retry_async].
pub(crate) async fn download(
    client: impl Deref<Target = reqwest::Client> + Send + Sync,
    url: impl AsRef<str> + Send + Sync,
    max_retries: usize,
) -> Result<String, reqwest::Error> {
    retry_async(max_retries, Some(Duration::from_secs(2)), || async {
        client
            .get(url.as_ref())
            .header(USER_AGENT, "WikidotForumIndex/1.0")
            .send()
            .then(async |r| match r {
                Ok(r) => r.text().await,
                Err(e) => Err(e),
            })
            .await
            .inspect_err(|e| eprintln!("Request error: {e}. Retrying in 2 seconds."))
    }).await
}

/// Trait allowing a method-like use of [tokio_stream::iter] and [join_all].
#[allow(unused)]
pub(crate) trait FutureIterator<F: Future>: Sized + Iterator<Item = F> {
    /// See [tokio_stream::iter].
    fn into_future_iter(self) -> Iter<Self> {
        tokio_stream::iter(self)
    }

    /// See [join_all].
    fn join_all(self) -> JoinAll<F> {
        join_all(self)
    }
}

/// Trait allowing a method-like use of [try_join_all].
#[allow(unused)]
pub(crate) trait TryFutureIterator<F: TryFuture>: FutureIterator<F> {
    /// See [try_join_all].
    fn try_join_all(self) -> TryJoinAll<F> {
        try_join_all(self)
    }
}

impl<I: Iterator<Item = F>, F: Future> FutureIterator<F> for I {}
impl<I: Iterator<Item = F>, F: TryFuture> TryFutureIterator<F> for I {}

/// Implements [TryIterator::partition_errors] on all [Iterator]s of [Result]s.
#[allow(unused)]
pub(crate) trait TryIterator<R, E>: Sized + Iterator<Item = Result<R, E>> {
    /// Partition the iterator of results and collects two collections, one of successes and one of errors.
    ///
    /// # Example
    ///
    /// ```
    /// let result_collection = vec![Ok(3), Err("Error"), Ok(5), Ok(10), Err("Error 2"), Ok(9)];
    /// let (successes, errors): (Vec<_>, Vec<_>) = result_collection.into_iter().partition_errors();
    /// assert_eq!(successes, vec![3, 5, 10, 9]);
    /// assert_eq!(errors, vec!["Error", "Error 2"]);
    /// ```
    fn partition_errors<C: FromIterator<R>, X: FromIterator<E>>(self) -> (C, X) {
        let (oks, errs): (Vec<_>, Vec<_>) = self.partition(|r| r.is_ok());
        (oks.into_iter().filter_map(Result::ok).collect(), errs.into_iter().filter_map(Result::err).collect())
    }
}

impl<R, E, I: Sized + Iterator<Item = Result<R, E>>> TryIterator<R, E> for I {}

/// Tries to run the closure `f`, retries if it returns [Err] until the maximum number of retries
/// is reached.
///
/// Returns the result of the closure if [Ok], else returns the last [Err].
///
/// If `sleep` is [Some], sleeps for the given duration between each try.
#[allow(unused)]
pub(crate) async fn retry_async<O, E, F: Future<Output = Result<O, E>>>(mut retries: usize, sleep: Option<Duration>, f: impl Fn() -> F) -> Result<O, E> {
    let mut res = f().await;
    while retries > 0 && res.is_err() {
        if let Some(dur) = sleep {
            tokio::time::sleep(dur).await;
        }
        retries -= 1;
        res = f().await;
    }
    res
}