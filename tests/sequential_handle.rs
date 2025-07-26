use std::{path::PathBuf, sync::Arc};

use pretty_assertions::assert_eq;
use random_image_server::{
    cache::{CacheKey, CacheValue},
    handle_sequential_image,
    state::ServerState,
};
use tokio::sync::RwLock;

#[tokio::test]
async fn test_handle_sequential_image_empty_cache() {
    let state = Arc::new(RwLock::new(ServerState::default()));
    let result = handle_sequential_image(state).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_handle_sequential_image_with_cache() {
    let mut server_state = ServerState::default();
    let key = CacheKey::ImagePath(PathBuf::from("/test/image.jpg"));
    let value = CacheValue {
        data: vec![1, 2, 3, 4],
        content_type: "image/jpeg".to_string(),
    };
    server_state.cache.set(key, value).unwrap();

    let state = Arc::new(RwLock::new(server_state));
    let result = handle_sequential_image(state).await;
    assert!(result.is_ok());

    let response = result.unwrap();
    assert_eq!(response.status(), hyper::StatusCode::OK);
}

#[tokio::test]
async fn test_handle_sequential_image_index_increment() {
    let mut server_state = ServerState::default();
    let key1 = CacheKey::ImagePath(PathBuf::from("/test/image1.jpg"));
    let key2 = CacheKey::ImagePath(PathBuf::from("/test/image2.jpg"));
    let value = CacheValue {
        data: vec![1, 2, 3, 4],
        content_type: "image/jpeg".to_string(),
    };
    server_state.cache.set(key1, value.clone()).unwrap();
    server_state.cache.set(key2, value).unwrap();

    let state = Arc::new(RwLock::new(server_state));

    // First call should use index 0
    let _result1 = handle_sequential_image(state.clone()).await.unwrap();

    // Check that index has incremented
    let current_index = state.read().await.current_index;
    assert_eq!(current_index, 1);

    // Second call should use index 1
    let _result2 = handle_sequential_image(state.clone()).await.unwrap();

    // Check that index wraps back to 0
    let current_index = state.read().await.current_index;
    assert_eq!(current_index, 0);
}
