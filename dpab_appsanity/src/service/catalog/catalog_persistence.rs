use std::{fs, path::Path};

use tracing::{debug, error};

const HASH_FILENAME: &'static str = "apps/catalog_hash";

#[derive(Debug)]
pub struct CatalogHashStorage {
    path: String,
    hash: Option<String>,
}

impl CatalogHashStorage {
    pub fn new(path: String) -> CatalogHashStorage {
        CatalogHashStorage {
            path: format!("{}/{}", path, HASH_FILENAME),
            hash: None,
        }
    }

    pub fn get(&mut self) -> String {
        if let None = self.hash {
            self.read_from_storage();
        }
        self.hash.clone().unwrap_or_default()
    }

    pub fn set(&mut self, hash: String) {
        self.hash = Some(hash);
        self.write_to_storage();
    }

    fn read_from_storage(&mut self) {
        debug!("read_from_storage: self.path={}", self.path);
        let path = Path::new(&self.path);
        if let Some(p) = path.to_str() {
            match fs::read_to_string(&p) {
                Ok(hash) => {
                    debug!("read_from_storage: hash={}", hash);
                    self.hash = Some(hash);
                }
                Err(e) => {
                    error!("read_from_storage: path={}, e={:?}", self.path, e);
                }
            }
        } else {
            error!("read_from_storage: Invalid path: path={}", self.path);
        }
    }

    fn write_to_storage(&self) {
        debug!("write_to_storage: self.path={}", self.path);

        if let None = self.hash {
            error!("write_to_storage: Hash not set");
            return;
        }

        let path = Path::new(&self.path);

        let parent = path.parent();
        if let None = parent {
            error!("write_to_storage: Invalid path: path={}", self.path);
            return;
        }

        if let Err(e) = fs::create_dir_all(parent.unwrap()) {
            error!("write_to_storage: Creation error: path={}", self.path);
            return;
        }

        if let Some(p) = path.to_str() {
            if let Err(e) = fs::write(p, self.hash.clone().unwrap()) {
                error!("write_to_storage: Write error: path={}, e={:?}", p, e);
            }
        } else {
            error!("write_to_storage: Invalid path: path={}", self.path);
        }
    }
}
