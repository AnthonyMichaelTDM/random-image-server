FROM clux/muslrust AS builder

# Set the working directory
WORKDIR /app

# Copy the source code
COPY src /app/src
COPY Cargo.toml /app/Cargo.toml
COPY Cargo.lock /app/Cargo.lock

# Build the application
RUN cargo build --release --target x86_64-unknown-linux-musl

# New stage for the final image
FROM alpine:latest

# Create a user and group for running the application
RUN addgroup --system random-image-server && \
    adduser --system --ingroup random-image-server random-image-server

# Copy the binary from the builder stage
COPY --from=builder /app/target/x86_64-unknown-linux-musl/release/random-image-server /usr/local/bin/random-image-server

# Create the configuration directory
RUN mkdir -p /etc/random-image-server

# Set the user and group for the application
USER random-image-server:random-image-server

ENTRYPOINT [ "/usr/local/bin/random-image-server", "/etc/random-image-server/config.toml" ]
