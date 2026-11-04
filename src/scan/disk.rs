use std::ffi::CString;
use std::mem::MaybeUninit;

#[derive(Debug, Clone, Copy)]
pub struct Disk {
    pub total: u64,
    pub available: u64,
}

impl Disk {
    pub fn root() -> Option<Disk> {
        let path = CString::new("/").ok()?;
        let mut stats = MaybeUninit::<libc::statfs>::uninit();

        if unsafe { libc::statfs(path.as_ptr(), stats.as_mut_ptr()) } != 0 {
            return None;
        }

        let stats = unsafe { stats.assume_init() };
        let block = u64::from(stats.f_bsize);

        Some(Disk {
            total: stats.f_blocks * block,
            available: stats.f_bavail * block,
        })
    }

    pub fn used(self) -> u64 {
        self.total.saturating_sub(self.available)
    }

    pub fn fraction(self) -> f32 {
        if self.total == 0 {
            0.0
        } else {
            self.used() as f32 / self.total as f32
        }
    }
}
