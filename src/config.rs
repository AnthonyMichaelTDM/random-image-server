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
