use std::{fs, io, path::PathBuf};

#[derive(Debug, Clone)]
pub struct FuzzerStateDir(PathBuf);

impl<P: Into<PathBuf>> From<P> for FuzzerStateDir {
    fn from(value: P) -> Self {
        Self(value.into())
    }
}

impl FuzzerStateDir {
    pub fn create(&self) -> io::Result<()> {
        fs::create_dir_all(&self.0)
    }

    pub fn corpus_dir(&self) -> PathBuf {
        self.0.join("corpus")
    }

    pub fn solution_dir(&self) -> PathBuf {
        self.0.join("solutions")
    }

    pub fn stats_file(&self) -> PathBuf {
        self.0.join("stats")
    }
}
