use std::{net::SocketAddr, path::PathBuf, str::FromStr};

use anyhow::{Result, anyhow};
use log::LevelFilter;
use serde::Deserialize;
use url::Url;

/// Configuration structure for the server
#[derive(Debug, Deserialize, Clone, PartialEq, Eq)]
pub struct Config {
    pub server: ServerConfig,
    #[serde(default)]
    pub cache: CacheConfig,
}

#[derive(Debug, Deserialize, Clone, PartialEq, Eq)]
pub struct ServerConfig {
    pub port: u16,
    #[serde(deserialize_with = "deserialize_host")]
    pub host: url::Host,
    #[serde(deserialize_with = "deserialize_log_level")]
    pub log_level: LevelFilter,
    #[serde(deserialize_with = "deserialize_sources")]
    pub sources: Vec<ImageSource>,
}

fn deserialize_host<'de, D>(deserializer: D) -> Result<url::Host, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s: String = Deserialize::deserialize(deserializer)?;
    url::Host::parse(&s).map_err(serde::de::Error::custom)
}

fn deserialize_log_level<'de, D>(deserializer: D) -> Result<LevelFilter, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let level: String = Deserialize::deserialize(deserializer)?;
    LevelFilter::from_str(&level).map_err(serde::de::Error::custom)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ImageSource {
    Url(Url),
    Path(PathBuf),
}

#[derive(Debug, Default, Deserialize, Clone, Copy, PartialEq, Eq)]
pub struct CacheConfig {
    pub backend: CacheBackendType,
}

#[derive(Debug, Default, Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CacheBackendType {
    #[default]
    InMemory,
    FileSystem,
}

impl FromStr for ImageSource {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if let Ok(url) = Url::parse(s) {
            Ok(Self::Url(url))
        } else if PathBuf::from(s).exists() {
            Ok(Self::Path(PathBuf::from(s).canonicalize()?))
        } else {
            Err(anyhow!(
                "Image source doesn't exist or couldn't be parsed as a URL: {s}"
            ))
        }
    }
}

impl FromStr for CacheBackendType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "in_memory" => Ok(Self::InMemory),
            "file_system" => Ok(Self::FileSystem),
            _ => Err(format!("Unknown cache backend type: {s}")),
        }
    }
}

fn deserialize_sources<'de, D>(deserializer: D) -> Result<Vec<ImageSource>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let sources: Vec<String> = Deserialize::deserialize(deserializer)?;
    let mut image_sources = Vec::new();

    for source in sources {
        match ImageSource::from_str(&source) {
            Ok(image_source) => image_sources.push(image_source),
            Err(e) => log::warn!("Invalid image source '{source}': {e}"),
        }
    }

    if image_sources.is_empty() {
        return Err(serde::de::Error::custom("No valid image sources found"));
    }

    Ok(image_sources)
}

impl Default for Config {
    fn default() -> Self {
        Self {
            server: ServerConfig {
                port: 3000,
                host: url::Host::Ipv4([127, 0, 0, 1].into()),
                log_level: LevelFilter::Info,
                sources: vec![],
            },
            cache: CacheConfig::default(),
        }
    }
}

impl Config {
    /// Load configuration from a TOML file
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be read or parsed.
    pub fn from_file(path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let content = std::fs::read_to_string(path)?;
        let config: Self = toml::from_str(&content)?;
        Ok(config)
    }

    /// Create a new configuration, with it's values updated from environment variables
    ///
    /// This function reads environment variables prefixed with `RANDOM_IMAGE_SERVER_`
    /// and updates the configuration accordingly. It supports the following variables:
    /// - `RANDOM_IMAGE_SERVER_PORT`: The port for the server
    /// - `RANDOM_IMAGE_SERVER_HOST`: The host for the server
    /// - `RANDOM_IMAGE_SERVER_LOG_LEVEL`: The log level for the server
    /// - `RANDOM_IMAGE_SERVER_SOURCES`: A comma-separated list of image sources (URLs or paths)
    /// - `RANDOM_IMAGE_SERVER_CACHE_BACKEND`: The cache backend type, either `in_memory` or `file_system`
    ///
    /// # Errors
    ///
    /// Returns an error if any environment variable is invalid or cannot be parsed.
    pub fn with_env(mut self) -> Result<Self> {
        macro_rules! set_from_env {
            ($field:expr, $var:literal,  $parse_fn:expr) => {
                if let Ok(value) = std::env::var(concat!("RANDOM_IMAGE_SERVER_", $var)) {
                    $field = $parse_fn(&value).map_err(|e| anyhow!(e))?;
                }
            };
        }

        set_from_env!(self.server.port, "PORT", u16::from_str);
        set_from_env!(self.server.host, "HOST", url::Host::parse);
        set_from_env!(self.server.log_level, "LOG_LEVEL", LevelFilter::from_str);
        set_from_env!(self.server.sources, "SOURCES", |s: &str| {
            s.split(',')
                .map(ImageSource::from_str)
                .collect::<Result<Vec<_>, _>>()
                .and_then(|sources| {
                    if sources.is_empty() {
                        Err(anyhow!("No valid image sources found"))
                    } else {
                        Ok(sources)
                    }
                })
        });
        set_from_env!(
            self.cache.backend,
            "CACHE_BACKEND",
            CacheBackendType::from_str
        );

        Ok(self)
    }

    /// Get the socket address for the server
    ///
    /// # Errors
    ///
    /// Shouldn't fail unless the host or port is invalid.
    pub fn socket_addr(&self) -> Result<SocketAddr, std::net::AddrParseError> {
        format!("{}:{}", self.server.host, self.server.port).parse()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::{assert_eq, assert_str_eq};
    use rstest::rstest;
    use std::fs;
    use tempdir::TempDir;

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

    #[test]
    fn test_from_file_success() {
        let temp_dir = TempDir::new("config_test").unwrap();
        let config_path = temp_dir.path().join("test.toml");

        let config_content = r#"
            [server]
            port = 9090
            host = "0.0.0.0"
            log_level = "debug"
            sources = [
                "./assets/blank.jpg",
            ]

            [cache]
            backend = "in_memory"
        "#;

        fs::write(&config_path, config_content).unwrap();

        let config = Config::from_file(config_path.to_str().unwrap()).unwrap();
        assert_eq!(config.server.port, 9090);
        assert_eq!(config.server.host.to_string(), "0.0.0.0");
        assert!(matches!(config.server.log_level, LevelFilter::Debug));
        assert_eq!(config.cache.backend, CacheBackendType::InMemory);
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
    fn test_cache_backend_deserialization(
        #[case] backend: &str,
        #[case] expected: CacheBackendType,
    ) {
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
    #[rstest]
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
        // Set environment variables
        for (key, value) in env_vars {
            unsafe { std::env::set_var(key, value) };
        }

        let config = Config::default().with_env().unwrap();

        assert_eq!(config, expected);

        // Clean up environment variables
        for (key, _) in env_vars {
            unsafe { std::env::remove_var(key) };
        }
    }
}
