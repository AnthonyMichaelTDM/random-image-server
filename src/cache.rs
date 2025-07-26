use std::{collections::HashMap, path::PathBuf};

use rand::prelude::*;
use url::Url;

pub trait CacheBackend: std::fmt::Debug + Send + Sync {
    /// Create a new cache backend
    fn new() -> Self
    where
        Self: Sized;

    /// Get an image from the cache by its URL
    fn get(&self, key: CacheKey) -> Option<CacheValue>;

    /// Get a random image from the cache
    fn get_random(&self) -> Option<CacheValue>;

    /// Store an image in the cache with its URL
    fn set(&mut self, key: CacheKey, image: CacheValue) -> Result<(), String>;

    /// Get the size of the cache
    fn size(&self) -> usize;

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
        self.cache.insert(key, image);
        Ok(())
    }

    fn size(&self) -> usize {
        self.cache.len()
    }

    fn clear(&mut self) -> Result<(), String> {
        self.cache.clear();
        Ok(())
    }
}
