# syntax=docker/dockerfile:1
# Stage 1: Build the Rust application
FROM rust:1.73-alpine as build_base

# Add Maintainer Info
LABEL maintainer="Sam Wang <sam.wang.0723@gmail.com>"

# Install required packages
RUN apk add --no-cache \
    build-base \
    libc6-compat \
    openssl-dev \
    pkgconfig

RUN apk add --no-cache git
RUN apk update && apk add ca-certificates && apk add tzdata

WORKDIR /app

# Copy everything from the current directory to the PWD (Present Working Directory) inside the container
COPY . .

RUN cargo build --release

# Stage 2: Start fresh from a smaller image
FROM scratch

WORKDIR /

COPY --from=build_base /usr/share/zoneinfo /usr/share/zoneinfo
COPY --from=build_base /etc/ssl/certs/ca-certificates.crt /etc/ssl/certs/
# Copy the binary from the builder stage
COPY --from=build_base /app/stocks.json /stocks.json
COPY --from=build_base /app/target/release/ultron /ultron

# This container exposes ports to the outside world
EXPOSE 80 443 9092 6379

ENV TZ=Asia/Taipei

# Set the CMD to your binary
CMD ["/ultron"]
