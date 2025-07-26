use std::fs;
use std::path::PathBuf;

use log::LevelFilter;
use pretty_assertions::{assert_eq, assert_str_eq};
use random_image_server::{
    config::{CacheBackendType, CacheConfig, Config, ImageSource, ServerConfig},
    env::{EnvBackend, MockEnvBackend},
};
use rstest::rstest;
use tempdir::TempDir;
use url::Url;

#[test]
fn test_deserialize_config() {
    let config_toml = r#"
            [server]
            port = 8080
            host = "localhost"
            log_level = "info"
            sources = ["./assets/blank.jpg", "https://images.unsplash.com/photo-1502790671504-542ad42d5189?auto=format&fit=crop&w=2560&q=80"]

            [cache]
            backend = "file_system"
        "#;
    let config: Config = toml::from_str(config_toml).expect("Failed to parse config");
    assert_eq!(config.server.port, 8080);
    assert_eq!(
        config.server.host,
        url::Host::Domain("localhost".to_string())
    );
    assert_eq!(config.server.sources.len(), 2);
    assert!(matches!(config.server.sources[0], ImageSource::Path(_)));
    assert!(matches!(config.server.sources[1], ImageSource::Url(_)));

    assert_eq!(config.cache.backend, CacheBackendType::FileSystem);
}

#[test]
fn test_config_default() {
    let config = Config::default();
    assert_eq!(config.server.port, 3000);
    assert_eq!(
        config.server.host,
        url::Host::<String>::Ipv4([127, 0, 0, 1].into())
    );
    assert!(matches!(config.server.log_level, LevelFilter::Info));
    assert!(config.server.sources.is_empty());
    assert_eq!(config.cache.backend, CacheBackendType::InMemory);
}

#[test]
fn test_socket_addr() {
    let config = Config {
        server: ServerConfig {
            port: 8080,
            host: url::Host::Domain("127.0.0.1".to_string()),
            ..Config::default().server
        },
        ..Config::default()
    };
    let addr = config.socket_addr().unwrap();
    assert_eq!(addr.port(), 8080);
    assert_eq!(addr.ip().to_string(), "127.0.0.1");
}

#[rstest]
#[case::full(
    "[server]\nport = 9090\nhost = \"0.0.0.0\"\nlog_level = \"debug\"\nsources = [\"./assets/blank.jpg\"]\n[cache]\nbackend = \"file_system\"", 
    Config {
        server: ServerConfig {
            port: 9090,
            host: url::Host::Ipv4(std::net::Ipv4Addr::new(0, 0, 0, 0)),
            log_level: LevelFilter::Debug,
            sources: vec![ImageSource::Path(PathBuf::from("./assets/blank.jpg").canonicalize().unwrap())],
        },
        cache: CacheConfig {
            backend: CacheBackendType::FileSystem,
        },
    }
)]
#[case::minimal(
    "[server]\nsources = [\"https://example.com/image.jpg\"]",
    Config {
        server: ServerConfig {
            sources: vec![ImageSource::Url(Url::parse("https://example.com/image.jpg").unwrap())],
            ..ServerConfig::default()
        },
        ..Config::default()
    }
)]
fn test_from_file(#[case] content: &str, #[case] expected: Config) {
    let temp_dir = TempDir::new("config_test").unwrap();
    let config_path = temp_dir.path().join("test.toml");

    fs::write(&config_path, content).unwrap();

    let config = Config::from_file(config_path.to_str().unwrap()).unwrap();
    assert_eq!(config, expected);
}

#[test]
fn test_from_file_not_found() {
    let result = Config::from_file("nonexistent.toml");
    assert!(result.is_err());
}

#[test]
fn test_from_file_invalid_toml() {
    let temp_dir = TempDir::new("config_test").unwrap();
    let config_path = temp_dir.path().join("invalid.toml");

    fs::write(&config_path, "invalid toml content [[[").unwrap();

    let result = Config::from_file(config_path.to_str().unwrap());
    assert!(result.is_err());
}

#[rstest]
#[case("trace", LevelFilter::Trace)]
#[case("debug", LevelFilter::Debug)]
#[case("info", LevelFilter::Info)]
#[case("warn", LevelFilter::Warn)]
#[case("error", LevelFilter::Error)]
#[case("off", LevelFilter::Off)]
fn test_log_level_deserialization(#[case] level: &str, #[case] expected: LevelFilter) {
    let trace_toml = &format!(
        r#"
            [server]
            port = 3000
            host = "localhost"
            log_level = "{level}"
            sources = [
                "https://example.com/image.jpg",
            ]
            "#
    );
    let config: Config = toml::from_str(trace_toml).unwrap();
    assert_eq!(config.server.log_level, expected);
}

#[rstest]
#[case("in_memory", CacheBackendType::InMemory)]
#[case("file_system", CacheBackendType::FileSystem)]
fn test_cache_backend_deserialization(#[case] backend: &str, #[case] expected: CacheBackendType) {
    let in_memory_toml = &format!(
        r#"
            [server]
            port = 3000
            host = "localhost"
            log_level = "info"
            sources = [
                "https://example.com/image.jpg",
            ]

            [cache]
            backend = "{backend}"
            "#
    );
    let config: Config = toml::from_str(in_memory_toml).unwrap();
    assert_eq!(config.cache.backend, expected);
}

#[test]
fn test_sources_deserialization_path() {
    // Create a temporary file for testing
    let temp_dir = TempDir::new("config_test").unwrap();
    let test_file = temp_dir.path().join("test.jpg");
    fs::write(&test_file, "fake image content").unwrap();

    let test_file = test_file.display();
    let config_toml = format!(
        r#"
            [server]
            port = 3000
            host = "localhost"
            log_level = "info"
            sources = ["{test_file}"]
            "#
    );

    let config: Config = toml::from_str(&config_toml).unwrap();
    assert_eq!(config.server.sources.len(), 1);
    assert!(matches!(config.server.sources[0], ImageSource::Path(_)));
}

#[rstest]
#[case::path(r#"["./assets/blank.jpg"]"#, Ok(vec![ImageSource::Path(PathBuf::from("./assets/blank.jpg").canonicalize().unwrap())]))]
#[case::url(r#"["https://example.com/image.jpg"]"#, Ok(vec![ImageSource::Url(Url::parse("https://example.com/image.jpg").unwrap())]))]
#[case::empty(r#"[]"#, Err("No valid image sources found"))]
#[case::invalid(
    r#"["/nonexistent/path.jpg", "not-a-url"]"#,
    Err("No valid image sources found")
)]
#[case::wrong_type(
    r#""not-a-list""#,
    Err("invalid type: string \"not-a-list\", expected a sequence")
)]
fn test_sources_deserialization(
    #[case] sources: &str,
    #[case] expected: Result<Vec<ImageSource>, &str>,
) {
    let config_toml = format!(
        r#"
            [server]
            port = 3000
            host = "localhost"
            log_level = "info"
            sources = {sources}
            "#,
    );

    match (toml::from_str::<Config>(&config_toml), expected) {
        (Ok(config), Ok(expected)) => {
            assert_eq!(config.server.sources, expected);
        }
        (Ok(_), Err(e)) => panic!("Expected an error but got a valid config, expected: {e:?}"),
        (Err(e), Ok(_)) => panic!("Failed to parse config when it should succeed: {e}"),
        (Err(err), Err(message)) => {
            assert_str_eq!(err.message(), message);
        }
    }
}

#[test]
fn test_invalid_host_deserialization() {
    let config_toml = r#"
            [server]
            port = 3000
            host = "invalid host with spaces"
            log_level = "info"
            sources = ["https://example.com/image.jpg"]
        "#;

    let result: Result<Config, _> = toml::from_str(config_toml);
    assert!(result.is_err());
}

#[rstest]
#[case::none(&[] as &[(&str, &str)], Config::default())]
#[case::port(&[("RANDOM_IMAGE_SERVER_PORT", "8080")], Config {
        server: ServerConfig {
            port: 8080,
            ..Config::default().server
        },
        ..Config::default()
    })]
#[case::host(&[("RANDOM_IMAGE_SERVER_HOST", "example.com")], Config {
        server: ServerConfig {
            host: url::Host::Domain("example.com".to_string()),
            ..Config::default().server
        },
        ..Config::default()
    })]
#[case::log_level(&[("RANDOM_IMAGE_SERVER_LOG_LEVEL", "debug")], Config {
        server: ServerConfig {
            log_level: LevelFilter::Debug,
            ..Config::default().server
        },
        ..Config::default()
    })]
#[case::sources(&[("RANDOM_IMAGE_SERVER_SOURCES", "https://example.com/image.jpg,./assets/blank.jpg")], Config {
        server: ServerConfig {
            sources: vec![
                ImageSource::Url(Url::parse("https://example.com/image.jpg").unwrap()),
                ImageSource::Path(PathBuf::from("./assets/blank.jpg").canonicalize().unwrap()),
            ],
            ..Config::default().server
        },
        ..Config::default()
    })]
#[case::cache_backend(&[("RANDOM_IMAGE_SERVER_CACHE_BACKEND", "file_system")], Config {
        cache: CacheConfig {
            backend: CacheBackendType::FileSystem,
        },
        ..Config::default()
    })]
#[case::all(
        &[
            ("RANDOM_IMAGE_SERVER_PORT", "8080"),
            ("RANDOM_IMAGE_SERVER_HOST", "example.com"),
            ("RANDOM_IMAGE_SERVER_LOG_LEVEL", "debug"),
            ("RANDOM_IMAGE_SERVER_SOURCES", "https://example.com/image.jpg,./assets/blank.jpg"),
            ("RANDOM_IMAGE_SERVER_CACHE_BACKEND", "file_system")
        ],
        Config {
            server: ServerConfig {
                port: 8080,
                host: url::Host::Domain("example.com".to_string()),
                log_level: LevelFilter::Debug,
                sources: vec![
                    ImageSource::Url(Url::parse("https://example.com/image.jpg").unwrap()),
                    ImageSource::Path(PathBuf::from("./assets/blank.jpg").canonicalize().unwrap()),
                ],
            },
            cache: CacheConfig {
                backend: CacheBackendType::FileSystem,
            },
        }
    )]
fn test_update_config_from_env(#[case] env_vars: &[(&str, &str)], #[case] expected: Config) {
    let mut mock_env = MockEnvBackend::default();

    // Set environment variables
    for (key, value) in env_vars {
        mock_env.set_var(key, value);
    }

    let config = Config::default().with_env_backend(&mock_env).unwrap();

    assert_eq!(config, expected);
}
