# ─── BUILD ─────────────────────────────────────────────
FROM rust:1.87 AS builder
WORKDIR /usr/src/eq_rng

# Copy in your entire project and build
COPY . .
RUN cargo build --release

# ─── RUNTIME ───────────────────────────────────────────
FROM debian:bookworm-slim
RUN apt-get update \
    && apt-get install -y ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Create a single install directory under /opt
RUN mkdir -p /opt/eq_rng

# Copy in the binary and static assets
COPY --from=builder /usr/src/eq_rng/target/release/eq_rng  /opt/eq_rng/eq_rng
COPY --from=builder /usr/src/eq_rng/zones             /opt/eq_rng/zones
COPY --from=builder /usr/src/eq_rng/data             /opt/eq_rng/data
COPY --from=builder /usr/src/eq_rng/public            /opt/eq_rng/public

# Make sure the binary is executable
RUN chmod +x /opt/eq_rng/eq_rng

# Optionally add a symlink into /usr/local/bin for convenience
RUN ln -s /opt/eq_rng/eq_rng /usr/local/bin/eq_rng

# Set working directory to /opt/eq_rng
WORKDIR /opt/eq_rng

# Expose your port
EXPOSE 3000

# Run your server directly
CMD ["./eq_rng"]
