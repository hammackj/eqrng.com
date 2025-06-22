# 1. Build Stage
FROM rust:1.87 as builder
WORKDIR /usr/src/eqrng

# Cache dependencies
COPY Cargo.toml Cargo.lock ./
RUN mkdir src && echo "fn main(){}" > src/main.rs
RUN cargo build --release
# Now copy the real source
COPY . .
RUN rm src/main.rs
RUN cargo build --release

# 2. Runtime Stage
FROM debian:bullseye-slim
# Install libssl if you use HTTPS or other deps
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*

# Copy the compiled binary and static assets
COPY --from=builder /usr/src/eqrng/target/release/eqrng /usr/local/bin/eqrng
COPY --from=builder /usr/src/eqrng/zones /etc/eqrng/zones
COPY --from=builder /usr/src/eqrng/public /etc/eqrng/public

WORKDIR /etc/eqrng
EXPOSE 3000

# Run as non-root (optional)
USER 1000:1000

CMD ["eqrng"]
