use std::path::{Path, PathBuf};

pub struct LocalStorage {
    pub path: PathBuf,
}

impl LocalStorage {
    pub fn new(uuid: &str) -> Self {
        LocalStorage {
            path: Path::new(".").join(uuid),
        }
    }

    pub fn overpass(&self) -> String {
        format!("{}/overpass.xml", self.path.display())
    }

    pub fn json(&self) -> String {
        format!("{}/features.json", self.path.display())
    }
}
