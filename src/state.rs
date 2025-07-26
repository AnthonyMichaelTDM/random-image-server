use std::{fmt::Debug, sync::Mutex};

use crate::cache::{CacheBackend, CacheKey};

/// State for the server
#[derive(Debug)]
pub struct ServerState {
    /// List of image sources and their cached images
    pub sources: Vec<CacheKey>,

    /// Cache backend for storing images
    pub cache: Box<dyn CacheBackend>,

    /// What is the current index (for sequential image serving)
    pub current_index: Mutex<usize>,
}

impl Default for ServerState {
    fn default() -> Self {
        Self {
            sources: vec![],
            cache: Box::new(crate::cache::InMemoryCache::new()),
            current_index: Mutex::new(0),
        }
    }
}
