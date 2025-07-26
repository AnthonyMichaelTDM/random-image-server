[![codecov](https://codecov.io/gh/AnthonyMichaelTDM/random-image-server/graph/badge.svg?token=iqU3gMydit)](https://codecov.io/gh/AnthonyMichaelTDM/random-image-server)
[![Continuous Integration](https://github.com/AnthonyMichaelTDM/random-image-server/actions/workflows/ci.yml/badge.svg)](https://github.com/AnthonyMichaelTDM/random-image-server/actions/workflows/ci.yml)

# random-image-server

A simple http server that returns a random image from a pre-configured directory.

The server exposes the following endpoints:

- `GET /health`: Returns a 200 OK response to indicate the server is running.
- `GET /random`: Returns a random image from the configured sources.
- `GET /sequential`: Returns the next image in sequence from the configured sources.

## Features

- Random image serving: Returns a random image from among the configured sources.
- Sequential image serving: Enumerates images sequentially from the configured sources.
- In-memory caching: Caches images at startup for fast access.
- File system caching: Caches images on disk for reduced memory usage.
  - if cached images are modified externally, the server will detect this and invalidate the entry in the cache.
    - TODO: instead, the server should reload the image from the source and update the cache.
- Can serve png, jpg, and webp images, as well as animated gifs.
- Supports both local file paths and URLs as image sources.
- Configurable via a `config.toml` file.
- Graceful shutdown on termination signals.
- Logging, with configurable log levels.

## Configuration

The server can be configured using a `config.toml` file. The configuration file should be placed in the same directory as the binary.
The configuration file should have the following structure:

```toml
[server]
port = 8080 # The port the server will listen on
host = "0.0.0.0" # The host the server will bind to
log_level = "info" # The log level for the server, can be "error", "warn", "info", "debug", or "trace"
sources = [
    "/path/to/image.jpg", 
    "/path/to/another/image.png",
    "/path/to/image/directory", 
    "http://example.com/images"
]

[cache]
# Configuration for the cache backend
backend = "file_system" # The type of cache backend to use, can be "in_memory" or "file_system"
```

## Installation

follow instructions in the Releases page, or install from crates.io:

```bash
cargo install random-image-server
```
