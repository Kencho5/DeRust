use std::sync::atomic::{AtomicU64, Ordering};

use crate::scan::Usage;

#[derive(Debug, Default)]
pub struct Progress {
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
        self.done.fetch_add(1, Ordering::Relaxed);
    }

    pub fn snapshot(&self) -> Snapshot {
        Snapshot {
            files: self.files.load(Ordering::Relaxed),
            bytes: self.bytes.load(Ordering::Relaxed),
            done: self.done.load(Ordering::Relaxed),
            total: self.total.load(Ordering::Relaxed),
        }
    }
}

impl Snapshot {
    pub fn fraction(self) -> Option<f32> {
        (self.total > 0).then(|| self.done as f32 / self.total as f32)
    }
}
