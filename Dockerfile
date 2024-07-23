# syntax=docker/dockerfile:1
# Stage 1: Build the Rust application
FROM rust:1.78-slim AS build_base

WORKDIR /usr/src

# Add Maintainer Info
LABEL maintainer="Sam Wang <sam.wang.0723@gmail.com>"

# Install required packages
RUN apt-get update && apt-get install -y \
    build-essential \
    libc6-dev \
    libssl-dev \
    pkg-config \
    cmake \
    ca-certificates \
    tzdata \
    git

#ENV PKG_CONFIG_SYSROOT_DIR=/usr/x86_64-unknown-linux-musl
#ENV OPENSSL_DIR=/usr

RUN USER=root cargo new ultron

# Copy everything from the current directory to the PWD (Present Working Directory) inside the container
COPY Cargo.toml Cargo.lock /usr/src/ultron/
COPY config.*.yaml /usr/src/ultron/

# Set the working directory
WORKDIR /usr/src/ultron

## Install target platform (Cross-Compilation) --> Needed for Alpine
#RUN rustup target add x86_64-unknown-linux-musl

# This is a dummy build to get the dependencies cached.
RUN cargo build --release

ENV SQLX_OFFLINE=true

COPY src /usr/src/ultron/src
## Touch main.rs to prevent cached release build
RUN touch /usr/src/ultron/src/main.rs

RUN cargo build --release

# Stage 2: Start fresh from a smaller image
#FROM alpine:latest

# Copy the binary from the builder stage
#COPY --from=build_base /usr/src/config.local.yaml /config.local.yaml
#COPY --from=build_base /usr/src/config.prod.yaml /config.prod.yaml
#COPY --from=build_base /usr/src/ultron/target/x86_64-unknown-linux-musl/release/ultron /usr/local/bin

# This container exposes ports to the outside world
EXPOSE 80 443

# Set the CMD to your binary
ENTRYPOINT ["/usr/src/ultron/target/release/ultron"]
