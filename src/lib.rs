use std::{convert::Infallible, fs, path::PathBuf, sync::Arc};

use anyhow::{Result, anyhow};
use http_body_util::Full;
use hyper::{Request, Response, body::Bytes, service::service_fn};
use hyper_util::{
    rt::{TokioExecutor, TokioIo},
    server::conn::auto,
};
use tokio::{
    net::TcpListener,
    sync::{RwLock, broadcast::Receiver},
};
use url::Url;

use crate::config::{Config, ImageSource};
use crate::state::ServerState;
use crate::termination::Interrupted;

pub mod cache;
pub mod config;
mod logging;
pub mod state;
pub use logging::init_logging;
pub mod termination;

pub const ALLOWED_IMAGE_EXTENSIONS: &[&str] = &["jpg", "jpeg", "png", "webp", "gif"];

/// The main server structure
pub struct ImageServer {
    pub config: Config,
    pub state: Arc<RwLock<ServerState>>,
}

impl ImageServer {
    /// Create a new `ImageServer` instance with default configuration
    #[must_use]
    pub fn new() -> Self {
        Self {
            config: Config::default(),
            state: Arc::new(RwLock::new(ServerState::default())),
        }
    }

    /// Create a new `ImageServer` instance with custom configuration
    #[must_use]
    pub fn with_config(config: Config) -> Self {
        Self {
            state: Arc::new(RwLock::new(ServerState::with_config(&config))),
            config,
        }
    }

    /// Populate the cache with the configured images
    ///
    /// # Errors
    ///
    /// Returns an error if the image file does not exist, is not a file, or has an unsupported extension.
    pub async fn populate_cache(&self) {
        // This method can be implemented to load images from configured sources
        // and populate the cache. For now, it is a placeholder.
        log::info!("Populating cache with configured images...");

        for source in &self.config.server.sources {
            match source {
                ImageSource::Url(url) => {
                    log::info!("Loading image from URL: {url}");
                    let key = cache::CacheKey::ImageUrl(url.clone());
                    // fetch the image from the URL and store it in the cache
                    match read_image_from_url(url).await {
                        Ok(image) => {
                            let set_result = self.state.write().await.cache.set(key, image);
                            if let Err(err) = set_result {
                                log::error!("Failed to store image in cache: {err}");
                            }
                        }
                        Err(e) => {
                            log::error!("Failed to read image from URL {url}: {e}");
                        }
                    }
                }
                ImageSource::Path(path) if path.is_file() => {
                    let path = path.canonicalize().unwrap_or_else(|_| {
                        log::warn!("Failed to canonicalize path: {}", path.display());
                        path.clone()
                    });
                    if path.extension().is_some_and(|ext| {
                        ALLOWED_IMAGE_EXTENSIONS.contains(&ext.to_string_lossy().as_ref())
                    }) {
                        log::info!("Loading image from file path: {}", path.display());
                        // read the image file from the path and store it in the cache
                        let Ok(image) = read_image_from_path(&path) else {
                            log::error!("Failed to read image file: {}", path.display());
                            continue;
                        };
                        let key = cache::CacheKey::ImagePath(path.clone());
                        let set_result = self.state.write().await.cache.set(key, image);
                        if let Err(err) = set_result {
                            log::error!("Failed to store image in cache: {err}");
                        }
                    } else {
                        log::warn!("Unsupported image file extension: {}", path.display());
                    }
                }
                ImageSource::Path(path) if path.is_dir() => {
                    let path = path.canonicalize().unwrap_or_else(|_| {
                        log::warn!("Failed to canonicalize path: {}", path.display());
                        path.clone()
                    });

                    log::info!("Loading images from directory: {}", path.display());
                    // Read all image files in the directory and store them in the cache
                    let mut state = self.state.write().await;
                    walkdir::WalkDir::new(&path)
                        .into_iter()
                        .filter_map(Result::ok)
                        .filter(|e| e.file_type().is_file())
                        .filter(|e| {
                            e.path()
                                .extension()
                                .and_then(|ext| ext.to_str())
                                .is_some_and(|ext| ALLOWED_IMAGE_EXTENSIONS.contains(&ext))
                        })
                        .for_each(|entry| {
                            let path = entry.path().to_path_buf();
                            log::info!("Loading image from file: {}", path.display());
                            // read the image file and store it in the cache
                            match read_image_from_path(&path) {
                                Ok(image) => {
                                    let key = cache::CacheKey::ImagePath(path.clone());
                                    let set_result = state.cache.set(key, image);
                                    if let Err(err) = set_result {
                                        log::error!("Failed to store image in cache: {err}");
                                    }
                                }
                                Err(e) => {
                                    log::error!(
                                        "Failed to read image from path {}: {e}",
                                        path.display(),
                                    );
                                }
                            }
                        });
                }
                ImageSource::Path(path) => {
                    log::warn!("Unsupported image path: {}", path.display());
                }
            }
        }
    }

    /// Start the server
    ///
    /// # Errors
    ///
    /// Returns an error if the server fails to start or encounters an unexpected error.
    pub async fn start(&self, mut interrupt_rx: Receiver<Interrupted>) -> Result<()> {
        let addr = self.config.socket_addr()?;
        let listener = TcpListener::bind(addr).await?;
        log::info!("Server running on http://{addr}");
        log::debug!("Configuration: {:?}", self.config);

        // Populate the cache with images from configured sources
        self.populate_cache().await;
        if self.state.read().await.cache.size() == 0 {
            log::error!("No images found in cache, please check your configuration");
            return Err(anyhow!(
                "No images found in cache, please check your configuration"
            ));
        }

        let executor = auto::Builder::new(TokioExecutor::new());

        loop {
            let (stream, _) = tokio::select! {
                stream = listener.accept() => stream?,
                _ = interrupt_rx.recv() => {
                    log::info!("Received termination signal, shutting down server");
                    break Ok(());
                }
            };

            let io = TokioIo::new(stream);

            // Clone state for the handler
            let state = self.state.clone();

            let service = service_fn(|req| {
                let value = state.clone();
                async move { handle_request(req, value).await }
            });

            // Spawn a new task to handle the connection
            if let Err(e) = executor.serve_connection(io, service).await {
                log::error!("Failed to serve connection: {e}");
            }
        }
    }
}

impl Default for ImageServer {
    fn default() -> Self {
        Self::new()
    }
}

/// Read an image file from the given path and return it as a `CacheValue`
///
/// # Errors
///
/// Returns an error if the file does not exist, is not a file, or has an unsupported extension.
pub fn read_image_from_path(path: &PathBuf) -> Result<cache::CacheValue> {
    if !path.exists() || !path.is_file() {
        return Err(anyhow!("Image file does not exist: {}", path.display()));
    }
    let Some(ext) = path.extension().and_then(|ext| ext.to_str()) else {
        return Err(anyhow!("Image file has no extension: {}", path.display()));
    };
    if !ALLOWED_IMAGE_EXTENSIONS.contains(&ext) {
        return Err(anyhow!(
            "Unsupported image file extension: {}",
            path.display()
        ));
    }

    let image_data = fs::read(path).map_err(|e| anyhow!("Failed to read image file: {}", e))?;
    let content_type = mime_guess::from_path(path)
        .first()
        .ok_or_else(|| {
            anyhow!(
                "Failed to determine content type for image file: {}",
                path.display()
            )
        })?
        .to_string();
    Ok(cache::CacheValue {
        data: image_data,
        content_type,
    })
}

/// Fetch an image from a URL and return it as a `CacheValue`
///
/// # Errors
///
/// Returns an error if the image cannot be fetched or if the content type is unsupported.
pub async fn read_image_from_url(url: &Url) -> Result<cache::CacheValue> {
    let response = reqwest::get(url.as_str())
        .await
        .map_err(|e| anyhow!("Failed to fetch image from URL: {}", e))?;

    if !response.status().is_success() {
        return Err(anyhow!(
            "Failed to fetch image, status: {}",
            response.status()
        ));
    }

    let content_type = response
        .headers()
        .get("Content-Type")
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| anyhow!("Failed to get Content-Type header from response"))?
        .to_string();

    if !ALLOWED_IMAGE_EXTENSIONS.contains(&content_type.split('/').next_back().unwrap_or("")) {
        return Err(anyhow!("Unsupported image content type: {}", content_type));
    }

    let data = response
        .bytes()
        .await
        .map_err(|e| anyhow!("Failed to read image bytes from response: {}", e))?;

    Ok(cache::CacheValue {
        data: data.to_vec(),
        content_type,
    })
}

/// Handle incoming HTTP requests
///
/// # Errors
///
/// should be Infallible
pub async fn handle_request(
    req: Request<hyper::body::Incoming>,
    state: Arc<RwLock<ServerState>>,
) -> Result<Response<Full<Bytes>>, Infallible> {
    match req.uri().path() {
        "/" => Ok(Response::new(Full::new(Bytes::from(
            "Welcome to the Random Image Server!",
        )))),
        "/health" => Ok(Response::new(Full::new(Bytes::from("OK")))),
        "/random" => match handle_random_image(state).await {
            Ok(response) => Ok(response),
            Err(err) => {
                log::error!("Failed to get random image: {err}");
                let mut not_found = Response::new(Full::new(Bytes::from("Not Found")));
                *not_found.status_mut() = hyper::StatusCode::NOT_FOUND;
                Ok(not_found)
            }
        },
        "/sequential" => match handle_sequential_image(state).await {
            Ok(response) => Ok(response),
            Err(err) => {
                log::error!("Failed to get sequential image: {err}");
                let mut not_found = Response::new(Full::new(Bytes::from("Not Found")));
                *not_found.status_mut() = hyper::StatusCode::NOT_FOUND;
                Ok(not_found)
            }
        },
        _ => {
            let mut not_found = Response::new(Full::new(Bytes::from("Not Found")));
            *not_found.status_mut() = hyper::StatusCode::NOT_FOUND;
            Ok(not_found)
        }
    }
}

/// Handle random image serving
///
/// # Errors
///
/// Returns an error if no images are configured or if the image cannot be found in the cache.
pub async fn handle_random_image(state: Arc<RwLock<ServerState>>) -> Result<Response<Full<Bytes>>> {
    let state = state.read().await;

    // get a random image from the cache
    state.cache.get_random().map_or_else(
        || {
            Err(anyhow!(
                "Failed to retrieve a random image, perhaps no images are configured"
            ))
        },
        |image| {
            let body = Full::new(Bytes::from(image.data));
            let mut response = Response::new(body);
            *response.status_mut() = hyper::StatusCode::OK;
            response
                .headers_mut()
                .insert(hyper::header::CONTENT_TYPE, image.content_type.parse()?);
            Ok(response)
        },
    )
}

/// Handle sequential image serving
///
/// # Errors
///
/// Returns an error if no images are configured or if the image cannot be found in the cache.
pub async fn handle_sequential_image(
    state: Arc<RwLock<ServerState>>,
) -> Result<Response<Full<Bytes>>> {
    let mut state = state.write().await;

    if state.cache.is_empty() {
        return Err(anyhow!("No image sources configured"));
    }

    let current_index = state.current_index % state.cache.size();
    let source = state.cache.keys()[current_index].clone();
    state.current_index = (current_index + 1) % state.cache.size();

    // Fetch the image from the cache or source
    if let Some(image) = state.cache.get(source.clone()) {
        let body = Full::new(Bytes::from(image.data));
        let mut response = Response::new(body);
        *response.status_mut() = hyper::StatusCode::OK;
        response
            .headers_mut()
            .insert(hyper::header::CONTENT_TYPE, image.content_type.parse()?);
        Ok(response)
    } else {
        state.cache.remove(&source);
        drop(state);
        Err(anyhow!("Image not found in cache"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::termination::create_termination;
    use pretty_assertions::assert_eq;
    use rstest::rstest;

    #[test]
    fn test_allowed_image_extensions() {
        assert!(ALLOWED_IMAGE_EXTENSIONS.contains(&"jpg"));
        assert!(ALLOWED_IMAGE_EXTENSIONS.contains(&"jpeg"));
        assert!(ALLOWED_IMAGE_EXTENSIONS.contains(&"png"));
        assert_eq!(ALLOWED_IMAGE_EXTENSIONS.len(), 3);
    }

    #[rstest]
    #[tokio::test]
    #[timeout(std::time::Duration::from_secs(2))]
    async fn test_start_stop_server() {
        let mut server = ImageServer::default();
        let port = 0;
        server.config.server.port = port;
        server.config.server.sources = vec![ImageSource::Path(PathBuf::from("assets"))];

        let (mut terminator, interrupt_rx) = create_termination();
        terminator.terminate(Interrupted::UserInt).unwrap();
        server.start(interrupt_rx).await.unwrap();
    }
}
