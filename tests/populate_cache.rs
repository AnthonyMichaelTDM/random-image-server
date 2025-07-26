use std::fs;

use pretty_assertions::assert_eq;
use random_image_server::{
    ImageServer,
    config::{Config, ImageSource},
};
use tempdir::TempDir;

#[tokio::test]
async fn test_image_server_populate_cache_no_sources() {
    let config = Config::default();
    let server = ImageServer::with_config(config);

    // Should not panic even with no sources
    server.populate_cache().await;

    assert_eq!(server.state.read().await.cache.size(), 0);
}

#[tokio::test]
async fn test_image_server_populate_cache_with_file() {
    let temp_dir = TempDir::new("image_test").unwrap();
    let image_path = temp_dir.path().join("test.jpg");
    let test_data = vec![0xFF, 0xD8, 0xFF]; // JPEG header
    fs::write(&image_path, &test_data).unwrap();

    let mut config = Config::default();
    config.server.sources = vec![ImageSource::Path(image_path)];

    let server = ImageServer::with_config(config);
    server.populate_cache().await;

    assert_eq!(server.state.read().await.cache.size(), 1);
}

#[tokio::test]
async fn test_image_server_populate_cache_with_directory() {
    let temp_dir = TempDir::new("image_test").unwrap();
    let image1_path = temp_dir.path().join("test1.jpg");
    let image2_path = temp_dir.path().join("test2.png");
    let text_file_path = temp_dir.path().join("readme.txt");

    fs::write(&image1_path, vec![0xFF, 0xD8, 0xFF]).unwrap();
    fs::write(&image2_path, vec![0x89, 0x50, 0x4E, 0x47]).unwrap();
    fs::write(&text_file_path, "not an image").unwrap();

    let mut config = Config::default();
    config.server.sources = vec![ImageSource::Path(temp_dir.path().to_path_buf())];

    let server = ImageServer::with_config(config);
    server.populate_cache().await;

    // Should only load image files, not text files
    assert_eq!(server.state.read().await.cache.size(), 2);
}

#[tokio::test]
async fn test_image_server_populate_cache_invalid_file() {
    let temp_dir = TempDir::new("image_test").unwrap();
    let text_path = temp_dir.path().join("test.txt");
    fs::write(&text_path, "not an image").unwrap();

    let mut config = Config::default();
    config.server.sources = vec![ImageSource::Path(text_path)];

    let server = ImageServer::with_config(config);
    server.populate_cache().await;

    // Should not load non-image files
    assert_eq!(server.state.read().await.cache.size(), 0);
}
