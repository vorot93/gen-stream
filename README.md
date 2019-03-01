# gen-stream

Generator-based streams for Rust and futures 0.3.

[Documentation](https://docs.rs/gen-stream)

### What is this for?
Rust ecosystem is currently moving towards asynchronous computation based on [Future](https://doc.rust-lang.org/nightly/std/future/trait.Future.html) trait and friends. One part of this is enabling us to write Futures-based code in synchronous fashion using [async/await]().

This is only for `Future`, however. How do you write a complicated asynchronous iterator ([Stream](https://docs.rs/futures-preview/*/futures/stream/trait.Stream.html), that is) without rolling enum-based state-machine? [async yield](https://github.com/rust-lang/rfcs/blob/master/text/2394-async_await.md#generators-and-streams) functions are supposed to give us a hand eventually. But until such time comes...

Just write your own generator and wrap it in one of GenStreams.

### How can I use this?
You need the latest Rust nightly, tested to be working as of `nightly-2019-03-02`.

Add this to Cargo.toml:

```toml
gen-stream = "0.2"
```

### Example

```rust
#![feature(futures_api)]
#![feature(never_type)]
#![feature(generators)]
#![feature(generator_trait)]
#![feature(gen_future)]

use futures::{
    compat::{Stream01CompatExt},
    executor::block_on,
    prelude::*,
    task::Poll,
};
use gen_stream::{gen_await, GenStreamNoReturn};
use std::{ops::Generator, pin::Pin, time::{Duration, SystemTime}};
use tokio::timer::Interval;

fn current_time() -> impl Generator<Yield = Poll<SystemTime>, Return = !> {
    move || {
        let mut i = Interval::new_interval(Duration::from_secs(2)).compat();

        loop {
            let (_, s) = gen_await!(i.into_future());
            i = s;

            yield Poll::Ready(SystemTime::now());
        }
    }
}

fn main() {
    let mut time_streamer = GenStreamNoReturn::from(current_time());

    for _ in 0..10 {
        let current_time = block_on(time_streamer.next());
        println!("Current time is {:?}", current_time);
    }
}
```

License: MIT OR Apache-2.0
