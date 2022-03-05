![Cargo Build & Test](https://github.com/Holy-Raccoon/throttle2/actions/workflows/ci.yml/badge.svg)
![Publish crate](https://github.com/Holy-Raccoon/throttle2/actions/workflows/publish.yml/badge.svg)

A simple threadsafe throttle (rate limiter) that maintains fixed time intervals between ticks.

# Usage

```
use throttle2::Throttle;
use std::time::{Duration, SystemTime};

fn main() {
    let throttle = Throttle::from_cooldown(Duration::from_secs(10));

    println!("Hello world!");
    throttle.tick();
    println!("Hello again 10 seconds later");
    throttle.tick();
    println!("Hello again 10 more seconds later");
}
```

See [Throttle] documentation and the tests for more examples