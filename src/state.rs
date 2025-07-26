use std::fmt::Debug;

use crate::{cache::CacheBackend, config::CacheBackendType};

/// State for the server
#[derive(Debug)]
pub struct ServerState {
    /// Cache backend for storing images
    pub cache: Box<dyn CacheBackend>,

    /// What is the current index (for sequential image serving)
    pub current_index: usize,
}

impl Default for ServerState {
    fn default() -> Self {
        Self {
            cache: Box::new(crate::cache::InMemoryCache::new()),
            current_index: 0,
        }
    }
}

impl CacheBackendType {
    /// Create a new cache backend based on the type
    pub fn create_backend(&self) -> Box<dyn CacheBackend> {
        match self {
            CacheBackendType::InMemory => Box::new(crate::cache::InMemoryCache::new()),
            CacheBackendType::FileSystem => Box::new(crate::cache::FileSystemCache::new()),
        }
    }
}

impl ServerState {
    /// Create a new ServerState with a specific configuration
    pub fn with_config(config: &crate::config::Config) -> Self {
        let mut state: ServerState = Self::default();
        state.cache = config.cache.backend.create_backend();
        state.current_index = 0;
        state
    }
}
