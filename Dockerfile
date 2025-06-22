# syntax=docker/dockerfile:1.4
FROM rust:1.87 AS build
WORKDIR /usr/src/eq_rng

# 1) Create a stub so Cargo.toml is valid
RUN mkdir -p src \
    && printf 'fn main() { println!("Hello, world!"); }' > src/main.rs

# 2) Copy just the manifests & fetch deps
COPY Cargo.toml Cargo.lock ./
RUN cargo fetch

# 3) Bring in your real project, overwriting the stub
COPY . .

# 4) Build with cache mounts for registry & target
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/src/eq_rng/target \
    cargo build --release

### Runtime stage ###
FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*

COPY --from=build /usr/src/eq_rng/target/release/eq_rng  /usr/local/bin/eq_rng
COPY --from=build /usr/src/eq_rng/zones              /etc/eq_rng/zones
COPY --from=build /usr/src/eq_rng/public             /etc/eq_rng/public

WORKDIR /etc/eq_rng
EXPOSE 3000
CMD ["eq_rng"]
