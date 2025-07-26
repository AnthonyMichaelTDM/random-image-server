use std::{path::PathBuf, sync::Arc};

use pretty_assertions::assert_eq;
use random_image_server::{
    cache::{CacheKey, CacheValue},
    handle_random_image,
    state::ServerState,
};
use tokio::sync::RwLock;

#[tokio::test]
async fn test_handle_random_image_empty_cache() {
    let state = Arc::new(RwLock::new(ServerState::default()));
    let result = handle_random_image(state).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_handle_random_image_with_cache() {
    let mut server_state = ServerState::default();
    let key = CacheKey::ImagePath(PathBuf::from("/test/image.jpg"));
    let value = CacheValue {
        data: vec![1, 2, 3, 4],
        content_type: "image/jpeg".to_string(),
    };
    server_state.cache.set(key, value).unwrap();

    let state = Arc::new(RwLock::new(server_state));
    let result = handle_random_image(state).await;
    assert!(result.is_ok());

    let response = result.unwrap();
    assert_eq!(response.status(), hyper::StatusCode::OK);
}
