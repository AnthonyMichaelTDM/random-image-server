# random-image-server

A simple http server that returns a random image from a pre-configured directory.

The server exposes the following endpoints:

- `GET /health`: Returns a 200 OK response to indicate the server is running.
- `GET /random`: Returns a random image from the configured sources.
- `GET /seqeuntial`: Returns the next image in sequence from the configured sources.

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
