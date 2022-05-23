use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

/// Defines some useful extensions for functional programming which reorder the control flow.
pub mod functional;

#[macro_export]
macro_rules! assert_symbolic_eq {
    ($x:expr, $y:expr) => {
        assert!($x.iff(&$y).is_true())
    };
}

#[macro_export]
macro_rules! assert_symbolic_ne {
    ($x:expr, $y:expr) => {
        assert!(!$x.xor(&$y).is_false())
    };
}

/* Adapter from: https://blog.yoshuawuyts.com/async-cancellation-1/ */

/// Spawn a new tokio Task that is canceled when the handle is dropped.
pub fn spawn_bound<T>(future: T) -> BoundJoinHandle<T::Output>
where
    T: Future + Send + 'static,
    T::Output: Send + 'static,
{
    BoundJoinHandle(tokio::task::spawn(future))
}

/// Cancels the wrapped tokio Task on Drop.
pub struct BoundJoinHandle<T>(tokio::task::JoinHandle<T>);

impl<T> Future for BoundJoinHandle<T> {
    type Output = Result<T, tokio::task::JoinError>;
    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        unsafe { Pin::new_unchecked(&mut self.0) }.poll(cx)
    }
}

impl<T> Drop for BoundJoinHandle<T> {
    fn drop(&mut self) {
        // do `let _ = self.0.cancel()` for `async_std::task::Task`
        self.0.abort();
    }
}
