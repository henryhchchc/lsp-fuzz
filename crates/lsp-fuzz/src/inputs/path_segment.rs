use std::path::Path;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PathSegmentInput {
    pub inner: String,
}

impl PathSegmentInput {
    pub fn as_path(&self) -> &Path {
        Path::new(&self.inner)
    }
}
