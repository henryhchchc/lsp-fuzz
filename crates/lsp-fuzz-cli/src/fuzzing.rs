use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct FuzzerStateDir(PathBuf);

impl<P: Into<PathBuf>> From<P> for FuzzerStateDir {
    fn from(value: P) -> Self {
        Self(value.into())
    }
}

impl FuzzerStateDir {
    pub fn corpus_dir(&self) -> PathBuf {
        self.0.join("corpus")
    }

    pub fn solution_dir(&self) -> PathBuf {
        self.0.join("solutions")
    }
}
