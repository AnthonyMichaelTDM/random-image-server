use std::{collections::HashMap, fs, path::PathBuf};

use rand::prelude::*;
use url::Url;

pub trait CacheBackend: std::fmt::Debug + Send + Sync {
    /// Create a new cache backend
    fn new() -> Self
    where
        Self: Sized;

    /// Get an image from the cache by its key
    fn get(&self, key: CacheKey) -> Option<CacheValue>;

    /// Get a random image from the cache
    fn get_random(&self) -> Option<CacheValue>;

    /// Store an image in the cache with its key
    fn set(&mut self, key: CacheKey, image: CacheValue) -> Result<(), String>;

    /// Remove an image from the cache by its key
    fn remove(&mut self, key: &CacheKey) -> Option<CacheValue>;

    /// Get the size of the cache
    fn size(&self) -> usize;

    /// Check if the cache is empty
    fn is_empty(&self) -> bool {
        self.size() == 0
    }

    /// Retrieve the keys in the cache
    fn keys(&self) -> &[CacheKey];

    /// Clear the cache
    fn clear(&mut self) -> Result<(), String>;
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum CacheKey {
    /// Cache key for an image URL
    ImageUrl(Url),
    /// Cache key for an image path
    ImagePath(PathBuf),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CacheValue {
    pub data: Vec<u8>,
    pub content_type: String,
}

#[derive(Debug)]
pub struct InMemoryCache {
    keys: Vec<CacheKey>,
    cache: HashMap<CacheKey, CacheValue>,
}

// Implement Default for InMemoryCache specifically
impl Default for InMemoryCache {
    fn default() -> Self {
        Self::new()
    }
}

impl CacheBackend for InMemoryCache {
    fn new() -> Self {
        Self {
            cache: HashMap::new(),
            keys: Vec::new(),
        }
    }

    fn get(&self, key: CacheKey) -> Option<CacheValue> {
        self.cache.get(&key).cloned()
    }

    fn get_random(&self) -> Option<CacheValue> {
        let keys: Vec<&CacheKey> = self.cache.keys().collect();
        if let Some(random_key) = keys.choose(&mut rand::rng()) {
            self.cache.get(random_key).cloned()
        } else {
            None
        }
    }

    fn set(&mut self, key: CacheKey, image: CacheValue) -> Result<(), String> {
        if !self.keys.contains(&key) {
            self.keys.push(key.clone());
        }
        self.cache.insert(key, image);
        Ok(())
    }

    fn remove(&mut self, key: &CacheKey) -> Option<CacheValue> {
        self.keys.retain(|k| k != key);
        self.cache.remove(key)
    }

    fn size(&self) -> usize {
        self.cache.len()
    }

    fn clear(&mut self) -> Result<(), String> {
        self.cache.clear();
        Ok(())
    }

    fn keys(&self) -> &[CacheKey] {
        debug_assert!(
            self.keys.len() == self.cache.len(),
            "Keys and cache size mismatch: {} != {}",
            self.keys.len(),
            self.cache.len()
        );
        &self.keys
    }
}

#[derive(Debug)]
struct FileSystemCacheValue {
    path: PathBuf,
    hash: String,
    content_type: String,
}

#[derive(Debug)]
pub struct FileSystemCache {
    tempdir: tempdir::TempDir,
    keys: Vec<CacheKey>,
    // map of keys to file paths and the hash of the file content
    cache: HashMap<CacheKey, FileSystemCacheValue>,
}

impl CacheBackend for FileSystemCache {
    fn new() -> Self {
        let tempdir =
            tempdir::TempDir::new("random_image_server_cache").expect("Failed to create temp dir");
        Self {
            tempdir,
            keys: Vec::new(),
            cache: HashMap::new(),
        }
    }

    fn get(&self, key: CacheKey) -> Option<CacheValue> {
        if let Some(FileSystemCacheValue {
            path,
            hash,
            content_type,
        }) = self.cache.get(&key)
        {
            if path.exists() {
                let data = std::fs::read(path).ok()?;
                // Validate the content type based on the file extension
                if hash != &format!("{:x}", md5::compute(&data)) {
                    log::warn!("Hash mismatch for cached file: {}", path.display());
                    fs::remove_file(path).ok()?;
                    return None;
                }

                return Some(CacheValue {
                    data,
                    content_type: content_type.clone(),
                });
            }
        }
        None
    }

    fn get_random(&self) -> Option<CacheValue> {
        let keys: Vec<&CacheKey> = self.cache.keys().collect();
        if let Some(random_key) = keys.choose(&mut rand::rng()).cloned() {
            self.get(random_key.clone())
        } else {
            None
        }
    }

    fn set(&mut self, key: CacheKey, image: CacheValue) -> Result<(), String> {
        let file_path = self
            .tempdir
            .path()
            .join(format!("{}.cache", uuid::Uuid::new_v4()));
        std::fs::write(&file_path, &image.data).map_err(|e| e.to_string())?;

        if !self.keys.contains(&key) {
            self.keys.push(key.clone());
        } else {
            log::warn!("Key already exists in cache: {:?}", key);
            if let Some(FileSystemCacheValue { path, .. }) = self.cache.get(&key) {
                fs::remove_file(path).ok();
            }
        }

        let hash = md5::compute(&image.data);
        let hash_str = format!("{:x}", hash);

        let content_type = image.content_type.clone();

        self.cache.insert(
            key,
            FileSystemCacheValue {
                path: file_path,
                hash: hash_str,
                content_type,
            },
        );
        Ok(())
    }

    fn remove(&mut self, key: &CacheKey) -> Option<CacheValue> {
        if let Some(FileSystemCacheValue { path, .. }) = self.cache.remove(key) {
            if path.exists() {
                let content_type = mime_guess::from_path(&path)
                    .first_or_octet_stream()
                    .to_string();
                fs::remove_file(&path).ok()?;

                let data = std::fs::read(path).ok()?;
                return Some(CacheValue { data, content_type });
            }
        }
        None
    }

    fn size(&self) -> usize {
        self.cache.len()
    }

    fn clear(&mut self) -> Result<(), String> {
        self.cache.clear();
        Ok(())
    }

    fn keys(&self) -> &[CacheKey] {
        debug_assert!(
            self.keys.len() == self.cache.len(),
            "Keys and cache size mismatch: {} != {}",
            self.keys.len(),
            self.cache.len()
        );
        &self.keys
    }
}
