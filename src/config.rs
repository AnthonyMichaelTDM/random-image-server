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
                host: url::Host::Domain("localhost".to_string()),
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
}
