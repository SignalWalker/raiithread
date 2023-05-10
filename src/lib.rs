use std::{
    marker::PhantomData,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread::JoinHandle,
};

use std::io;
use std::thread;

#[cfg(feature = "future")]
mod future;

// TODO :: use trait alias once #![feature(trait_alias)] is stable
// pub trait ThreadFn<Output> = FnOnce() -> Output + Send;

pub trait ThreadFn<Output>: FnOnce() -> Output + Send {}
impl<Output, T: FnOnce() -> Output + Send> ThreadFn<Output> for T {}

#[derive(Default, Debug)]
pub struct RaiiThreadBuilder {
    name: Option<String>,
}

impl RaiiThreadBuilder {
    pub fn name(mut self, name: String) -> Self {
        self.name.replace(name);
        self
    }

    pub fn spawn<'data, Output: Send + 'static>(
        self,
        f: impl ThreadFn<Output> + 'data,
    ) -> io::Result<RaiiThread<'data, Output>> {
        // we know that f outlives the thread -- because the thread joins when self is dropped,
        // and `f: 'data` -- so we know that it's safe to pass f to the thread, which is what we're
        // promising with the transmute
        let f_static: Box<dyn ThreadFn<Output> + 'static> =
            unsafe { std::mem::transmute::<Box<dyn ThreadFn<Output> + 'data>, _>(Box::new(f)) };

        let done_flag = Arc::new(AtomicBool::new(false));
        let done_flag_ext = done_flag.clone();

        Ok(RaiiThread {
            handle: Some({
                let mut builder = std::thread::Builder::new();
                if let Some(n) = self.name {
                    builder = builder.name(n);
                }
                builder.spawn(move || {
                    let res = f_static();
                    done_flag_ext.store(true, Ordering::Relaxed);
                    res
                })?
            }),
            done_flag,
            _lifetime: Default::default(),
        })
    }
}

#[derive(Debug)]
pub struct RaiiThread<'data, Output> {
    handle: Option<JoinHandle<Output>>,
    done_flag: Arc<AtomicBool>,
    _lifetime: PhantomData<&'data ()>,
}

impl<'data, Output> Drop for RaiiThread<'data, Output> {
    fn drop(&mut self) {
        if let Some(h) = self.handle.take() {
            h.join().unwrap();
        }
    }
}

impl<'data, Output: Send + 'static> RaiiThread<'data, Output> {
    #[inline]
    pub fn builder() -> RaiiThreadBuilder {
        RaiiThreadBuilder::default()
    }

    #[inline]
    pub fn spawn(f: impl ThreadFn<Output> + 'data) -> io::Result<Self> {
        Self::builder().spawn(f)
    }

    pub fn is_done(&self) -> bool {
        self.done_flag.load(Ordering::Relaxed)
    }

    pub fn join(&mut self) -> thread::Result<Output> {
        self.handle.take().unwrap().join()
    }

    pub unsafe fn leak(mut self) -> JoinHandle<Output> {
        self.handle.take().unwrap()
    }
}
