use futures_util::FutureExt;
use tokio::task::JoinHandle;

/// Based on https://github.com/smol-rs/async-task/issues/1#issuecomment-626395280
/// and FutureExt

pub trait ManualPoll {
    type Output;

    fn poll(&mut self) -> Option<Self::Output>;
}

impl <T> ManualPoll for JoinHandle<anyhow::Result<T>> {
    type Output = anyhow::Result<T>;

    fn poll(&mut self) -> Option<Self::Output> {
        match self.now_or_never() {
            None => { None }
            Some(v) => {
                match v {
                    Ok(v) => { Some(v) }
                    Err(err) => { Some(Err(err.into())) }
                }
            }
        }
    }
}

///

pub trait ToSome {
    type Inner;
    fn some(self) -> Option<Self::Inner>;
}

impl <T> ToSome for T {
    type Inner = T;

    fn some(self) -> Option<Self::Inner> {
        Some(self)
    }
}
