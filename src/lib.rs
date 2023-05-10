use std::{marker::PhantomData, thread::JoinHandle};

use std::io;
use std::thread;

#[cfg(feature = "future")]
mod future;

// TODO :: use trait alias once #![feature(trait_alias)] is stable
// pub trait ThreadFn<Output> = FnOnce() -> Output + Send;

/// Trait alias for functions that can be passed to [RaiiThreads](RaiiThread).
pub trait ThreadFn<Output>: FnOnce() -> Output + Send {}
impl<Output, T: FnOnce() -> Output + Send> ThreadFn<Output> for T {}

/// Builder for [RaiiThreads](RaiiThread).
#[derive(Default, Debug)]
pub struct RaiiThreadBuilder {
    pub name: Option<String>,
}

impl RaiiThreadBuilder {
    /// Set the thread's name.
    pub fn name(mut self, name: String) -> Self {
        self.name.replace(name);
        self
    }

    /// Spawn the thread.
    pub fn spawn<'data, Output: Send + 'static>(
        self,
        f: impl ThreadFn<Output> + 'data,
    ) -> io::Result<RaiiThread<'data, Output>> {
        // we know that f outlives the thread -- because the thread joins when self is dropped,
        // and `f: 'data` -- so we know that it's safe to pass f to the thread, which is what we're
        // promising with the transmute
        let f_static: Box<dyn ThreadFn<Output> + 'static> =
            unsafe { std::mem::transmute::<Box<dyn ThreadFn<Output> + 'data>, _>(Box::new(f)) };

        Ok(RaiiThread {
            handle: Some({
                let mut builder = std::thread::Builder::new();
                if let Some(n) = self.name {
                    builder = builder.name(n);
                }
                builder.spawn(f_static)?
            }),
            _lifetime: Default::default(),
        })
    }
}

/// Thread wrapper that allows passing objects by reference into the thread closure.
#[derive(Debug)]
pub struct RaiiThread<'data, Output> {
    handle: Option<JoinHandle<Output>>,
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
    /// Begin building a new [RaiiThread].
    #[inline]
    pub fn builder() -> RaiiThreadBuilder {
        RaiiThreadBuilder::default()
    }

    /// Spawn a new thread using the provided function.
    #[inline]
    pub fn spawn(f: impl ThreadFn<Output> + 'data) -> io::Result<Self> {
        Self::builder().spawn(f)
    }

    /// Whether the thread has finished running.
    #[inline]
    pub fn is_finished(&self) -> bool {
        self.handle
            .as_ref()
            .map(JoinHandle::is_finished)
            .unwrap_or(true)
    }

    /// Join the thread and return its output.
    #[inline]
    pub fn join(&mut self) -> thread::Result<Output> {
        self.handle.take().unwrap().join()
    }

    /// Return the raw [JoinHandle] and `done` flag associated with this thread.
    ///
    /// # Safety
    ///
    /// Any resources borrowed by the function passed to this thread must outlive it.
    #[inline]
    pub unsafe fn leak(mut self) -> JoinHandle<Output> {
        self.handle.take().unwrap()
    }
}
