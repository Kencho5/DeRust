pub mod apps;
pub mod disk;
pub mod explorer;
pub mod junk;
pub mod progress;

mod bulk;

use std::fs;
use std::ops::Add;
use std::os::unix::fs::MetadataExt;
use std::path::Path;

use rayon::prelude::*;

use bulk::{Dir, Kind, Names};
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
        Ok(metadata) if metadata.is_dir() => Dir::open(path)
            .map(|dir| walk(&dir, progress))
            .unwrap_or_default(),
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

fn walk(dir: &Dir, progress: &Progress) -> Usage {
    let mut usage = Usage::default();
    let mut subdirs = Names::default();

    let _ = dir.read(|entry| match entry.kind {
        Kind::Dir => subdirs.push(entry.name),
        Kind::File => {
            usage.bytes += entry.allocated;
            usage.files += 1;
        }
        Kind::Other => {}
    });

    progress.add(usage);

    usage
        + (0..subdirs.len())
            .into_par_iter()
            .filter_map(|index| dir.open_at(subdirs.get(index)?).ok())
            .map(|child| walk(&child, progress))
            .reduce(Usage::default, Add::add)
}
