use std::convert::Infallible;
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;

use anyhow::{Result, anyhow};
use http_body_util::Full;
use hyper::body::Bytes;
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::{Request, Response};
use hyper_util::rt::TokioIo;
use tokio::net::TcpListener;
use tokio::sync::RwLock;
use tokio::sync::broadcast::Receiver;
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

pub const ALLOWED_IMAGE_EXTENSIONS: &[&str] = &["jpg", "jpeg", "png"];

/// The main server structure
pub struct ImageServer {
    config: Config,
    state: Arc<RwLock<ServerState>>,
}

impl ImageServer {
    /// Create a new ImageServer instance with default configuration
    pub fn new() -> Self {
        Self {
            config: Config::default(),
            state: Arc::new(RwLock::new(ServerState::default())),
        }
    }

    /// Create a new ImageServer instance with custom configuration
    pub fn with_config(config: Config) -> Self {
        Self {
            state: Arc::new(RwLock::new(ServerState::with_config(&config))),
            config,
        }
    }

    /// Populate the cache with the configured images
    pub async fn populate_cache(&self) {
        // This method can be implemented to load images from configured sources
        // and populate the cache. For now, it is a placeholder.
        log::info!("Populating cache with configured images...");

        for source in &self.config.server.sources {
            match source {
                ImageSource::Url(url) => {
                    log::info!("Loading image from URL: {}", url);
                    let key = cache::CacheKey::ImageUrl(url.clone());
                    // fetch the image from the URL and store it in the cache
                    match read_image_from_url(url).await {
                        Ok(image) => {
                            if let Err(err) = self.state.write().await.cache.set(key.clone(), image)
                            {
                                log::error!("Failed to store image in cache: {}", err);
                            }
                        }
                        Err(e) => {
                            log::error!("Failed to read image from URL {}: {}", url, e);
                        }
                    }
                }
                ImageSource::Path(path) if path.is_file() => {
                    let path = path.canonicalize().unwrap_or_else(|_| {
                        log::warn!("Failed to canonicalize path: {}", path.display());
                        return path.clone();
                    });
                    if path.extension().is_some_and(|ext| {
                        ALLOWED_IMAGE_EXTENSIONS.contains(&ext.to_string_lossy().as_ref())
                    }) {
                        log::info!("Loading image from file path: {}", path.display());
                        // read the image file from the path and store it in the cache
                        let image = read_image_from_path(&path).expect("Failed to read image file");
                        let key = cache::CacheKey::ImagePath(path.clone());
                        if let Err(err) = self.state.write().await.cache.set(key, image) {
                            log::error!("Failed to store image in cache: {}", err);
                        }
                    } else {
                        log::warn!("Unsupported image file extension: {}", path.display());
                        continue;
                    }
                }
                ImageSource::Path(path) if path.is_dir() => {
                    let path = path.canonicalize().unwrap_or_else(|_| {
                        log::warn!("Failed to canonicalize path: {}", path.display());
                        return path.clone();
                    });

                    log::info!("Loading images from directory: {}", path.display());
                    // Read all image files in the directory and store them in the cache
                    let walk = walkdir::WalkDir::new(&path)
                        .into_iter()
                        .filter_map(Result::ok)
                        .filter(|e| e.file_type().is_file())
                        .filter(|e| {
                            e.path()
                                .extension()
                                .and_then(|ext| ext.to_str())
                                .map_or(false, |ext| ALLOWED_IMAGE_EXTENSIONS.contains(&ext))
                        });
                    for path in walk {
                        log::info!("Loading image from file: {}", path.path().display());
                        let path = path.path().to_path_buf();
                        // read the image file and store it in the cache
                        match read_image_from_path(&path) {
                            Ok(image) => {
                                let key = cache::CacheKey::ImagePath(path.clone());
                                if let Err(err) = self.state.write().await.cache.set(key, image) {
                                    log::error!("Failed to store image in cache: {}", err);
                                }
                            }
                            Err(e) => {
                                log::error!(
                                    "Failed to read image from path {}: {}",
                                    path.display(),
                                    e
                                );
                            }
                        }
                    }
                }
                _ => {
                    log::warn!("Unsupported image source: {:?}", source);
                }
            }
        }
    }

    /// Start the server
    pub async fn start(&self, mut interrupt_rx: Receiver<Interrupted>) -> Result<()> {
        let addr = self.config.socket_addr()?;
        let listener = TcpListener::bind(addr).await?;
        log::info!("Server running on http://{}", addr);
        log::debug!("Configuration: {:?}", self.config);

        // Populate the cache with images from configured sources
        self.populate_cache().await;
        if self.state.read().await.cache.size() == 0 {
            log::error!("No images found in cache, please check your configuration");
            return Err(anyhow!(
                "No images found in cache, please check your configuration"
            ));
        }

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

            tokio::task::spawn(async move {
                if let Err(err) = http1::Builder::new()
                    .serve_connection(
                        io,
                        service_fn(async |req| handle_request(req, state.clone()).await),
                    )
                    .await
                {
                    eprintln!("Error serving connection: {:?}", err);
                }
            });
        }
    }
}

impl Default for ImageServer {
    fn default() -> Self {
        Self::new()
    }
}

pub fn read_image_from_path(path: &PathBuf) -> Result<cache::CacheValue> {
    if !path.exists() || !path.is_file() {
        return Err(anyhow!("Image file does not exist: {}", path.display()));
    }
    let ext = path.extension().and_then(|ext| ext.to_str());
    if ext.is_none() || !ALLOWED_IMAGE_EXTENSIONS.contains(&ext.unwrap()) {
        return Err(anyhow!(
            "Unsupported image file extension: {}",
            path.display()
        ));
    }

    let image_data = fs::read(path).map_err(|e| anyhow!("Failed to read image file: {}", e))?;
    let content_type = mime_guess::from_path(path)
        .first_or_octet_stream()
        .to_string();
    Ok(cache::CacheValue {
        data: image_data,
        content_type,
    })
}

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
        .ok_or(anyhow!("Failed to get Content-Type header from response"))?
        .to_string();

    if !ALLOWED_IMAGE_EXTENSIONS.contains(&content_type.split('/').last().unwrap_or("")) {
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
async fn handle_request(
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
                log::error!("Failed to get random image: {}", err);
                let mut not_found = Response::new(Full::new(Bytes::from("Not Found")));
                *not_found.status_mut() = hyper::StatusCode::NOT_FOUND;
                Ok(not_found)
            }
        },
        "/sequential" => match handle_sequential_image(state).await {
            Ok(response) => Ok(response),
            Err(err) => {
                log::error!("Failed to get sequential image: {}", err);
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

async fn handle_random_image(state: Arc<RwLock<ServerState>>) -> Result<Response<Full<Bytes>>> {
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

async fn handle_sequential_image(state: Arc<RwLock<ServerState>>) -> Result<Response<Full<Bytes>>> {
    let mut state = state.write().await;

    if state.cache.is_empty() {
        return Err(anyhow!("No image sources configured"));
    }

    let current_index = state.current_index % state.cache.size();
    let source = state.cache.keys()[current_index].clone();
    state.current_index = (current_index + 1) % state.cache.size();

    // Fetch the image from the cache or source
    match state.cache.get(source.clone()) {
        Some(image) => {
            let body = Full::new(Bytes::from(image.data));
            let mut response = Response::new(body);
            *response.status_mut() = hyper::StatusCode::OK;
            response
                .headers_mut()
                .insert(hyper::header::CONTENT_TYPE, image.content_type.parse()?);
            Ok(response)
        }
        None => {
            state.cache.remove(&source);
            Err(anyhow!("Image not found in cache"))
        }
    }
}
