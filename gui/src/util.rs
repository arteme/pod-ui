use std::mem;
use std::sync::{Arc, Mutex};
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

//

pub fn move_out<T,F: FnOnce() -> T>(arc_mutex: &Arc<Mutex<T>>, f: F) -> T {
    let v = &mut *arc_mutex.lock().unwrap();
    mem::replace(v, f())
}
pub fn move_in<T>(arc_mutex: &Arc<Mutex<T>>, value: T) {
    let v = &mut *arc_mutex.lock().unwrap();
    *v = value;
}
pub fn move_clone<T,F: FnOnce() -> T>(from: &mut Arc<Mutex<T>>, to: &Arc<Mutex<T>>, f: F) {
    let v = move_out(from, f);
    move_in(to, v);
    *from = to.clone();
}
