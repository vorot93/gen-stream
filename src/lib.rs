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
//! You need the latest Rust nightly, tested to be working as of `nightly-2019-03-02`.
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
//! #![feature(futures_api)]
//! #![feature(never_type)]
//! #![feature(generators)]
//! #![feature(generator_trait)]
//! #![feature(gen_future)]
//!
//! use futures::{
//!     compat::{Stream01CompatExt},
//!     executor::block_on,
//!     prelude::*,
//!     task::Poll,
//! };
//! use gen_stream::{gen_await, GenStreamNoReturn};
//! use std::{ops::Generator, pin::Pin, time::{Duration, SystemTime}};
//! use tokio::timer::Interval;
//!
//! fn current_time() -> impl Generator<Yield = Poll<SystemTime>, Return = !> {
//!     move || {
//!         let mut i = Interval::new_interval(Duration::from_secs(2)).compat();
//!
//!         loop {
//!             let (_, s) = gen_await!(i.into_future());
//!             i = s;
//!
//!             yield Poll::Ready(SystemTime::now());
//!         }
//!     }
//! }
//!
//! fn main() {
//!     let mut time_streamer = GenStreamNoReturn::from(current_time());
//!
//!     for _ in 0..10 {
//!         let current_time = block_on(time_streamer.next());
//!         println!("Current time is {:?}", current_time);
//!     }
//! }
//! ```

#![feature(futures_api)]
#![feature(generator_trait)]
#![feature(gen_future)]
#![feature(never_type)]

use core::{
    ops::{Generator, GeneratorState},
    pin::Pin,
    task::{Poll, Waker},
};
use std::future::set_task_waker;

use futures::prelude::*;
use pin_utils::unsafe_pinned;

/// Like await!() but for bare generators.
#[macro_export]
macro_rules! gen_await {
    ($e:expr) => {{
        let mut pinned = $e;
        loop {
            if let Poll::Ready(x) =
                std::future::poll_with_tls_waker(unsafe { Pin::new_unchecked(&mut pinned) })
            {
                break x;
            }
            yield Poll::Pending;
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

    fn poll_next(self: Pin<&mut Self>, waker: &Waker) -> Poll<Option<Self::Item>> {
        set_task_waker(waker, || match self.inner().resume() {
            GeneratorState::Yielded(v) => v.map(Some),
            GeneratorState::Complete(_) => Poll::Ready(None),
        })
    }
}

impl<G: Unpin> Unpin for GenStream<G> {}

/// Stream based on generator that never returns.
pub struct GenStreamNoReturn<G> {
    inner: G,
}

impl<G> GenStreamNoReturn<G> {
    unsafe_pinned!(inner: G);
}

impl<G> From<G> for GenStreamNoReturn<G> {
    fn from(inner: G) -> Self {
        Self { inner }
    }
}

impl<G, Y> Stream for GenStreamNoReturn<G>
where
    G: Generator<Yield = Poll<Y>, Return = !>,
{
    type Item = Y;

    fn poll_next(self: Pin<&mut Self>, waker: &Waker) -> Poll<Option<Self::Item>> {
        set_task_waker(waker, || match self.inner().resume() {
            GeneratorState::Yielded(v) => v.map(Some),
            GeneratorState::Complete(_) => unreachable!(),
        })
    }
}

impl<G: Unpin> Unpin for GenStreamNoReturn<G> {}

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
    G: Generator<Yield = Poll<Result<T, E>>, Return = Result<(), E>>,
{
    type Item = Result<T, E>;

    fn poll_next(mut self: Pin<&mut Self>, waker: &Waker) -> Poll<Option<Self::Item>> {
        if self.finished {
            return Poll::Ready(None);
        }

        set_task_waker(waker, || match self.as_mut().inner().resume() {
            GeneratorState::Yielded(v) => v.map(Some),
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
