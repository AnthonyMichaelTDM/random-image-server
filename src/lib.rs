use std::convert::Infallible;
use std::sync::Arc;

use anyhow::{Result, anyhow};
use http_body_util::Full;
use hyper::body::Bytes;
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::{Request, Response};
use hyper_util::rt::TokioIo;
use tokio::net::TcpListener;

use crate::config::{Config, ImageSource};
use crate::state::ServerState;

pub mod cache;
pub mod config;
pub mod state;

pub const ALLOWED_IMAGE_EXTENSIONS: &[&str] = &["jpg", "jpeg", "png"];

/// The main server structure
pub struct ImageServer {
    config: Config,
    state: Arc<ServerState>,
}

impl ImageServer {
    /// Create a new ImageServer instance with default configuration
    pub fn new() -> Self {
        Self {
            config: Config::default(),
            state: Arc::new(ServerState::default()),
        }
    }

    /// Create a new ImageServer instance with custom configuration
    pub fn with_config(config: Config) -> Self {
        Self {
            config,
            state: Arc::new(ServerState::default()),
        }
    }

    /// Populate the cache with the configured images
    pub fn populate_cache(&self) {
        // This method can be implemented to load images from configured sources
        // and populate the cache. For now, it is a placeholder.
        log::info!("Populating cache with configured images...");

        for source in &self.config.server.sources {
            match source {
                ImageSource::Url(url) => {
                    log::info!("Loading image from URL: {}", url);
                    // Here you would fetch the image from the URL and store it in the cache
                }
                ImageSource::Path(path) if path.is_file() => {
                    if path.extension().is_some_and(|ext| {
                        ALLOWED_IMAGE_EXTENSIONS.contains(&ext.to_string_lossy().as_ref())
                    }) {
                        log::info!("Loading image from file path: {}", path.display());
                    // Here you would read the image file from the path and store it in the cache
                    } else {
                        log::warn!("Unsupported image file extension: {}", path.display());
                        continue;
                    }
                }
                ImageSource::Path(path) if path.is_dir() => {
                    log::info!("Loading images from directory: {}", path.display());
                    // Here you would read all image files in the directory and store them in the cache
                }
                _ => {
                    log::warn!("Unsupported image source: {:?}", source);
                }
            }
        }
    }

    /// Start the server
    pub async fn start(&self) -> Result<()> {
        let addr = self.config.socket_addr()?;
        let listener = TcpListener::bind(addr).await?;
        println!("Server running on http://{}", addr);
        println!("Configuration: {:?}", self.config);

        // Populate the cache with images from configured sources
        self.populate_cache();

        loop {
            let (stream, _) = listener.accept().await?;
            let io = TokioIo::new(stream);

            // Clone state for the handler
            let state = self.state.clone();

            tokio::task::spawn(async move {
                if let Err(err) = http1::Builder::new()
                    .serve_connection(
                        io,
                        service_fn(move |req| handle_request(req, state.clone())),
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

/// Handle incoming HTTP requests
async fn handle_request(
    req: Request<hyper::body::Incoming>,
    state: Arc<ServerState>,
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

async fn handle_random_image(state: Arc<ServerState>) -> Result<Response<Full<Bytes>>> {
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

async fn handle_sequential_image(state: Arc<ServerState>) -> Result<Response<Full<Bytes>>> {
    let mut current_index = state.current_index.lock().unwrap();
    if state.sources.is_empty() {
        return Err(anyhow!("No image sources configured"));
    }

    let source = &state.sources[*current_index];
    *current_index = (*current_index + 1) % state.sources.len();

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
        None => Err(anyhow!("Image not found in cache")),
    }
}
