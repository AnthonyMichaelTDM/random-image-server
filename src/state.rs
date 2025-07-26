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
    #[must_use]
    pub fn create_backend(&self) -> Box<dyn CacheBackend> {
        match self {
            Self::InMemory => Box::new(crate::cache::InMemoryCache::new()),
            Self::FileSystem => Box::new(crate::cache::FileSystemCache::new()),
        }
    }
}

impl ServerState {
    /// Create a new `ServerState` with a specific configuration
    #[must_use]
    pub fn with_config(config: &crate::config::Config) -> Self {
        Self {
            cache: config.cache.backend.create_backend(),
            current_index: 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{CacheBackendType, CacheConfig, Config};
    use pretty_assertions::assert_eq;

    #[test]
    fn test_server_state_default() {
        let state = ServerState::default();
        assert_eq!(state.current_index, 0);
        assert!(state.cache.is_empty());
    }

    #[test]
    fn test_server_state_with_config_in_memory() {
        let config = Config {
            cache: CacheConfig {
                backend: CacheBackendType::InMemory,
            },
            ..Config::default()
        };
        let state = ServerState::with_config(&config);
        assert_eq!(state.cache.backend_type(), "InMemory");
        assert_eq!(state.current_index, 0);
        assert!(state.cache.is_empty());
    }

    #[test]
    fn test_server_state_with_config_file_system() {
        let config = Config {
            cache: CacheConfig {
                backend: CacheBackendType::FileSystem,
            },
            ..Config::default()
        };
        let state = ServerState::with_config(&config);
        assert_eq!(state.cache.backend_type(), "FileSystem");
        assert_eq!(state.current_index, 0);
        assert!(state.cache.is_empty());
    }

    #[test]
    fn test_cache_backend_type_create_backend_in_memory() {
        let backend = CacheBackendType::InMemory.create_backend();
        assert_eq!(backend.size(), 0);
        assert!(backend.is_empty());
    }

    #[test]
    fn test_cache_backend_type_create_backend_file_system() {
        let backend = CacheBackendType::FileSystem.create_backend();
        assert_eq!(backend.size(), 0);
        assert!(backend.is_empty());
    }
}
