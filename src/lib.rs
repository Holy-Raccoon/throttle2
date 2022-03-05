// Copyright (c) 2022 Roman K. (@holy_raccoon:matrix.org)
// Provided under MIT or Apache 2.0 license
#![deny(warnings)]

#![deny(rustdoc::broken_intra_doc_links)]
#![deny(rustdoc::private_intra_doc_links)]
#![warn(missing_docs)]
#![deny(rustdoc::missing_crate_level_docs)]
#![deny(rustdoc::missing_doc_code_examples)]
#![deny(rustdoc::invalid_codeblock_attributes)]
#![deny(rustdoc::invalid_html_tags)]
#![deny(rustdoc::invalid_rust_codeblocks)]
#![deny(rustdoc::bare_urls)]

#![doc = include_str!("../README.md")]

// Rust standard library dependencies
use std::clone::Clone;
use std::cmp::Eq;
use std::fmt::{Debug, Formatter, Error};
use std::hash::{Hash, Hasher};
use std::sync::{Arc, RwLock};
use std::thread;
use std::time::{Duration, SystemTime};
// Test dependencies
#[cfg(test)] use std::collections::HashSet;
#[cfg(test)] use std::sync::atomic::{AtomicUsize, Ordering};

/// Throttle instance that shares its' state among clones
#[derive(Clone)]
pub struct Throttle {
    last_tick: Arc<RwLock<SystemTime>>
    , cooldown: Duration
}

impl Throttle {

    /// Immediately ticks and returns if the throttle is ready to tick, otherwise blocks the calling thread and ticks later
    pub fn tick(&self) {

        loop {
            let next_tick;

            {
                /* last_tick is a scoped write lock that protects self.last_tick from being mutated by another thread
                    while this scope exists. We need to protect self.last_tick to ensure it's not mutated between
                    checking and overwriting it, otherwise it could be mutated and tick rate would exceed the limit
                    because another thread could mutate it after we ensured this thread can tick, but before we saved
                    this thread's tick timestamp there */
                let mut last_tick = self.last_tick.write().unwrap();
                next_tick = *last_tick + self.cooldown;

                match next_tick.elapsed() {

                    Ok(_x) => {
                        *last_tick = SystemTime::now();
                        return;
                    }

                    Err(_err) => {}
                }

            }

            // last_tick scoped write lock is freed by now, safe to suspend the thread
            thread::sleep(next_tick.duration_since(SystemTime::now()).unwrap());
        }

    }

    /// Immediately ticks and returns true if the throttle is ready to tick, otherwise immediately returns false without
    /// ticking if the throttle was exhausted
    pub fn try_tick(&self) -> bool {

        loop {
            /* last_tick is a scoped write lock that protects self.last_tick from being mutated by another thread
                while this scope exists. We need to protect self.last_tick to ensure it's not mutated between checking
                and overwriting it, otherwise it could be mutated and tick rate would exceed the limit because another
                thread could mutate it after we ensured this thread can tick, but before we saved this thread's tick
                timestamp there */
            let mut last_tick = self.last_tick.write().unwrap();
            let next_tick = *last_tick + self.cooldown;

            match next_tick.elapsed() {

                Ok(_x) => {
                    *last_tick = SystemTime::now();
                    return true;
                }

                Err(_err) => {return false;}
            }
        }

    }

    /// Immediately returns true without ticking if the throttle was ready to tick before this method returned,
    ///  otherwise immediately returns false without ticking if the throttle was exhausted.
    ///
    /// This method doesn't tick and is not recommended for concurrent use - use try_tick() instead if you
    /// want the throttle to atomically validate rate constraint and immediately tick if the throttle is ready.
    ///
    /// ```
    /// # use throttle2::Throttle;
    /// # use std::time::Duration;
    /// #
    /// # let throttle = Throttle::from_cooldown(Duration::from_secs(1));
    /// if throttle.was_ready() {
    ///     // Beware, by this point another thread could exhaust the throttle after was_ready()
    ///     // returned but before the first statement in this if body was evaluated
    ///     println!("The throttle was ready to tick");
    /// } else {
    ///     println!("The throttle was exhausted and couldn't tick");
    /// }
    /// ```
    pub fn was_ready(&self) -> bool {
        return SystemTime::now() > *self.last_tick.read().unwrap() + self.cooldown;
    }

    // O(1) getters

    /// Returns last tick timestamp
    ///
    /// ```
    /// # use throttle2::Throttle;
    /// # use std::time::{Duration, SystemTime};
    /// #
    /// # let throttle = Throttle::from_cooldown(Duration::from_secs(1));
    /// if throttle.try_tick() {
    ///     println!("Throttle was ready and ticked {} ms ago",
    ///         SystemTime::now().duration_since(throttle.get_last_tick()).unwrap().as_millis());
    /// } else {
    ///     println!("Throttle was exhausted {} ms ago and couldn't tick",
    ///         SystemTime::now().duration_since(throttle.get_last_tick()).unwrap().as_millis());
    /// }
    /// ```
    pub fn get_last_tick(&self) -> SystemTime {
        (*self.last_tick.read().unwrap()).clone()
    }

    /// Returns next possible tick timestamp
    ///
    /// ```
    /// # use throttle2::Throttle;
    /// # use std::time::{Duration, SystemTime};
    /// #
    /// # let throttle = Throttle::from_cooldown(Duration::from_secs(1));
    /// if throttle.try_tick() {
    ///     println!("Throttle was ready and ticked, next tick possible in {} ms",
    ///         throttle.get_next_tick().duration_since(SystemTime::now()).unwrap().as_millis());
    /// } else {
    ///     println!("Throttle was exhausted and couldn't tick, next tick possible in {} ms",
    ///         throttle.get_next_tick().duration_since(SystemTime::now()).unwrap().as_millis());
    /// }
    /// ```
    pub fn get_next_tick(&self) -> SystemTime {
        *self.last_tick.read().unwrap() + self.cooldown
    }

    /// Returns throttle cooldown interval
    ///
    /// ```
    /// # use throttle2::Throttle;
    /// # use std::time::Duration;
    /// #
    /// # let throttle = Throttle::from_cooldown(Duration::from_secs(1));
    /// if !throttle.try_tick() {
    ///     println!("Throttle was exhausted and can tick only once every {} ms",
    ///         throttle.get_cooldown().as_millis());
    /// }
    ///
    /// ```
    pub fn get_cooldown(&self) -> &Duration {
        &self.cooldown
    }

}

impl Throttle {

    /// Constructs a new [Throttle] based on cooldown interval between ticks
    pub fn from_cooldown(cooldown: Duration) -> Self {

        Throttle {
            last_tick: Arc::new(RwLock::new(SystemTime::UNIX_EPOCH))
            , cooldown: cooldown
        }

    }

    /// Constructs a new [Throttle] based on maximum ticks count per specified period
    pub fn from_rate(ticks: usize, per_period: Duration) -> Self {

        Throttle {
            last_tick: Arc::new(RwLock::new(SystemTime::UNIX_EPOCH))
            , cooldown: per_period.div_f64(ticks as f64)
        }

    }

}

impl PartialEq for Throttle {

    // Two entities pointing to the same underlying state should be considered equivalent
    fn eq(&self, other: &Self) -> bool {
        return &*self.last_tick.read().unwrap() as *const SystemTime == &*other.last_tick.read().unwrap() as *const SystemTime;
    }

}

impl Hash for Throttle {

    fn hash<H: Hasher>(&self, state: &mut H) {
        // Hash raw pointer
        (&*self.last_tick.read().unwrap() as *const SystemTime).hash(state);
        self.cooldown.hash(state);
    }

}

impl Eq for Throttle {}

impl Debug for Throttle {

    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        f.debug_struct("Throttle")
            .field("Last tick", &self.last_tick)
            .field("Interval", &self.cooldown)
            .finish()
    }

}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn eq() {
        let throttle = Duration::new(0, 0);

        assert_eq!(throttle, throttle.clone());
    }

    #[test]
    fn hash() {
        let throttle = Throttle::from_cooldown(Duration::new(0, 0));
        let mut set = HashSet::<Throttle>::new();

        set.insert(throttle.clone());
        set.insert(throttle);

        assert_eq!(set.len(), 1);
    }

    #[test]
    fn cooldown() {
        const INTERVAL: Duration = Duration::from_millis(100);

        let throttle = Throttle::from_cooldown(INTERVAL);

        let start = SystemTime::now();
        throttle.tick(); // First tick ever, never delayed
        throttle.tick(); // Delayed by INTERVAL
        throttle.tick(); // Delayed by INTERVAL x2
        let finish = start.elapsed();

        assert_eq!(finish.unwrap().as_millis() / INTERVAL.as_millis(), 2)
    }

    #[test]
    fn rate() {
        const SECOND: Duration = Duration::from_secs(1);

        let throttle = Throttle::from_rate(10, SECOND);

        let start = SystemTime::now();
        throttle.tick(); // First tick ever, never delayed
        throttle.tick(); // Delayed by 100ms
        throttle.tick(); // Delayed by 200ms
        let finish = start.elapsed();

        assert_eq!(finish.unwrap().as_millis() / 100, 2)
    }

    #[test]
    fn concurrency() {
        const TOTAL_TIME: Duration = Duration::from_secs(30);
        const TOTAL_TICKS: usize = 300;
        const NUM_THREADS: usize = 15;
        const TICKS_PER_THREAD: usize = TOTAL_TICKS / NUM_THREADS;

        let throttle = Throttle::from_rate(TOTAL_TICKS, TOTAL_TIME);
        let tasks_count = Arc::new(AtomicUsize::new(0));

        let tasks_count_captured = tasks_count.clone();
        let task = move || {

            for _i in 0..TICKS_PER_THREAD {
                throttle.clone().tick();
            }

            tasks_count_captured.fetch_add(1, Ordering::SeqCst);
        };

        let start = SystemTime::now();

        for _i in 0..NUM_THREADS {
            thread::spawn(task.clone());
        }

        loop {
            if tasks_count.as_ref().load(Ordering::Relaxed) != NUM_THREADS {
                thread::yield_now();
            } else {
                break;
            }
        }

        let finish = start.elapsed().unwrap();

        const TOTAL_TIME_DELTA_MS: u128 = 100;

        // As the first tick occurs without delay, the last tick occurs on (TOTAL_TIME - 1 tick), not on TOTAL_TIME
        assert!(finish.as_millis() >= TOTAL_TIME.as_millis() - TOTAL_TIME_DELTA_MS);
        assert!(finish.as_millis() <= TOTAL_TIME.as_millis());
    }

    #[test]
    fn try_tick() {
        let cooldown = Duration::from_secs(1);
        let throttle = Throttle::from_cooldown(cooldown.clone());

        throttle.tick();

        if throttle.try_tick() {
            panic!("try_tick() should return false if the throttle is exhausted");
        }

        thread::sleep(cooldown);

        if !throttle.try_tick() {
            panic!("try_tick() should return true if the throttle was ready and ticked");
        }

        if throttle.try_tick() {
            panic!("try_tick() should tick and subsequent calls while it's exhausted should immediately return false");
        }

    }

}