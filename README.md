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

You can also override the configuration using environment variables. The environment variables should be prefixed with `RANDOM_IMAGE_SERVER_`, and the keys should be in uppercase with underscores instead of dots. For example, to set the port, you can use the environment variable `RANDOM_IMAGE_SERVER_PORT`.

## Installation

follow instructions in the Release page for the latest release, which involves curling a script and piping it to `sh`, or install from crates.io:

```bash
cargo install random-image-server
```

## Usage

### As a Docker Container

> I don't know how to publish the docker image to a registry without having to pay for it, so you will have to build the image yourself.
> So, you'll need to clone the repository and build the image yourself.

You can build and run the server as a docker container using the provided Dockerfile and docker-compose file.

1. Make sure to update the `config.toml` file with your desired configuration, and update the `compose.yml` file to mount the configuration file and any image directories you want to serve.
2. Build the docker image with `docker compose build`:
3. Run the docker container with `docker compose up`:

This will start the server and expose it on port 8080. You can access the server at `http://localhost:8080`.

### As a Systemd Service

After downloading the binary:

1. put the binary in `/usr/local/bin/random-image-server`
2. download the `random-image-server.service` file from the repo and place it in `/etc/systemd/system/`
3. place your `config.toml` file in `/etc/random-image-server/config.toml`, and edit it to your liking.
4. run the following commands to enable and start the service:

```bash
sudo systemctl enable random-image-server.service
sudo systemctl start random-image-server.service
```

You can check the status of the service with:

```bash
sudo systemctl status random-image-server.service
```

and view the logs with:

```bash
sudo journalctl -u random-image-server.service
```
