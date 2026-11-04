use std::fs;
use std::path::{Path, PathBuf};

use plist::Value;
use rayon::prelude::*;

use crate::scan;
use crate::scan::progress::Progress;

const APP_DIRS_DEPTH: u8 = 2;

#[derive(Debug, Clone)]
pub struct InstalledApp {
    pub name: String,
    pub path: PathBuf,
    pub bytes: u64,
    pub leftovers: Vec<PathBuf>,
    pub leftover_bytes: u64,
}

impl InstalledApp {
    pub fn total_bytes(&self) -> u64 {
        self.bytes + self.leftover_bytes
    }

    pub fn paths(&self) -> impl Iterator<Item = &PathBuf> {
        std::iter::once(&self.path).chain(&self.leftovers)
    }
}

pub fn scan(progress: &Progress) -> Vec<InstalledApp> {
    let Some(home) = dirs::home_dir() else {
        return Vec::new();
    };

    let library = home.join("Library");
    let group_containers = children(&library.join("Group Containers"));

    let mut bundles = Vec::new();
    for root in [PathBuf::from("/Applications"), home.join("Applications")] {
        collect_bundles(&root, APP_DIRS_DEPTH, &mut bundles);
    }

    progress.set_total(bundles.len() as u64);

    let mut apps: Vec<InstalledApp> = bundles
        .par_iter()
        .filter_map(|bundle| {
            let app = inspect(bundle, &library, &group_containers, progress);
            progress.advance();
            app
        })
        .collect();

    apps.sort_by_cached_key(|app| app.name.to_lowercase());
    apps
}

fn collect_bundles(dir: &Path, depth: u8, bundles: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };

    for entry in entries.flatten() {
        let path = entry.path();

        if path.extension().is_some_and(|extension| extension == "app") {
            bundles.push(path);
        } else if depth > 1 && entry.file_type().is_ok_and(|file_type| file_type.is_dir()) {
            collect_bundles(&path, depth - 1, bundles);
        }
    }
}

fn inspect(
    bundle: &Path,
    library: &Path,
    group_containers: &[PathBuf],
    progress: &Progress,
) -> Option<InstalledApp> {
    let info = Value::from_file(bundle.join("Contents/Info.plist")).ok()?;
    let info = info.as_dictionary()?;

    let bundle_id = info.get("CFBundleIdentifier")?.as_string()?;
    if bundle_id.starts_with("com.apple.") {
        return None;
    }

    let name = bundle.file_stem()?.to_str()?.to_owned();

    let mut names = vec![name.as_str()];
    if let Some(bundle_name) = info.get("CFBundleName").and_then(Value::as_string)
        && bundle_name != name
    {
        names.push(bundle_name);
    }

    let leftovers = leftovers(library, group_containers, bundle_id, &names);
    let leftover_bytes = leftovers
        .par_iter()
        .map(|path| scan::disk_usage(path, progress).bytes)
        .sum();

    Some(InstalledApp {
        bytes: scan::disk_usage(bundle, progress).bytes,
        path: bundle.to_path_buf(),
        name,
        leftovers,
        leftover_bytes,
    })
}

fn leftovers(
    library: &Path,
    group_containers: &[PathBuf],
    bundle_id: &str,
    names: &[&str],
) -> Vec<PathBuf> {
    let support = library.join("Application Support");
    let logs = library.join("Logs");

    let mut paths = vec![
        support.join(bundle_id),
        library.join("Caches").join(bundle_id),
        library.join("Containers").join(bundle_id),
        library.join("HTTPStorages").join(bundle_id),
        library.join("WebKit").join(bundle_id),
        library
            .join("Preferences")
            .join(format!("{bundle_id}.plist")),
        library
            .join("Saved Application State")
            .join(format!("{bundle_id}.savedState")),
    ];

    for name in names {
        paths.push(support.join(name));
        paths.push(logs.join(name));
    }

    let suffix = format!(".{bundle_id}");
    paths.extend(
        group_containers
            .iter()
            .filter(|path| {
                path.file_name()
                    .and_then(|name| name.to_str())
                    .is_some_and(|name| name.ends_with(&suffix))
            })
            .cloned(),
    );

    paths.sort_unstable();
    paths.dedup();
    paths.retain(|path| fs::symlink_metadata(path).is_ok());
    paths
}

fn children(dir: &Path) -> Vec<PathBuf> {
    fs::read_dir(dir)
        .map(|entries| entries.flatten().map(|entry| entry.path()).collect())
        .unwrap_or_default()
}
