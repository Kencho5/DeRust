use std::cmp::Reverse;
use std::path::{Path, PathBuf};

use rayon::prelude::*;

use crate::scan::Usage;
use crate::scan::bulk::{self, Kind};
use crate::scan::progress::Progress;

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

pub fn scan(progress: &Progress) -> Vec<LargeFile> {
    let Some(home) = dirs::home_dir() else {
        return Vec::new();
    };

    let skipped: Vec<PathBuf> = SKIPPED_DIRS.iter().map(|dir| home.join(dir)).collect();

    let mut files = walk(&home, &skipped, progress);
    files.sort_unstable_by_key(|file| Reverse(file.bytes));
    files.truncate(LIMIT);
    files
}

fn walk(dir: &Path, skipped: &[PathBuf], progress: &Progress) -> Vec<LargeFile> {
    let mut files = Vec::new();
    let mut subdirs = Vec::new();
    let mut usage = Usage::default();

    let _ = bulk::read_dir(dir, |entry| match entry.kind {
        Kind::Dir => {
            let path = dir.join(entry.name);
            if !is_skipped(&path, skipped) {
                subdirs.push(path);
            }
        }
        Kind::File => {
            usage.bytes += entry.allocated;
            usage.files += 1;

            if entry.allocated >= MIN_BYTES {
                files.push(LargeFile {
                    path: dir.join(entry.name),
                    bytes: entry.allocated,
                });
            }
        }
        Kind::Other => {}
    });

    progress.add(usage);

    files.par_extend(
        subdirs
            .par_iter()
            .flat_map_iter(|path| walk(path, skipped, progress)),
    );
    files
}

fn is_skipped(path: &Path, skipped: &[PathBuf]) -> bool {
    skipped.iter().any(|dir| dir == path)
        || path
            .extension()
            .and_then(|extension| extension.to_str())
            .is_some_and(|extension| BUNDLE_EXTENSIONS.contains(&extension))
}
