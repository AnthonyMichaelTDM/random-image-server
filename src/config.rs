use std::{
    net::{Ipv4Addr, SocketAddr},
    path::PathBuf,
    str::FromStr,
};

use anyhow::{Result, anyhow};
use serde::Deserialize;
use tracing::Level;
use url::Url;

const DEFAULT_PORT: u16 = 3000;
const DEFAULT_HOST: url::Host = url::Host::Ipv4(Ipv4Addr::LOCALHOST);
const DEFAULT_LOG_LEVEL: Level = Level::INFO;

/// Configuration structure for the server
#[derive(Debug, Default, Deserialize, Clone, PartialEq, Eq)]
pub struct Config {
    pub server: ServerConfig,
    #[serde(default)]
    pub cache: CacheConfig,
}

#[derive(Debug, Deserialize, Clone, PartialEq, Eq)]
pub struct ServerConfig {
    #[serde(default = "default_port")]
    pub port: u16,
    #[serde(deserialize_with = "deserialize_host", default = "default_host")]
    pub host: url::Host,
    #[serde(
        deserialize_with = "deserialize_log_level",
        default = "default_log_level"
    )]
    pub log_level: Level,
    #[serde(deserialize_with = "deserialize_sources")]
    pub sources: Vec<ImageSource>,
}

const fn default_port() -> u16 {
    DEFAULT_PORT
}
const fn default_host() -> url::Host {
    DEFAULT_HOST
}
const fn default_log_level() -> Level {
    DEFAULT_LOG_LEVEL
}

fn deserialize_host<'de, D>(deserializer: D) -> Result<url::Host, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s: String = Deserialize::deserialize(deserializer)?;
    url::Host::parse(&s).map_err(serde::de::Error::custom)
}

fn deserialize_log_level<'de, D>(deserializer: D) -> Result<Level, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let level: String = Deserialize::deserialize(deserializer)?;
    Level::from_str(&level).map_err(serde::de::Error::custom)
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
            Err(e) => tracing::warn!("Invalid image source '{source}': {e}"),
        }
    }

    if image_sources.is_empty() {
        return Err(serde::de::Error::custom("No valid image sources found"));
    }

    Ok(image_sources)
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            port: DEFAULT_PORT,
            host: DEFAULT_HOST,
            log_level: DEFAULT_LOG_LEVEL,
            sources: vec![],
        }
    }
}

impl Config {
    /// Load configuration from a TOML file
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be read or parsed.
    pub fn from_file(path: &str) -> Result<Self> {
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
    pub fn with_env(self) -> Result<Self> {
        self.with_env_backend(&crate::env::StdEnvBackend)
    }

    /// Create a new configuration, with it's values updated from environment variables.
    ///
    /// Same as `with_env`, but allows passing a custom environment backend (e.g., for testing).
    ///
    /// # Errors
    ///
    /// Returns an error if any environment variable is invalid or cannot be parsed.
    pub fn with_env_backend(mut self, env: &impl crate::env::EnvBackend) -> Result<Self> {
        macro_rules! set_from_env {
            ($field:expr, $var:literal,  $parse_fn:expr) => {
                if let Ok(value) = env.var(concat!("RANDOM_IMAGE_SERVER_", $var)) {
                    $field = $parse_fn(&value).map_err(|e| {
                        anyhow!("Failed to parse environment variable '{}': {}", $var, e)
                    })?
                }
            };
        }

        set_from_env!(self.server.port, "PORT", u16::from_str);
        set_from_env!(self.server.host, "HOST", url::Host::parse);
        set_from_env!(self.server.log_level, "LOG_LEVEL", Level::from_str);
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
