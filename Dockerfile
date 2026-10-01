# The static site as an image: the Rust engine is compiled to WebAssembly in the
# first stage, and the second stage is a web server holding nothing but the
# result. No Rust or Python is present at runtime.

FROM rust:1.98.1-slim-bookworm AS build

# python3 runs web/build.py; curl lets wasm-pack fetch its wasm-bindgen binary.
RUN apt-get update \
    && apt-get install -y --no-install-recommends python3 curl ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Before the sources, so that these layers survive a code change.
RUN rustup target add wasm32-unknown-unknown \
    && cargo install wasm-pack --locked --version 0.13.1

WORKDIR /app
COPY rust-toolchain.toml Cargo.toml Cargo.lock ./
COPY src src
# Cargo.toml names the test files explicitly, so the manifest does not load
# without them.
COPY tests tests
COPY models models
COPY web web

RUN python3 web/build.py

FROM caddy:2-alpine
COPY Caddyfile /etc/caddy/Caddyfile
COPY --from=build /app/dist/web /srv
EXPOSE 80
