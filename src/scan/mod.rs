pub mod apps;
pub mod junk;
pub mod large_files;

use std::fs::{self, Metadata};
use std::ops::Add;
use std::os::unix::fs::MetadataExt;
use std::path::Path;

use rayon::prelude::*;

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

pub fn allocated(metadata: &Metadata) -> u64 {
    metadata.blocks() * 512
}

pub fn disk_usage(path: &Path) -> Usage {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.is_dir() => walk(path),
        Ok(metadata) if metadata.is_file() => Usage {
            bytes: allocated(&metadata),
            files: 1,
        },
        _ => Usage::default(),
    }
}

fn walk(dir: &Path) -> Usage {
    let Ok(entries) = fs::read_dir(dir) else {
        return Usage::default();
    };

    let mut usage = Usage::default();
    let mut subdirs = Vec::new();

    for entry in entries.flatten() {
        let Ok(file_type) = entry.file_type() else {
            continue;
        };

        if file_type.is_dir() {
            subdirs.push(entry.path());
        } else if file_type.is_file()
            && let Ok(metadata) = entry.metadata()
        {
            usage.bytes += allocated(&metadata);
            usage.files += 1;
        }
    }

    usage
        + subdirs
            .par_iter()
            .map(|path| walk(path))
            .reduce(Usage::default, Add::add)
}
