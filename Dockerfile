# ---- Planner Stage ----
FROM rust:latest AS planner
WORKDIR /app
RUN cargo install --locked cargo-chef
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM rust:latest AS builder
WORKDIR /app
RUN cargo install --locked cargo-chef
# Copy the dependency recipe from the planner stage.
COPY --from=planner /app/recipe.json recipe.json

RUN apt-get update && apt-get install -y cmake pkg-config build-essential

# Build dependencies based on the recipe. This layer is cached heavily.
RUN cargo chef cook --release --recipe-path recipe.json

# Copy your application source code and build it.
# This is the only part that will re-run on code changes.
COPY . .
RUN cargo build --release --bin web

# ---- Final Stage ----
FROM debian:trixie-slim

COPY --from=builder /app/target/release/web /usr/local/bin/web

RUN apt-get update && apt-get install -y ca-certificates && update-ca-certificates

EXPOSE 3000
CMD ["web"]