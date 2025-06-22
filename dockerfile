# 1. Build Stage
FROM rust:1.87 as builder
WORKDIR /usr/src/eqrng

# 1a. Copy only Cargo files and build a dummy src/main.rs to cache deps
COPY Cargo.toml Cargo.lock ./
RUN mkdir -p src \
    && echo 'fn main() { println!("dummy"); }' > src/main.rs

# Build dependencies (this layer is cached unless Cargo.toml changes)
RUN cargo build --release

# 1b. Copy the real source & assets
COPY . .

# Build the actual binary
RUN cargo build --release

# 2. Runtime Stage
FROM debian:bullseye-slim
RUN apt-get update \
    && apt-get install -y ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Copy the compiled binary and static assets
COPY --from=builder /usr/src/eqrng/target/release/eqrng /usr/local/bin/eqrng
COPY --from=builder /usr/src/eqrng/zones /etc/eqrng/zones
COPY --from=builder /usr/src/eqrng/public /etc/eqrng/public

WORKDIR /etc/eqrng
EXPOSE 3000

# Run as non-root (optional)
USER 1000:1000

CMD ["eqrng"]
