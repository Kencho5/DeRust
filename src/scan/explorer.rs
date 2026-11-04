use std::borrow::Cow;
use std::cmp::Reverse;
use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};

use rayon::prelude::*;

use crate::scan::Usage;
use crate::scan::bulk::{self, Kind as EntryKind};
use crate::scan::disk::Disk;
use crate::scan::progress::Progress;

const MIN_FILE_BYTES: u64 = 1_000_000;
const SKIPPED: [&str; 5] = ["/System/Volumes", "/Volumes", "/dev", "/net", "/home"];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Folder,
    File(FileKind),
    Rest,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileKind {
    Video,
    Image,
    Audio,
    Document,
    Archive,
    DiskImage,
    Code,
    Other,
}

pub type Breakdown = [u64; FileKind::ALL.len()];

impl FileKind {
    pub const ALL: [FileKind; 8] = [
        FileKind::Video,
        FileKind::Image,
        FileKind::Audio,
        FileKind::Document,
        FileKind::Archive,
        FileKind::DiskImage,
        FileKind::Code,
        FileKind::Other,
    ];

    pub fn title(self) -> &'static str {
        match self {
            FileKind::Video => "Videos",
            FileKind::Image => "Images",
            FileKind::Audio => "Audio",
            FileKind::Document => "Documents",
            FileKind::Archive => "Archives",
            FileKind::DiskImage => "Disk Images",
            FileKind::Code => "Developer",
            FileKind::Other => "Other",
        }
    }

    fn of(name: &OsStr) -> FileKind {
        let Some(extension) = Path::new(name).extension().and_then(OsStr::to_str) else {
            return FileKind::Other;
        };

        let mut lowercase = [0u8; 8];
        let Some(slot) = lowercase.get_mut(..extension.len()) else {
            return FileKind::Other;
        };
        slot.copy_from_slice(extension.as_bytes());
        slot.make_ascii_lowercase();

        match &*slot {
            b"mp4" | b"mov" | b"mkv" | b"avi" | b"m4v" | b"webm" | b"wmv" | b"flv" => {
                FileKind::Video
            }
            b"jpg" | b"jpeg" | b"png" | b"gif" | b"heic" | b"heif" | b"webp" | b"tiff" | b"tif"
            | b"bmp" | b"raw" | b"cr2" | b"nef" | b"arw" | b"dng" | b"psd" => FileKind::Image,
            b"mp3" | b"wav" | b"flac" | b"aac" | b"m4a" | b"aiff" | b"aif" | b"ogg" => {
                FileKind::Audio
            }
            b"pdf" | b"doc" | b"docx" | b"xls" | b"xlsx" | b"ppt" | b"pptx" | b"pages"
            | b"numbers" | b"key" | b"txt" | b"rtf" | b"csv" | b"epub" => FileKind::Document,
            b"zip" | b"tar" | b"gz" | b"tgz" | b"bz2" | b"xz" | b"7z" | b"rar" | b"zst" => {
                FileKind::Archive
            }
            b"dmg" | b"iso" | b"img" | b"pkg" | b"vmdk" | b"qcow2" | b"vdi" | b"sparseimage"
            | b"sparsebundle" => FileKind::DiskImage,
            b"rlib" | b"rmeta" | b"o" | b"a" | b"dylib" | b"so" | b"jar" | b"class" | b"pyc"
            | b"wasm" | b"node" | b"pch" | b"pcm" | b"ipa" | b"xcarchive" => FileKind::Code,
            _ => FileKind::Other,
        }
    }

    fn index(self) -> usize {
        self as usize
    }
}

#[derive(Debug, Clone)]
pub struct Node {
    pub name: Box<OsStr>,
    pub kind: Kind,
    pub bytes: u64,
    pub items: u64,
    pub children: Vec<Node>,
}

#[derive(Debug, Clone)]
pub struct Tree {
    pub root: PathBuf,
    pub node: Node,
}

impl Node {
    fn file(name: &OsStr, bytes: u64) -> Node {
        Node {
            name: name.into(),
            kind: Kind::File(FileKind::of(name)),
            bytes,
            items: 1,
            children: Vec::new(),
        }
    }

    pub fn breakdown(&self) -> Breakdown {
        match self.kind {
            Kind::File(kind) => {
                let mut breakdown = Breakdown::default();
                breakdown[kind.index()] = self.bytes;
                breakdown
            }
            Kind::Rest => {
                let mut breakdown = Breakdown::default();
                breakdown[FileKind::Other.index()] = self.bytes;
                breakdown
            }
            Kind::Folder => self.children.par_iter().map(Node::breakdown).reduce(
                Breakdown::default,
                |mut total, part| {
                    for (sum, bytes) in total.iter_mut().zip(part) {
                        *sum += bytes;
                    }
                    total
                },
            ),
        }
    }

    pub fn files_of(&self, kind: FileKind, prefix: &mut Vec<usize>, found: &mut Vec<Vec<usize>>) {
        for (index, child) in self.children.iter().enumerate() {
            prefix.push(index);
            match child.kind {
                Kind::File(file) if file == kind => found.push(prefix.clone()),
                Kind::Folder => child.files_of(kind, prefix, found),
                _ => {}
            }
            prefix.pop();
        }
    }

    pub fn name(&self) -> Cow<'_, str> {
        self.name.to_string_lossy()
    }

    pub fn find(&self, indices: &[usize]) -> Option<&Node> {
        indices
            .iter()
            .try_fold(self, |node, &index| node.children.get(index))
    }

    fn remove(&mut self, indices: &[usize]) -> Option<(u64, u64)> {
        let (&index, rest) = indices.split_first()?;

        let removed = if rest.is_empty() {
            if index >= self.children.len() {
                return None;
            }
            let child = self.children.remove(index);
            (child.bytes, child.items)
        } else {
            self.children.get_mut(index)?.remove(rest)?
        };

        self.bytes -= removed.0;
        self.items -= removed.1;
        Some(removed)
    }
}

impl Tree {
    pub fn path(&self, indices: &[usize]) -> Option<PathBuf> {
        let mut path = self.root.clone();
        let mut node = &self.node;

        for &index in indices {
            node = node.children.get(index)?;
            path.push(&*node.name);
        }

        Some(path)
    }

    pub fn remove(&mut self, indices: &[usize]) {
        self.node.remove(indices);
    }
}

pub fn scan(progress: &Progress) -> Tree {
    let root = PathBuf::from("/");

    if let Some(disk) = Disk::root() {
        progress.set_total(disk.used());
    }

    let node = walk(&root, OsStr::new("Startup Disk"), progress);
    Tree { root, node }
}

fn walk(dir: &Path, name: &OsStr, progress: &Progress) -> Node {
    let mut children = Vec::new();
    let mut subdirs = Vec::new();
    let mut rest = Usage::default();
    let mut files = Usage::default();

    let _ = bulk::read_dir(dir, |entry| match entry.kind {
        EntryKind::Dir => {
            let path = dir.join(entry.name);
            if !is_skipped(&path) {
                subdirs.push(path);
            }
        }
        EntryKind::File => {
            files.bytes += entry.allocated;
            files.files += 1;

            if entry.allocated >= MIN_FILE_BYTES {
                children.push(Node::file(entry.name, entry.allocated));
            } else {
                rest.bytes += entry.allocated;
                rest.files += 1;
            }
        }
        EntryKind::Other => {}
    });

    progress.add(files);
    progress.advance_by(files.bytes);

    children.par_extend(subdirs.par_iter().map(|path| {
        let name = path.file_name().unwrap_or(path.as_os_str());
        walk(path, name, progress)
    }));

    if rest.files > 0 {
        children.push(Node {
            name: OsString::from(format!("{} smaller files", rest.files)).into(),
            kind: Kind::Rest,
            bytes: rest.bytes,
            items: rest.files,
            children: Vec::new(),
        });
    }

    children.sort_unstable_by_key(|child| Reverse(child.bytes));
    children.shrink_to_fit();

    Node {
        name: name.into(),
        kind: Kind::Folder,
        bytes: children.iter().map(|child| child.bytes).sum(),
        items: children.iter().map(|child| child.items).sum(),
        children,
    }
}

fn is_skipped(path: &Path) -> bool {
    SKIPPED.iter().any(|skipped| path.as_os_str() == *skipped)
}
