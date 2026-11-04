use std::cmp::Reverse;
use std::fs;
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};

use rayon::prelude::*;

const MIN_BYTES: u64 = 100_000_000;
const LIMIT: usize = 500;
const SKIPPED_DIRS: [&str; 2] = ["Library", ".Trash"];
const BUNDLE_EXTENSIONS: [&str; 3] = ["app", "photoslibrary", "musiclibrary"];

#[derive(Debug, Clone)]
pub struct LargeFile {
    pub path: PathBuf,
    pub bytes: u64,
}

impl LargeFile {
    pub fn name(&self) -> &str {
        self.path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or_default()
    }
}

pub fn scan() -> Vec<LargeFile> {
    let Some(home) = dirs::home_dir() else {
        return Vec::new();
    };

    let skipped: Vec<PathBuf> = SKIPPED_DIRS.iter().map(|dir| home.join(dir)).collect();

    let mut files = walk(&home, &skipped);
    files.sort_unstable_by_key(|file| Reverse(file.bytes));
    files.truncate(LIMIT);
    files
}

fn walk(dir: &Path, skipped: &[PathBuf]) -> Vec<LargeFile> {
    let Ok(entries) = fs::read_dir(dir) else {
        return Vec::new();
    };

    let mut files = Vec::new();
    let mut subdirs = Vec::new();

    for entry in entries.flatten() {
        let Ok(file_type) = entry.file_type() else {
            continue;
        };

        if file_type.is_dir() {
            let path = entry.path();
            if !is_skipped(&path, skipped) {
                subdirs.push(path);
            }
        } else if file_type.is_file()
            && let Ok(metadata) = entry.metadata()
        {
            let bytes = metadata.blocks() * 512;
            if bytes >= MIN_BYTES {
                files.push(LargeFile {
                    path: entry.path(),
                    bytes,
                });
            }
        }
    }

    let nested = subdirs
        .par_iter()
        .map(|path| walk(path, skipped))
        .reduce(Vec::new, |mut all, mut found| {
            all.append(&mut found);
            all
        });

    files.extend(nested);
    files
}

fn is_skipped(path: &Path, skipped: &[PathBuf]) -> bool {
    skipped.iter().any(|dir| dir == path)
        || path
            .extension()
            .and_then(|extension| extension.to_str())
            .is_some_and(|extension| BUNDLE_EXTENSIONS.contains(&extension))
}
