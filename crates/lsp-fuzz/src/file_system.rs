use std::{
    collections::VecDeque,
    path::{Path, PathBuf},
};

use libafl::inputs::HasTargetBytes;
use libafl_bolts::{AsSlice, HasLen};
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

    pub fn get(&self, name: &str) -> Option<&FileSystemEntry<F>> {
        if let Some((first_seg, remainder)) = name.split_once('/')
            && let Some(dir_entry @ FileSystemEntry::Directory(inner_dir)) =
                self.inner.get(first_seg)
        {
            if remainder.is_empty() {
                Some(dir_entry)
            } else {
                inner_dir.get(remainder)
            }
        } else {
            self.inner.get(name)
        }
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

    pub fn write_to_fs(&self, root: &Path) -> std::io::Result<()>
    where
        F: HasTargetBytes,
    {
        std::fs::create_dir_all(root)?;
        for (path, entry) in self.iter() {
            let item_path = root.join(path);
            match entry {
                FileSystemEntry::File(file) => {
                    std::fs::write(item_path, file.target_bytes().as_slice())?;
                }
                FileSystemEntry::Directory(dir) => {
                    dir.write_to_fs(&item_path)?;
                }
            }
        }
        Ok(())
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

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_get_simple_access() {
        let dir = FileSystemDirectory::<()>::from([
            (Utf8Input::from("file1"), FileSystemEntry::File(())),
            (Utf8Input::from("file2"), FileSystemEntry::File(())),
        ]);

        assert!(dir.get("file1").is_some());
        assert!(dir.get("file1").unwrap().is_file());
        assert!(dir.get("file2").is_some());
        assert!(dir.get("file2").unwrap().is_file());
        assert!(dir.get("nonexistent").is_none());
    }

    #[test]
    fn test_get_nested_access() {
        let nested_dir = FileSystemDirectory::<()>::from([(
            Utf8Input::from("nested_file"),
            FileSystemEntry::File(()),
        )]);

        let dir = FileSystemDirectory::<()>::from([
            (
                Utf8Input::from("subdir"),
                FileSystemEntry::Directory(nested_dir),
            ),
            (Utf8Input::from("file"), FileSystemEntry::File(())),
        ]);

        assert!(dir.get("file").is_some());
        assert!(dir.get("file").unwrap().is_file());

        assert!(dir.get("subdir").is_some());
        assert!(dir.get("subdir").unwrap().is_directory());

        assert!(dir.get("subdir/nested_file").is_some());
        assert!(dir.get("subdir/nested_file").unwrap().is_file());

        assert!(dir.get("subdir/nonexistent").is_none());
        assert!(dir.get("nonexistent/whatever").is_none());
    }

    #[test]
    fn test_get_deeply_nested() {
        let deepest = FileSystemDirectory::<()>::from([(
            Utf8Input::from("target"),
            FileSystemEntry::File(()),
        )]);

        let middle = FileSystemDirectory::<()>::from([(
            Utf8Input::from("next"),
            FileSystemEntry::Directory(deepest),
        )]);

        let dir = FileSystemDirectory::<()>::from([(
            Utf8Input::from("first"),
            FileSystemEntry::Directory(middle),
        )]);

        assert!(dir.get("first/next/target").is_some());
        assert!(dir.get("first/next/target").unwrap().is_file());
        assert!(dir.get("first/next/missing").is_none());
        assert!(dir.get("first/missing/target").is_none());
    }
}
