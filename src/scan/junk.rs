use std::fs;
use std::ops::Add;
use std::path::PathBuf;

use rayon::prelude::*;

use crate::recycle::{self, Cleanup};
use crate::scan::{self, Usage};

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
            usage: scan::disk_usage(&home.join(category.path())),
        })
        .collect();

    Report { entries }
}
