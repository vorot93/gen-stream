# gen-stream

Generator-based streams for Rust and futures 0.3.

[Documentation](https://docs.rs/gen-stream)

### What is this for?
Rust ecosystem is currently moving towards asynchronous computation based on [Future](https://doc.rust-lang.org/nightly/std/future/trait.Future.html) trait and friends. One part of this is enabling us to write Futures-based code in synchronous fashion using [async/await]().

This is only for `Future`, however. How do you write a complicated asynchronous iterator ([Stream](https://docs.rs/futures-preview/*/futures/stream/trait.Stream.html), that is) without rolling enum-based state-machine? [async yield](https://github.com/rust-lang/rfcs/blob/master/text/2394-async_await.md#generators-and-streams) functions are supposed to give us a hand eventually. But until such time comes...

Just write your own generator and wrap it in one of GenStreams.

### How can I use this?
You need Rust nightly 2019-05-09 or later.

Add this to Cargo.toml:

```toml
gen-stream = "0.2"
```

### Example

```rust
#![feature(async_await)]
#![feature(never_type)]
#![feature(generators)]
#![feature(generator_trait)]
#![feature(gen_future)]

use {
    futures::{
        compat::*,
        prelude::*,
        task::Poll,
    },
    gen_stream::{gen_await, GenPerpetualStream},
    std::{ops::Generator, time::{Duration, SystemTime}},
    tokio::{runtime::current_thread::Runtime, timer::Interval},
};

fn current_time() -> impl Generator<Yield = Poll<SystemTime>, Return = !> {
    static move || {
        let mut i = Interval::new_interval(Duration::from_millis(500)).compat();

        loop {
            let _ = gen_await!(i.next()).unwrap().unwrap();

            yield Poll::Ready(SystemTime::now());
        }
    }
}

fn main() {
    let mut time_streamer = GenPerpetualStream::from(Box::pin(current_time()));

    let mut rt = Runtime::new().unwrap();
    rt.spawn(Compat::new(async move {
        for _ in 0..3 {
            let current_time = time_streamer.next().await;
            println!("Current time is {:?}", current_time);
        }

        Ok(())
    }.boxed()));
    rt.run();
}
```

License: MIT OR Apache-2.0
