use std::{collections::VecDeque, path::PathBuf};

use libafl_bolts::HasLen;
use ordermap::OrderMap;
use serde::{Deserialize, Serialize};

use crate::utf8::Utf8Input;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
#[repr(transparent)]
pub struct FileSystemDirectory<F> {
    inner: OrderMap<Utf8Input, FileSystemEntry<F>>,
}

impl<F> Default for FileSystemDirectory<F> {
    fn default() -> Self {
        Self {
            inner: Default::default(),
        }
    }
}

impl<F> FileSystemDirectory<F> {
    /// Returns if the directory is empty.
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    pub fn iter(&self) -> FileSystemIter<'_, F> {
        let queue = self
            .inner
            .iter()
            .map(|(name, item)| (PathBuf::from(name.as_str()), item))
            .collect();
        FileSystemIter { queue }
    }

    pub fn iter_files(&self) -> impl Iterator<Item = (PathBuf, &F)> + use<'_, F> {
        self.iter().filter_map(|(path, entry)| match entry {
            FileSystemEntry::File(file) => Some((path, file)),
            FileSystemEntry::Directory(_) => None,
        })
    }

    pub fn iter_files_mut(&mut self) -> FilesIterMut<'_, F> {
        let queue = self
            .inner
            .iter_mut()
            .map(|(name, item)| (PathBuf::from(name.as_str()), item))
            .collect();
        FilesIterMut { queue }
    }
}

impl<F, const N: usize> From<[(Utf8Input, FileSystemEntry<F>); N]> for FileSystemDirectory<F> {
    fn from(entries: [(Utf8Input, FileSystemEntry<F>); N]) -> Self {
        Self {
            inner: OrderMap::from(entries),
        }
    }
}

impl<F: HasLen> HasLen for FileSystemDirectory<F> {
    fn len(&self) -> usize {
        self.inner
            .iter()
            .map(|(name, content)| name.len() + content.len())
            .sum()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FileSystemEntry<F> {
    File(F),
    Directory(FileSystemDirectory<F>),
}

impl<F: HasLen> HasLen for FileSystemEntry<F> {
    fn len(&self) -> usize {
        match self {
            FileSystemEntry::File(f) => f.len(),
            FileSystemEntry::Directory(dir) => dir.len(),
        }
    }
}

impl<F> FileSystemEntry<F> {
    /// Returns if the entry is a file.
    pub const fn is_file(&self) -> bool {
        matches!(self, FileSystemEntry::File(_))
    }

    /// Returns if the entry is a directory.
    pub const fn is_directory(&self) -> bool {
        matches!(self, FileSystemEntry::Directory(_))
    }

    pub fn is_leave(&self) -> bool {
        match self {
            FileSystemEntry::File(_) => true,
            FileSystemEntry::Directory(entries) => entries.is_empty(),
        }
    }

    pub fn iter(&self) -> FileSystemIter<'_, F> {
        match self {
            file @ Self::File(_) => FileSystemIter {
                queue: VecDeque::from([(PathBuf::default(), file)]),
            },
            Self::Directory(dir) => dir.iter(),
        }
    }

    pub fn iter_files(&self) -> impl Iterator<Item = (PathBuf, &F)> + use<'_, F> {
        self.iter().filter_map(|(path, entry)| match entry {
            FileSystemEntry::File(file) => Some((path, file)),
            FileSystemEntry::Directory(_) => None,
        })
    }

    pub fn iter_files_mut(&mut self) -> FilesIterMut<'_, F> {
        match self {
            file @ Self::File(_) => FilesIterMut {
                queue: VecDeque::from(vec![(PathBuf::default(), file)]),
            },
            Self::Directory(dir) => dir.iter_files_mut(),
        }
    }
}

#[derive(Debug)]
pub struct FileSystemIter<'a, F> {
    queue: VecDeque<(PathBuf, &'a FileSystemEntry<F>)>,
}

#[derive(Debug)]
pub struct FilesIterMut<'a, F> {
    queue: VecDeque<(PathBuf, &'a mut FileSystemEntry<F>)>,
}

impl<'a, F> Iterator for FileSystemIter<'a, F> {
    type Item = (PathBuf, &'a FileSystemEntry<F>);

    fn next(&mut self) -> Option<Self::Item> {
        let (item_path, item) = self.queue.pop_front()?;
        if let FileSystemEntry::Directory(dir) = item {
            self.queue.extend(
                dir.inner
                    .iter()
                    .map(|(name, entry)| (item_path.join(name.as_str()), entry)),
            );
        }
        Some((item_path, item))
    }
}

impl<'a, F> Iterator for FilesIterMut<'a, F> {
    type Item = (PathBuf, &'a mut F);

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let (item_path, item) = self.queue.pop_front()?;
            match item {
                FileSystemEntry::File(file) => break Some((item_path, file)),
                FileSystemEntry::Directory(dir) => {
                    self.queue.extend(
                        dir.inner
                            .iter_mut()
                            .map(|(name, entry)| (item_path.join(name.as_str()), entry)),
                    );
                }
            }
        }
    }
}
