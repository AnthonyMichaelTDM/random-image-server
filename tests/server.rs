use std::{net::SocketAddr, path::PathBuf, time::Duration};

use hyper::service::service_fn;
use hyper_util::{
    rt::{TokioExecutor, TokioIo},
    server::conn::auto,
};
use pretty_assertions::{assert_eq, assert_ne};
use random_image_server::{ImageServer, config::ImageSource, handle_request};
use rstest::{fixture, rstest};
use tokio::net::TcpListener;

// Test state to hold server address and join handle
//
// This server will handle `n` requests before shutting down
#[derive(Debug)]
struct TestState {
    pub addr: SocketAddr,
    pub join_handle: tokio::task::JoinHandle<()>,
}

impl TestState {
    async fn new(requests_to_handle: usize) -> Self {
        let mut server = ImageServer::default();
        server.config.server.sources = vec![ImageSource::Path(PathBuf::from("assets"))];

        // Populate the cache with images from configured sources
        server.populate_cache().await;
        assert_ne!(server.state.read().await.cache.size(), 0);

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        let executor = auto::Builder::new(TokioExecutor::new());

        let handle = tokio::spawn(async move {
            for _ in 0..requests_to_handle {
                // Handle each request in a loop
                let (stream, _) = listener.accept().await.unwrap();

                let io = TokioIo::new(stream);

                let service = service_fn(|req| {
                    let value = server.state.clone();
                    async move { handle_request(req, value).await }
                });

                // Spawn a new task to handle the connection
                if let Err(e) = executor.serve_connection(io, service).await {
                    log::error!("Failed to serve connection: {e}");
                }
            }
        });

        Self {
            addr,
            join_handle: handle,
        }
    }
}

#[fixture]
async fn test_one_request() -> TestState {
    TestState::new(1).await
}

#[rstest]
#[timeout(std::time::Duration::from_secs(2))]
#[tokio::test]
async fn test_handle_request_root(#[future] test_one_request: TestState) {
    let TestState { addr, join_handle } = test_one_request.await;

    let response = reqwest::get(format!("http://{addr}/")).await.unwrap();

    assert_eq!(response.status(), hyper::StatusCode::OK);

    assert_eq!(
        response.text().await.unwrap(),
        "Welcome to the Random Image Server!"
    );

    join_handle.await.unwrap();
}

#[rstest]
#[timeout(Duration::from_secs(2))]
#[tokio::test]
async fn test_handle_request_health(#[future] test_one_request: TestState) {
    let TestState { addr, join_handle } = test_one_request.await;

    let response = reqwest::get(format!("http://{addr}/health")).await.unwrap();
    assert_eq!(response.status(), hyper::StatusCode::OK);
    assert_eq!(response.text().await.unwrap(), "OK");

    join_handle.await.unwrap();
}

#[rstest]
#[timeout(Duration::from_secs(2))]
#[tokio::test]
async fn test_handle_request_not_found(#[future] test_one_request: TestState) {
    let TestState { addr, join_handle } = test_one_request.await;

    let response = reqwest::get(format!("http://{addr}/unknown"))
        .await
        .unwrap();

    assert_eq!(response.status(), hyper::StatusCode::NOT_FOUND);
    assert_eq!(response.text().await.unwrap(), "Not Found");
    join_handle.await.unwrap();
}

#[rstest]
#[timeout(Duration::from_secs(2))]
#[tokio::test]
async fn test_handle_request_random_image(#[future] test_one_request: TestState) {
    let TestState { addr, join_handle } = test_one_request.await;

    let response = reqwest::get(format!("http://{addr}/random")).await.unwrap();

    assert_eq!(response.status(), hyper::StatusCode::OK);
    assert!(response.headers().get("Content-Type").is_some());
    assert_eq!(
        response.headers().get("Content-Type").unwrap(),
        "image/jpeg"
    );
    assert!(!response.bytes().await.unwrap().is_empty());

    join_handle.await.unwrap();
}

#[rstest]
#[timeout(Duration::from_secs(2))]
#[tokio::test]
async fn test_handle_request_sequential_image(#[future] test_one_request: TestState) {
    let TestState { addr, join_handle } = test_one_request.await;

    let response = reqwest::get(format!("http://{addr}/sequential"))
        .await
        .unwrap();

    assert_eq!(response.status(), hyper::StatusCode::OK);
    assert!(response.headers().get("Content-Type").is_some());
    assert_eq!(
        response.headers().get("Content-Type").unwrap(),
        "image/jpeg"
    );
    assert!(!response.bytes().await.unwrap().is_empty());
    join_handle.await.unwrap();
}
