use std::path::PathBuf;

use pretty_assertions::assert_eq;
use random_image_server::cache::{CacheBackend, CacheKey, CacheValue, InMemoryCache};
use url::Url;

#[test]
fn test_new_cache() {
    let cache = InMemoryCache::new();
    assert_eq!(cache.size(), 0);
    assert!(cache.is_empty());
    assert!(cache.keys().is_empty());
}

#[test]
fn test_set_and_get() {
    let mut cache = InMemoryCache::new();
    let key = CacheKey::ImagePath(PathBuf::from("/test/image.jpg"));
    let value = CacheValue {
        data: vec![1, 2, 3, 4],
        content_type: "image/jpeg".to_string(),
    };

    assert!(cache.set(key.clone(), value.clone()).is_ok());
    assert_eq!(cache.size(), 1);
    assert!(!cache.is_empty());
    assert_eq!(cache.get(key.clone()), Some(value));
}

#[test]
fn test_get_nonexistent() {
    let cache = InMemoryCache::new();
    let key = CacheKey::ImagePath(PathBuf::from("/nonexistent.jpg"));
    assert_eq!(cache.get(key), None);
}

#[test]
fn test_remove() {
    let mut cache = InMemoryCache::new();
    let key = CacheKey::ImagePath(PathBuf::from("/test/image.jpg"));
    let value = CacheValue {
        data: vec![1, 2, 3, 4],
        content_type: "image/jpeg".to_string(),
    };

    cache.set(key.clone(), value.clone()).unwrap();
    assert_eq!(cache.size(), 1);

    let removed = cache.remove(&key);
    assert_eq!(removed, Some(value));
    assert_eq!(cache.size(), 0);
    assert!(cache.is_empty());
}

#[test]
fn test_remove_nonexistent() {
    let mut cache = InMemoryCache::new();
    let key = CacheKey::ImagePath(PathBuf::from("/nonexistent.jpg"));
    assert_eq!(cache.remove(&key), None);
}

#[test]
fn test_get_random_empty_cache() {
    let cache = InMemoryCache::new();
    assert_eq!(cache.get_random(), None);
}

#[test]
fn test_get_random_single_item() {
    let mut cache = InMemoryCache::new();
    let key = CacheKey::ImagePath(PathBuf::from("/test/image.jpg"));
    let value = CacheValue {
        data: vec![1, 2, 3, 4],
        content_type: "image/jpeg".to_string(),
    };

    cache.set(key, value.clone()).unwrap();
    assert_eq!(cache.get_random(), Some(value));
}

#[test]
fn test_get_random_multiple_items() {
    let mut cache = InMemoryCache::new();
    let key1 = CacheKey::ImagePath(PathBuf::from("/test/image1.jpg"));
    let key2 = CacheKey::ImagePath(PathBuf::from("/test/image2.jpg"));
    let value1 = CacheValue {
        data: vec![1, 2, 3, 4],
        content_type: "image/jpeg".to_string(),
    };
    let value2 = CacheValue {
        data: vec![5, 6, 7, 8],
        content_type: "image/png".to_string(),
    };

    cache.set(key1, value1.clone()).unwrap();
    cache.set(key2, value2.clone()).unwrap();

    // Test that get_random returns one of the values
    let random_value = cache.get_random().unwrap();
    assert!(random_value == value1 || random_value == value2);
}

#[test]
fn test_clear() {
    let mut cache = InMemoryCache::new();
    let key = CacheKey::ImagePath(PathBuf::from("/test/image.jpg"));
    let value = CacheValue {
        data: vec![1, 2, 3, 4],
        content_type: "image/jpeg".to_string(),
    };

    cache.set(key, value).unwrap();
    assert_eq!(cache.size(), 1);

    assert!(cache.clear().is_ok());
    assert_eq!(cache.size(), 0);
    assert!(cache.is_empty());
}

#[test]
fn test_keys() {
    let mut cache = InMemoryCache::new();
    let key1 = CacheKey::ImagePath(PathBuf::from("/test/image1.jpg"));
    let key2 = CacheKey::ImageUrl(Url::parse("https://example.com/image.jpg").unwrap());
    let value = CacheValue {
        data: vec![1, 2, 3, 4],
        content_type: "image/jpeg".to_string(),
    };

    cache.set(key1.clone(), value.clone()).unwrap();
    cache.set(key2.clone(), value).unwrap();

    let keys = cache.keys();
    assert_eq!(keys.len(), 2);
    assert!(keys.contains(&key1));
    assert!(keys.contains(&key2));
}

#[test]
fn test_set_duplicate_key() {
    let mut cache = InMemoryCache::new();
    let key = CacheKey::ImagePath(PathBuf::from("/test/image.jpg"));
    let value1 = CacheValue {
        data: vec![1, 2, 3, 4],
        content_type: "image/jpeg".to_string(),
    };
    let value2 = CacheValue {
        data: vec![5, 6, 7, 8],
        content_type: "image/png".to_string(),
    };

    cache.set(key.clone(), value1).unwrap();
    assert_eq!(cache.size(), 1);

    // Setting with same key should overwrite
    cache.set(key.clone(), value2.clone()).unwrap();
    assert_eq!(cache.size(), 1);
    assert_eq!(cache.get(key), Some(value2));
}
