FROM rust:latest as builder

# Create a new empty shell project
WORKDIR /usr/src/zayden-backend
RUN cargo new --bin app
WORKDIR /usr/src/zayden-backend/app

# Copy over your manifests
COPY ./web-backend/Cargo.lock ./Cargo.lock
COPY ./web-backend/Cargo.toml ./Cargo.toml

# Build the dependencies to cache them
RUN cargo build --release
RUN rm src/*.rs

# Copy your actual source code
COPY ./web-backend/src ./src

# Build for release
RUN rm ./target/release/deps/app*
RUN cargo build --release

# ---- Final Stage ----
FROM debian:trixie-slim

# Copy the built binary from the builder stage
COPY --from=builder /usr/src/zayden-backend/app/target/release/web /usr/local/bin/web

# Expose the port your app runs on
EXPOSE 3000

# Set the startup command to run your binary
CMD ["web"]