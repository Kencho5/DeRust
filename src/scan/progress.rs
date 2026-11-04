use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

use crate::scan::Usage;

#[derive(Debug)]
pub struct Progress {
    started: Instant,
    files: AtomicU64,
    bytes: AtomicU64,
    done: AtomicU64,
    total: AtomicU64,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct Snapshot {
    pub files: u64,
    pub bytes: u64,
    pub done: u64,
    pub total: u64,
    pub elapsed: Duration,
}

impl Default for Progress {
    fn default() -> Self {
        Self {
            started: Instant::now(),
            files: AtomicU64::new(0),
            bytes: AtomicU64::new(0),
            done: AtomicU64::new(0),
            total: AtomicU64::new(0),
        }
    }
}

impl Progress {
    pub fn add(&self, usage: Usage) {
        if usage.files > 0 {
            self.files.fetch_add(usage.files, Ordering::Relaxed);
            self.bytes.fetch_add(usage.bytes, Ordering::Relaxed);
        }
    }

    pub fn set_total(&self, total: u64) {
        self.total.store(total, Ordering::Relaxed);
    }

    pub fn advance(&self) {
        self.advance_by(1);
    }

    pub fn advance_by(&self, amount: u64) {
        self.done.fetch_add(amount, Ordering::Relaxed);
    }

    pub fn snapshot(&self) -> Snapshot {
        Snapshot {
            files: self.files.load(Ordering::Relaxed),
            bytes: self.bytes.load(Ordering::Relaxed),
            done: self.done.load(Ordering::Relaxed),
            total: self.total.load(Ordering::Relaxed),
            elapsed: self.started.elapsed(),
        }
    }
}

impl Snapshot {
    pub fn fraction(self) -> Option<f32> {
        (self.total > 0).then(|| (self.done as f32 / self.total as f32).min(1.0))
    }

    pub fn files_per_second(self) -> u64 {
        let seconds = self.elapsed.as_secs_f64();
        if seconds > 0.0 {
            (self.files as f64 / seconds) as u64
        } else {
            0
        }
    }

    pub fn bytes_per_second(self) -> u64 {
        let seconds = self.elapsed.as_secs_f64();
        if seconds > 0.0 {
            (self.bytes as f64 / seconds) as u64
        } else {
            0
        }
    }
}
