use std::{fs, path::PathBuf};

use pretty_assertions::assert_eq;
use random_image_server::read_image_from_path;
use tempdir::TempDir;

#[test]
fn test_read_image_from_path_success() {
    let temp_dir = TempDir::new("image_test").unwrap();
    let image_path = temp_dir.path().join("test.jpg");
    let test_data = vec![0xFF, 0xD8, 0xFF]; // JPEG header
    fs::write(&image_path, &test_data).unwrap();

    let result = read_image_from_path(&image_path);
    assert!(result.is_ok());

    let cache_value = result.unwrap();
    assert_eq!(cache_value.data, test_data);
    assert!(cache_value.content_type.contains("image/jpeg"));
}

#[test]
fn test_read_image_from_path_file_not_found() {
    let nonexistent_path = PathBuf::from("/nonexistent/image.jpg");
    let result = read_image_from_path(&nonexistent_path);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("does not exist"));
}

#[test]
fn test_read_image_from_path_no_extension() {
    let temp_dir = TempDir::new("image_test").unwrap();
    let image_path = temp_dir.path().join("test_no_ext");
    fs::write(&image_path, "test data").unwrap();

    let result = read_image_from_path(&image_path);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("no extension"));
}

#[test]
fn test_read_image_from_path_unsupported_extension() {
    let temp_dir = TempDir::new("image_test").unwrap();
    let image_path = temp_dir.path().join("test.txt");
    fs::write(&image_path, "test data").unwrap();

    let result = read_image_from_path(&image_path);
    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("Unsupported image file extension")
    );
}

#[test]
fn test_read_image_from_path_directory() {
    let temp_dir = TempDir::new("image_test").unwrap();

    let result = read_image_from_path(&temp_dir.path().to_path_buf());
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("does not exist"));
}

#[test]
fn test_read_image_from_path_different_extensions() {
    let temp_dir = TempDir::new("image_test").unwrap();
    let test_data = vec![1, 2, 3, 4];

    // Test .jpg
    let jpg_path = temp_dir.path().join("test.jpg");
    fs::write(&jpg_path, &test_data).unwrap();
    let result = read_image_from_path(&jpg_path);
    assert!(result.is_ok());

    // Test .jpeg
    let jpeg_path = temp_dir.path().join("test.jpeg");
    fs::write(&jpeg_path, &test_data).unwrap();
    let result = read_image_from_path(&jpeg_path);
    assert!(result.is_ok());

    // Test .png
    let png_path = temp_dir.path().join("test.png");
    fs::write(&png_path, &test_data).unwrap();
    let result = read_image_from_path(&png_path);
    assert!(result.is_ok());
}
