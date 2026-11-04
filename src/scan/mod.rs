pub mod apps;
pub mod junk;
pub mod large_files;
pub mod progress;

mod bulk;

use std::fs;
use std::ops::Add;
use std::os::unix::fs::MetadataExt;
use std::path::Path;

use rayon::prelude::*;

use bulk::Kind;
use progress::Progress;

#[derive(Debug, Clone, Copy, Default)]
pub struct Usage {
    pub bytes: u64,
    pub files: u64,
}

impl Add for Usage {
    type Output = Usage;

    fn add(self, other: Usage) -> Usage {
        Usage {
            bytes: self.bytes + other.bytes,
            files: self.files + other.files,
        }
    }
}

pub fn disk_usage(path: &Path, progress: &Progress) -> Usage {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.is_dir() => walk(path, progress),
        Ok(metadata) if metadata.is_file() => {
            let usage = Usage {
                bytes: metadata.blocks() * 512,
                files: 1,
            };
            progress.add(usage);
            usage
        }
        _ => Usage::default(),
    }
}

fn walk(dir: &Path, progress: &Progress) -> Usage {
    let mut usage = Usage::default();
    let mut subdirs = Vec::new();

    let _ = bulk::read_dir(dir, |entry| match entry.kind {
        Kind::Dir => subdirs.push(dir.join(entry.name)),
        Kind::File => {
            usage.bytes += entry.allocated;
            usage.files += 1;
        }
        Kind::Other => {}
    });

    progress.add(usage);

    usage
        + subdirs
            .par_iter()
            .map(|path| walk(path, progress))
            .reduce(Usage::default, Add::add)
}
