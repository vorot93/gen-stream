//! Generator-based streams for Rust and futures 0.3.
//!
//! ## What is this for?
//! Rust ecosystem is currently moving towards asynchronous computation based on [Future](https://doc.rust-lang.org/nightly/std/future/trait.Future.html) trait and friends. One part of this is enabling us to write Futures-based code in synchronous fashion using [async/await]().
//!
//! This is only for `Future`, however. How do you write a complicated asynchronous iterator ([Stream](https://docs.rs/futures-preview/*/futures/stream/trait.Stream.html), that is) without rolling enum-based state-machine? [async yield](https://github.com/rust-lang/rfcs/blob/master/text/2394-async_await.md#generators-and-streams) functions are supposed to give us a hand eventually. But until such time comes...
//!
//! Just write your own generator and wrap it in one of GenStreams.
//!
//! ## How can I use this?
//! You need Rust nightly 2019-05-09 or later.
//!
//! Add this to Cargo.toml:
//!
//! ```toml
//! gen-stream = "0.2"
//! ```
//!
//! ## Example
//!
//! ```rust
//! #![feature(async_await)]
//! #![feature(never_type)]
//! #![feature(generators)]
//! #![feature(generator_trait)]
//! #![feature(gen_future)]
//!
//! use {
//!     futures::{
//!         compat::*,
//!         prelude::*,
//!         task::Poll,
//!     },
//!     gen_stream::{gen_await, GenPerpetualStream},
//!     std::{ops::Generator, time::{Duration, SystemTime}},
//!     tokio::{runtime::current_thread::Runtime, timer::Interval},
//! };
//!
//! fn current_time() -> impl Generator<Yield = Poll<SystemTime>, Return = !> {
//!     static move || {
//!         let mut i = Interval::new_interval(Duration::from_millis(500)).compat();
//!
//!         loop {
//!             let _ = gen_await!(i.next()).unwrap().unwrap();
//!
//!             yield Poll::Ready(SystemTime::now());
//!         }
//!     }
//! }
//!
//! fn main() {
//!     let mut time_streamer = GenPerpetualStream::from(Box::pin(current_time()));
//!
//!     let mut rt = Runtime::new().unwrap();
//!     rt.spawn(Compat::new(async move {
//!         for _ in 0..3 {
//!             let current_time = time_streamer.next().await;
//!             println!("Current time is {:?}", current_time);
//!         }
//!
//!         Ok(())
//!     }.boxed()));
//!     rt.run();
//! }
//! ```

#![feature(generator_trait)]
#![feature(gen_future)]
#![feature(never_type)]

use {
    core::{
        ops::{Generator, GeneratorState},
        pin::Pin,
        task::{Context, Poll},
    },
    futures_core::*,
    pin_utils::unsafe_pinned,
    std::future::set_task_context,
};

/// Like await!() but for bare generators.
#[macro_export]
macro_rules! gen_await {
    ($e:expr) => {{
        let mut pinned = $e;
        loop {
            if let ::core::task::Poll::Ready(x) = std::future::poll_with_tls_context(unsafe {
                ::core::pin::Pin::new_unchecked(&mut pinned)
            }) {
                break x;
            }
            yield ::core::task::Poll::Pending;
        }
    }};
}

/// Simple generator-based stream.
pub struct GenStream<G> {
    inner: G,
}

impl<G> GenStream<G> {
    unsafe_pinned!(inner: G);
}

impl<G> From<G> for GenStream<G> {
    fn from(inner: G) -> Self {
        Self { inner }
    }
}

impl<G, Y> Stream for GenStream<G>
where
    G: Generator<Yield = Poll<Y>, Return = ()>,
{
    type Item = Y;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        set_task_context(cx, || match self.inner().resume() {
            GeneratorState::Yielded(v) => v.map(Some),
            GeneratorState::Complete(_) => Poll::Ready(None),
        })
    }
}

impl<G: Unpin> Unpin for GenStream<G> {}

/// Stream based on generator that never ends.
pub struct GenPerpetualStream<G> {
    inner: G,
}

impl<G> GenPerpetualStream<G> {
    unsafe_pinned!(inner: G);
}

impl<G> From<G> for GenPerpetualStream<G> {
    fn from(inner: G) -> Self {
        Self { inner }
    }
}

impl<G, Y> Stream for GenPerpetualStream<G>
where
    G: Generator<Yield = Poll<Y>, Return = !>,
{
    type Item = Y;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        set_task_context(cx, || match self.inner().resume() {
            GeneratorState::Yielded(v) => v.map(Some),
            GeneratorState::Complete(_) => unreachable!(),
        })
    }
}

impl<G: Unpin> Unpin for GenPerpetualStream<G> {}

/// Stream based on generator that may fail.
pub struct GenTryStream<G> {
    inner: G,
    finished: bool,
}

impl<G> GenTryStream<G> {
    unsafe_pinned!(inner: G);
    unsafe_pinned!(finished: bool);
}

impl<G> From<G> for GenTryStream<G> {
    fn from(inner: G) -> Self {
        Self {
            inner,
            finished: false,
        }
    }
}

impl<G, T, E> Stream for GenTryStream<G>
where
    G: Generator<Yield = Poll<T>, Return = Result<(), E>>,
{
    type Item = Result<T, E>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        if self.finished {
            return Poll::Ready(None);
        }

        set_task_context(cx, || match self.as_mut().inner().resume() {
            GeneratorState::Yielded(v) => v.map(Ok).map(Some),
            GeneratorState::Complete(res) => {
                self.as_mut().finished().set(true);

                if let Err(e) = res {
                    Poll::Ready(Some(Err(e)))
                } else {
                    Poll::Ready(None)
                }
            }
        })
    }
}

impl<G: Unpin> Unpin for GenTryStream<G> {}
