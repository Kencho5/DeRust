use std::cell::RefCell;
use std::ffi::{CStr, CString, OsStr};
use std::io;
use std::mem::size_of;
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};
use std::os::unix::ffi::OsStrExt;
use std::path::Path;

use libc::{
    ATTR_BIT_MAP_COUNT, ATTR_CMN_NAME, ATTR_CMN_OBJTYPE, ATTR_CMN_RETURNED_ATTRS,
    ATTR_FILE_ALLOCSIZE, attribute_set_t, attrlist, attrreference_t, c_int,
};

const BUFFER_SIZE: usize = 128 * 1024;
const OPEN_FLAGS: c_int = libc::O_RDONLY | libc::O_DIRECTORY | libc::O_CLOEXEC;
const VREG: u32 = 1;
const VDIR: u32 = 2;

thread_local! {
    static BUFFER: RefCell<Box<[u8]>> = RefCell::new(vec![0; BUFFER_SIZE].into_boxed_slice());
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    File,
    Dir,
    Other,
}

#[derive(Debug)]
pub struct Entry<'a> {
    pub name: &'a OsStr,
    pub kind: Kind,
    pub allocated: u64,
}

#[derive(Debug)]
pub struct Dir {
    fd: OwnedFd,
}

#[derive(Debug, Default)]
pub struct Names {
    bytes: Vec<u8>,
    ends: Vec<usize>,
}

impl Names {
    pub fn push(&mut self, name: &OsStr) {
        self.bytes.extend_from_slice(name.as_bytes());
        self.bytes.push(0);
        self.ends.push(self.bytes.len());
    }

    pub fn len(&self) -> usize {
        self.ends.len()
    }

    pub fn get(&self, index: usize) -> Option<&CStr> {
        let end = *self.ends.get(index)?;
        let start = index
            .checked_sub(1)
            .map_or(0, |previous| self.ends[previous]);
        CStr::from_bytes_with_nul(self.bytes.get(start..end)?).ok()
    }
}

impl Dir {
    pub fn open(path: &Path) -> io::Result<Dir> {
        let path = CString::new(path.as_os_str().as_bytes())?;
        Dir::from_raw(unsafe { libc::open(path.as_ptr(), OPEN_FLAGS) })
    }

    pub fn open_at(&self, name: &CStr) -> io::Result<Dir> {
        Dir::from_raw(unsafe {
            libc::openat(
                self.fd.as_raw_fd(),
                name.as_ptr(),
                OPEN_FLAGS | libc::O_NOFOLLOW,
            )
        })
    }

    fn from_raw(fd: c_int) -> io::Result<Dir> {
        if fd < 0 {
            return Err(io::Error::last_os_error());
        }

        Ok(Dir {
            fd: unsafe { OwnedFd::from_raw_fd(fd) },
        })
    }

    pub fn read(&self, visit: impl FnMut(Entry<'_>)) -> io::Result<()> {
        list(&self.fd, visit)
    }
}

fn list(fd: &OwnedFd, mut visit: impl FnMut(Entry<'_>)) -> io::Result<()> {
    let mut attributes = attrlist {
        bitmapcount: ATTR_BIT_MAP_COUNT,
        reserved: 0,
        commonattr: ATTR_CMN_RETURNED_ATTRS | ATTR_CMN_NAME | ATTR_CMN_OBJTYPE,
        volattr: 0,
        dirattr: 0,
        fileattr: ATTR_FILE_ALLOCSIZE,
        forkattr: 0,
    };

    BUFFER.with_borrow_mut(|buffer| {
        loop {
            let count = unsafe {
                libc::getattrlistbulk(
                    fd.as_raw_fd(),
                    (&raw mut attributes).cast(),
                    buffer.as_mut_ptr().cast(),
                    buffer.len(),
                    0,
                )
            };

            if count < 0 {
                let error = io::Error::last_os_error();
                if error.kind() == io::ErrorKind::Interrupted {
                    continue;
                }
                return Err(error);
            }
            if count == 0 {
                return Ok(());
            }

            let mut offset = 0;
            for _ in 0..count {
                let Some(length) = read::<u32>(buffer, offset) else {
                    break;
                };
                let length = length as usize;
                let Some(record) = buffer.get(offset..offset + length) else {
                    break;
                };

                if let Some(entry) = parse(record) {
                    visit(entry);
                }

                offset += length;
            }
        }
    })
}

fn parse(record: &[u8]) -> Option<Entry<'_>> {
    let mut cursor = size_of::<u32>();

    let returned: attribute_set_t = read(record, cursor)?;
    cursor += size_of::<attribute_set_t>();

    if returned.commonattr & ATTR_CMN_NAME == 0 {
        return None;
    }

    let reference: attrreference_t = read(record, cursor)?;
    let start = cursor.checked_add_signed(reference.attr_dataoffset as isize)?;
    let bytes = record.get(start..start + reference.attr_length as usize)?;
    let name = OsStr::from_bytes(bytes.strip_suffix(&[0]).unwrap_or(bytes));
    cursor += size_of::<attrreference_t>();

    let mut kind = Kind::Other;
    if returned.commonattr & ATTR_CMN_OBJTYPE != 0 {
        kind = match read::<u32>(record, cursor)? {
            VREG => Kind::File,
            VDIR => Kind::Dir,
            _ => Kind::Other,
        };
        cursor += size_of::<u32>();
    }

    let mut allocated = 0;
    if returned.fileattr & ATTR_FILE_ALLOCSIZE != 0 {
        allocated = read::<i64>(record, cursor)?.max(0) as u64;
    }

    Some(Entry {
        name,
        kind,
        allocated,
    })
}

fn read<T: Copy>(bytes: &[u8], offset: usize) -> Option<T> {
    let slice = bytes.get(offset..offset.checked_add(size_of::<T>())?)?;
    Some(unsafe { slice.as_ptr().cast::<T>().read_unaligned() })
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::fs;
    use std::os::unix::fs::MetadataExt;

    use super::*;

    #[test]
    fn matches_std_read_dir() {
        let dir = std::env::temp_dir().join(format!("derust-bulk-{}", std::process::id()));
        fs::create_dir_all(dir.join("nested")).unwrap();
        fs::write(dir.join("small.txt"), b"hello").unwrap();
        fs::write(dir.join("large.bin"), vec![7u8; 300_000]).unwrap();
        std::os::unix::fs::symlink("small.txt", dir.join("link")).unwrap();

        let mut found = HashMap::new();
        Dir::open(&dir)
            .unwrap()
            .read(|entry| {
                found.insert(entry.name.to_owned(), (entry.kind, entry.allocated));
            })
            .unwrap();

        for entry in fs::read_dir(&dir).unwrap().flatten() {
            let metadata = entry.metadata().unwrap();
            let kind = if metadata.is_dir() {
                Kind::Dir
            } else if metadata.is_file() {
                Kind::File
            } else {
                Kind::Other
            };
            let allocated = if kind == Kind::File {
                metadata.blocks() * 512
            } else {
                0
            };

            assert_eq!(found.remove(&entry.file_name()), Some((kind, allocated)));
        }

        assert!(found.is_empty());
        fs::remove_dir_all(&dir).unwrap();
    }
}
