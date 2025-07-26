use std::{net::SocketAddr, path::PathBuf};

use serde::Deserialize;
use url::Url;

/// Configuration structure for the server
#[derive(Debug, Deserialize)]
pub struct Config {
    pub server: ServerConfig,
    #[serde(default)]
    pub cache: CacheConfig,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ServerConfig {
    pub port: u16,
    #[serde(deserialize_with = "deserialize_host")]
    pub host: url::Host,
    pub log_level: LogLevel,
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

#[derive(Debug, Deserialize, Clone, Copy)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
    Off,
}

#[derive(Debug, Clone)]
pub enum ImageSource {
    Url(Url),
    Path(PathBuf),
}

#[derive(Debug, Default, Deserialize, Clone)]
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

fn deserialize_sources<'de, D>(deserializer: D) -> Result<Vec<ImageSource>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let sources: Vec<String> = Deserialize::deserialize(deserializer)?;
    let mut image_sources = Vec::new();

    for source in sources {
        if let Ok(url) = Url::parse(&source) {
            image_sources.push(ImageSource::Url(url));
        } else if PathBuf::from(&source).exists() {
            image_sources.push(ImageSource::Path(
                PathBuf::from(source)
                    .canonicalize()
                    .map_err(serde::de::Error::custom)?,
            ));
        } else {
            log::warn!("Unsupported image source: {source}");
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
                log_level: LogLevel::Info,
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
        assert!(matches!(config.server.log_level, LogLevel::Info));
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
        assert!(matches!(config.server.log_level, LogLevel::Debug));
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

    #[test]
    fn test_log_level_deserialization() {
        let trace_toml = r#"
            [server]
            port = 3000
            host = "localhost"
            log_level = "trace"
            sources = [
                "https://example.com/image.jpg",
            ]
        "#;
        let config: Config = toml::from_str(trace_toml).unwrap();
        assert!(matches!(config.server.log_level, LogLevel::Trace));

        let error_toml = r#"
            [server]
            port = 3000
            host = "localhost"
            log_level = "error"
            sources = [
                "https://example.com/image.jpg",
            ]
        "#;
        let config: Config = toml::from_str(error_toml).unwrap();
        assert!(matches!(config.server.log_level, LogLevel::Error));

        let off_toml = r#"
            [server]
            port = 3000
            host = "localhost"
            log_level = "off"
            sources = [
                "https://example.com/image.jpg",
            ]
        "#;
        let config: Config = toml::from_str(off_toml).unwrap();
        assert!(matches!(config.server.log_level, LogLevel::Off));
    }

    #[test]
    fn test_cache_backend_deserialization() {
        let in_memory_toml = r#"
            [server]
            port = 3000
            host = "localhost"
            log_level = "info"
            sources = [
                "https://example.com/image.jpg",
            ]

            [cache]
            backend = "in_memory"
        "#;
        let config: Config = toml::from_str(in_memory_toml).unwrap();
        assert_eq!(config.cache.backend, CacheBackendType::InMemory);

        let file_system_toml = r#"
            [server]
            port = 3000
            host = "localhost"
            log_level = "info"
            sources = [
                "https://example.com/image.jpg",
            ]

            [cache]
            backend = "file_system"
        "#;
        let config: Config = toml::from_str(file_system_toml).unwrap();
        assert_eq!(config.cache.backend, CacheBackendType::FileSystem);
    }

    #[test]
    fn test_sources_deserialization_with_existing_path() {
        // Create a temporary file for testing
        let temp_dir = TempDir::new("config_test").unwrap();
        let test_file = temp_dir.path().join("test.jpg");
        fs::write(&test_file, "fake image content").unwrap();

        let config_toml = format!(
            r#"
            [server]
            port = 3000
            host = "localhost"
            log_level = "info"
            sources = ["{}"]
            "#,
            test_file.to_str().unwrap()
        );

        let config: Config = toml::from_str(&config_toml).unwrap();
        assert_eq!(config.server.sources.len(), 1);
        assert!(matches!(config.server.sources[0], ImageSource::Path(_)));
    }

    #[test]
    fn test_sources_deserialization_with_url() {
        let config_toml = r#"
            [server]
            port = 3000
            host = "localhost"
            log_level = "info"
            sources = ["https://example.com/image.jpg"]
        "#;

        let config: Config = toml::from_str(config_toml).unwrap();
        assert_eq!(config.server.sources.len(), 1);
        assert!(matches!(config.server.sources[0], ImageSource::Url(_)));
    }

    #[test]
    fn test_sources_deserialization_empty_sources_error() {
        let config_toml = r#"
            [server]
            port = 3000
            host = "localhost"
            log_level = "info"
            sources = []
        "#;

        let result: Result<Config, _> = toml::from_str(config_toml);
        assert!(result.is_err());
    }

    #[test]
    fn test_sources_deserialization_invalid_sources() {
        let config_toml = r#"
            [server]
            port = 3000
            host = "localhost"
            log_level = "info"
            sources = ["/nonexistent/path.jpg", "not-a-url"]
        "#;

        let result: Result<Config, _> = toml::from_str(config_toml);
        assert!(result.is_err());
    }

    #[test]
    fn test_image_source_debug() {
        let url = Url::parse("https://example.com/image.jpg").unwrap();
        let path = PathBuf::from("/path/to/image.jpg");

        let url_source = ImageSource::Url(url);
        let path_source = ImageSource::Path(path);

        let url_debug = format!("{:?}", url_source);
        let path_debug = format!("{:?}", path_source);

        assert!(url_debug.contains("Url"));
        assert!(path_debug.contains("Path"));
    }

    #[test]
    fn test_config_debug() {
        let config = Config::default();
        let debug_output = format!("{:?}", config);
        assert!(debug_output.contains("Config"));
        assert!(debug_output.contains("server"));
        assert!(debug_output.contains("cache"));
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
}
