use std::cmp::Reverse;
use std::fs;
use std::ops::Add;
use std::path::{Path, PathBuf};

use plist::Value;
use rayon::prelude::*;

use crate::recycle::{self, Cleanup};
use crate::scan::progress::Progress;
use crate::scan::{self, Usage};

const LARGEST: usize = 12;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Category {
    UserCaches,
    SystemCaches,
    UserLogs,
    SystemLogs,
    Developer,
    BrokenLoginItems,
    DeviceBackups,
}

impl Category {
    pub const ALL: [Category; 7] = [
        Category::UserCaches,
        Category::SystemCaches,
        Category::UserLogs,
        Category::SystemLogs,
        Category::Developer,
        Category::BrokenLoginItems,
        Category::DeviceBackups,
    ];

    pub fn title(self) -> &'static str {
        match self {
            Category::UserCaches => "User Caches",
            Category::SystemCaches => "System Caches",
            Category::UserLogs => "User Logs",
            Category::SystemLogs => "System Logs",
            Category::Developer => "Developer Junk",
            Category::BrokenLoginItems => "Broken Login Items",
            Category::DeviceBackups => "iOS Device Backups",
        }
    }

    pub fn description(self) -> &'static str {
        match self {
            Category::UserCaches => {
                "Temporary data apps keep to load faster, including sandboxed apps. They rebuild it when needed."
            }
            Category::SystemCaches => {
                "Caches shared by every user. Some are owned by macOS and may need an admin to remove."
            }
            Category::UserLogs => "Diagnostic logs written by your apps and background services.",
            Category::SystemLogs => {
                "Logs written by macOS and system services. Some may need an admin to remove."
            }
            Category::Developer => {
                "Xcode build products, device support files and simulator caches. Xcode recreates them."
            }
            Category::BrokenLoginItems => {
                "Launch agents that point to apps or tools that no longer exist."
            }
            Category::DeviceBackups => {
                "Local iPhone and iPad backups. Only remove these if you back up to iCloud or elsewhere."
            }
        }
    }

    pub fn location(self) -> &'static str {
        match self {
            Category::UserCaches => "~/Library/Caches and app containers",
            Category::SystemCaches => "/Library/Caches",
            Category::UserLogs => "~/Library/Logs and app containers",
            Category::SystemLogs => "/Library/Logs and /var/log",
            Category::Developer => "~/Library/Developer",
            Category::BrokenLoginItems => "~/Library/LaunchAgents",
            Category::DeviceBackups => "~/Library/Application Support/MobileSync",
        }
    }

    pub fn selected_by_default(self) -> bool {
        !matches!(self, Category::DeviceBackups)
    }

    fn targets(self, home: &Path) -> Vec<Target> {
        let library = home.join("Library");

        match self {
            Category::UserCaches => {
                let mut targets = children(&library.join("Caches"), None);
                targets.extend(containers(&library, "Data/Library/Caches"));
                targets
            }
            Category::SystemCaches => children(Path::new("/Library/Caches"), None),
            Category::UserLogs => {
                let mut targets = children(&library.join("Logs"), None);
                targets.extend(containers(&library, "Data/Library/Logs"));
                targets
            }
            Category::SystemLogs => {
                let mut targets = children(Path::new("/Library/Logs"), None);
                targets.extend(children(Path::new("/private/var/log"), None));
                targets
            }
            Category::Developer => {
                let developer = library.join("Developer");
                let mut targets =
                    children(&developer.join("Xcode/DerivedData"), Some("DerivedData"));
                targets.extend(children(
                    &developer.join("Xcode/iOS DeviceSupport"),
                    Some("iOS DeviceSupport"),
                ));
                targets.extend(children(
                    &developer.join("CoreSimulator/Caches"),
                    Some("Simulator Caches"),
                ));
                targets
            }
            Category::BrokenLoginItems => broken_login_items(&library.join("LaunchAgents")),
            Category::DeviceBackups => {
                children(&library.join("Application Support/MobileSync/Backup"), None)
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct Target {
    pub path: PathBuf,
    pub name: String,
    pub bytes: u64,
}

#[derive(Debug, Clone)]
pub struct Entry {
    pub category: Category,
    pub usage: Usage,
    pub targets: Vec<Target>,
}

impl Entry {
    pub fn largest(&self) -> impl Iterator<Item = &Target> {
        self.targets.iter().take(LARGEST)
    }
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

    pub fn paths(&self, categories: &[Category]) -> Vec<PathBuf> {
        self.entries
            .iter()
            .filter(|entry| categories.contains(&entry.category))
            .flat_map(|entry| entry.targets.iter().map(|target| target.path.clone()))
            .collect()
    }
}

pub fn clean(paths: &[PathBuf]) -> Cleanup {
    recycle::move_to_trash(paths)
}

pub fn scan(progress: &Progress) -> Report {
    let Some(home) = dirs::home_dir() else {
        return Report::default();
    };

    progress.set_total(Category::ALL.len() as u64);

    let entries = Category::ALL
        .par_iter()
        .map(|&category| {
            let entry = measure(category, category.targets(&home), progress);
            progress.advance();
            entry
        })
        .collect();

    Report { entries }
}

fn measure(category: Category, targets: Vec<Target>, progress: &Progress) -> Entry {
    let mut measured: Vec<(Target, Usage)> = targets
        .into_par_iter()
        .map(|mut target| {
            let usage = scan::disk_usage(&target.path, progress);
            target.bytes = usage.bytes;
            (target, usage)
        })
        .collect();

    measured.sort_unstable_by_key(|(target, _)| Reverse(target.bytes));

    let usage = measured
        .iter()
        .map(|&(_, usage)| usage)
        .fold(Usage::default(), Add::add);

    Entry {
        category,
        usage,
        targets: measured.into_iter().map(|(target, _)| target).collect(),
    }
}

fn children(dir: &Path, group: Option<&str>) -> Vec<Target> {
    let Ok(entries) = fs::read_dir(dir) else {
        return Vec::new();
    };

    entries
        .flatten()
        .map(|entry| {
            let name = entry.file_name().to_string_lossy().into_owned();
            Target {
                path: entry.path(),
                name: match group {
                    Some(group) => format!("{group} › {name}"),
                    None => name,
                },
                bytes: 0,
            }
        })
        .collect()
}

fn containers(library: &Path, inner: &str) -> Vec<Target> {
    let Ok(entries) = fs::read_dir(library.join("Containers")) else {
        return Vec::new();
    };

    entries
        .flatten()
        .flat_map(|container| {
            let owner = container.file_name().to_string_lossy().into_owned();
            children(&container.path().join(inner), Some(&owner))
        })
        .collect()
}

fn broken_login_items(dir: &Path) -> Vec<Target> {
    children(dir, None)
        .into_iter()
        .filter(|target| {
            target
                .path
                .extension()
                .is_some_and(|extension| extension == "plist")
        })
        .filter(|target| program(&target.path).is_some_and(|program| !program.exists()))
        .collect()
}

fn program(plist: &Path) -> Option<PathBuf> {
    let value = Value::from_file(plist).ok()?;
    let agent = value.as_dictionary()?;

    let program = agent
        .get("Program")
        .and_then(Value::as_string)
        .or_else(|| {
            agent
                .get("ProgramArguments")?
                .as_array()?
                .first()?
                .as_string()
        })?;

    let program = Path::new(program);
    program.is_absolute().then(|| program.to_path_buf())
}
