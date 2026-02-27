use std::time::Duration;
use futures_util::future::{join_all, try_join_all, JoinAll, TryJoinAll};
use futures_util::{FutureExt, TryFuture};
use reqwest::header::USER_AGENT;
use scraper::Html;
use tokio_stream::Iter;

pub async fn download_html(
    client: impl AsRef<reqwest::Client>,
    url: impl AsRef<str>,
    max_retries: usize,
) -> Result<Html, reqwest::Error> {
    retry_async(max_retries, Some(Duration::from_secs(2)), async || {
        client.as_ref()
            .get(url.as_ref())
            .header(USER_AGENT, "ScpScriptsAnthology/1.0")
            .send()
            .then(async |r| match r {
                Ok(r) => r.text().await,
                Err(e) => Err(e),
            })
            .await
            .inspect_err(|e| eprintln!("Request error: {e}. Retrying in 2 seconds."))
    }).await
        .map(|s| Html::parse_document(s.as_str()))
}

#[allow(unused)]
pub trait FutureIterator<F: Future>: Sized + Iterator<Item = F> {
    fn into_future_iter(self) -> Iter<Self> {
        tokio_stream::iter(self)
    }

    fn join_all(self) -> JoinAll<F> {
        join_all(self)
    }
}

#[allow(unused)]
pub trait TryFutureIterator<F: TryFuture>: FutureIterator<F> {
    fn try_join_all(self) -> TryJoinAll<F> {
        try_join_all(self)
    }
}

impl<I: Iterator<Item = F>, F: Future> FutureIterator<F> for I {}
impl<I: Iterator<Item = F>, F: TryFuture> TryFutureIterator<F> for I {}

#[allow(unused)]
pub trait TryIterator<R, E>: Sized + Iterator<Item = Result<R, E>> {
    fn stable_try_collect<C: FromIterator<R> + Default>(mut self) -> Result<C, E> {
        let error = self.find(|r| r.is_err());
        error.map(|r| r.map(|_| C::default()))
            .unwrap_or_else(|| self.collect())
    }

    fn partition_errors<C: FromIterator<R>, X: FromIterator<E>>(self) -> (C, X) {
        let (oks, errs): (Vec<_>, Vec<_>) = self.partition(|r| r.is_ok());
        (oks.into_iter().filter_map(Result::ok).collect(), errs.into_iter().filter_map(Result::err).collect())
    }
}

impl<R, E, I: Sized + Iterator<Item = Result<R, E>>> TryIterator<R, E> for I {}

#[allow(unused)]
pub(crate) async fn retry_async<O, E>(mut retries: usize, sleep: Option<Duration>, f: impl AsyncFn() -> Result<O, E>) -> Result<O, E> {
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