use std::task::Poll;

use crate::RaiiThread;

impl<'data, Output: Send + 'static> std::future::Future for RaiiThread<'data, Output> {
    type Output = std::thread::Result<Output>;

    fn poll(
        mut self: std::pin::Pin<&mut Self>,
        _cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        if self.is_finished() {
            Poll::Ready(self.join())
        } else {
            Poll::Pending
        }
    }
}
