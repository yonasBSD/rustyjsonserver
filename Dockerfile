# syntax=docker/dockerfile:1

ARG RUST_VERSION=1.91.1

FROM rust:${RUST_VERSION}-alpine AS build
WORKDIR /app

# Install host build dependencies.
RUN apk add --no-cache clang lld musl-dev git

# Build the app
RUN --mount=type=bind,source=./src,target=/app/src \
    --mount=type=bind,source=./Cargo.toml,target=/app/Cargo.toml \
    --mount=type=bind,source=./Cargo.lock,target=/app/Cargo.lock \
    --mount=type=cache,target=/app/target/ \
    --mount=type=cache,target=/usr/local/cargo/git/db \
    --mount=type=cache,target=/usr/local/cargo/registry/ \
    cargo build --locked --release --bin rjserver && \
    cp ./target/release/rjserver /bin/server

# Create a new stage for running the application.

FROM alpine:3.18 AS final

# Create the directory for the application.
RUN mkdir -p /app

# Copy the executable from the "build" stage.
COPY --from=build /bin/server /bin/

# Expose the port that the application listens on.
EXPOSE 8080

# Use a bind mount for the configuration file.
# The config file should be mounted at /app/config.json at runtime.

# run: rjserver serve --config /app/config.json
ENTRYPOINT ["/bin/server"]
CMD ["serve", "--config", "/app/config.json"]
