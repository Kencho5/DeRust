use std::fs;
use std::ops::Add;
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};

use rayon::prelude::*;

use crate::recycle::{self, Cleanup};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Category {
    UserCaches,
    UserLogs,
    XcodeDerivedData,
}

impl Category {
    pub const ALL: [Category; 3] = [
        Category::UserCaches,
        Category::UserLogs,
        Category::XcodeDerivedData,
    ];

    pub fn title(self) -> &'static str {
        match self {
            Category::UserCaches => "User Caches",
            Category::UserLogs => "User Logs",
            Category::XcodeDerivedData => "Xcode Derived Data",
        }
    }

    fn path(self) -> &'static str {
        match self {
            Category::UserCaches => "Library/Caches",
            Category::UserLogs => "Library/Logs",
            Category::XcodeDerivedData => "Library/Developer/Xcode/DerivedData",
        }
    }
}

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

#[derive(Debug, Clone)]
pub struct Entry {
    pub category: Category,
    pub usage: Usage,
}

#[derive(Debug, Clone, Default)]
pub struct Report {
    pub entries: Vec<Entry>,
}

impl Report {
    pub fn total(&self) -> Usage {
        self.entries
            .iter()
            .map(|entry| entry.usage)
            .fold(Usage::default(), Add::add)
    }
}

pub fn clean(categories: &[Category]) -> Cleanup {
    let Some(home) = dirs::home_dir() else {
        return Cleanup::default();
    };

    let paths: Vec<PathBuf> = categories
        .iter()
        .filter_map(|category| fs::read_dir(home.join(category.path())).ok())
        .flat_map(|entries| entries.flatten().map(|entry| entry.path()))
        .collect();

    recycle::move_to_trash(&paths)
}

pub fn scan() -> Report {
    let Some(home) = dirs::home_dir() else {
        return Report::default();
    };

    let entries = Category::ALL
        .par_iter()
        .map(|&category| Entry {
            category,
            usage: walk(&home.join(category.path())),
        })
        .collect();

    Report { entries }
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
            usage.bytes += metadata.blocks() * 512;
            usage.files += 1;
        }
    }

    usage
        + subdirs
            .par_iter()
            .map(|path| walk(path))
            .reduce(Usage::default, Add::add)
}
