FROM lukemathwalker/cargo-chef:latest-rust-1 AS chef
WORKDIR /app

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json
# Compile dependencies... (this layer caches)
RUN cargo chef cook --release --recipe-path recipe.json

# And then the main program.
COPY . .
RUN cargo build --release

FROM debian:latest
WORKDIR /app
COPY --from=builder /app/target/release/intersection /usr/local/bin
ENTRYPOINT ["/usr/local/bin/intersection"]