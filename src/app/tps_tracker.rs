use std::time::{Duration, Instant};

pub struct TpsTracker {
    counter: usize,
    resets_at: Instant,
    tps: usize,
}

impl TpsTracker {
    pub fn new() -> Self {
        Self {
            counter: 0,
            tps: 0,
            resets_at: Instant::now() + Duration::from_secs(1),
        }
    }

    pub fn tick(&mut self) {
        let now = Instant::now();
        if self.resets_at < now {
            self.resets_at = now + Duration::from_secs(1);
            self.tps = self.counter;
            self.counter = 0;
        } else {
            self.counter += 1;
        }
    }

    pub fn get(&self) -> usize {
        self.tps
    }
}
