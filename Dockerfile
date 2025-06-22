# ─── BUILD ─────────────────────────────────────────────
FROM rust:1.87 AS builder
WORKDIR /usr/src/eq_rng

# Copy everything (your real src/, Cargo.toml, zones/, public/, etc.)
COPY . .

# Build the real binary
RUN cargo build --release

# ─── RUNTIME ───────────────────────────────────────────
FROM debian:bookworm-slim
RUN apt-get update \
    && apt-get install -y ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Copy in the compiled binary and your static assets
COPY --from=builder /usr/src/eq_rng/target/release/eq_rng /usr/local/bin/eq_rng
COPY --from=builder /usr/src/eq_rng/zones             /etc/eq_rng/zones
COPY --from=builder /usr/src/eq_rng/public            /etc/eq_rng/public

WORKDIR /etc/eq_rng
EXPOSE 3000
CMD ["eq_rng"]
