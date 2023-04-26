FROM rustlang/rust:nightly AS chef
WORKDIR /app
RUN cargo install cargo-chef

FROM debian:buster-slim AS runtime
RUN apt-get update && apt-get upgrade

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder 
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json

COPY . .
RUN cargo build --release

FROM runtime
WORKDIR /app
COPY --from=builder /app/target/release/intersection /usr/local/bin
ENTRYPOINT ["/usr/local/bin/intersection"]