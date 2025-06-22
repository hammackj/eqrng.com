### ─── 1) BUILD & CACHE STAGE ────────────────────────────────
FROM rust:1.87 AS build
WORKDIR /usr/src/eq_rng

# Copy manifests & fetch deps (populates registry cache)
COPY Cargo.toml Cargo.lock ./
RUN cargo fetch

# Copy the rest of your code
COPY . .

# Build with cache mounts for both registry and target
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/src/eq_rng/target \
    cargo build --release

### ─── 2) RUNTIME STAGE ──────────────────────────────────────
FROM debian:bookworm-slim
RUN apt-get update \
    && apt-get install -y ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Copy your compiled server + assets
COPY --from=build /usr/src/eq_rng/target/release/eq_rng  /usr/local/bin/eq_rng
COPY --from=build /usr/src/eq_rng/zones              /etc/eq_rng/zones
COPY --from=build /usr/src/eq_rng/public             /etc/eq_rng/public

WORKDIR /etc/eq_rng
EXPOSE 3000

# Optionally drop privileges
RUN addgroup --gid 1000 app \
    && adduser  --uid 1000 --gid 1000 --disabled-password --gecos "" app \
    && chown -R app:app /etc/eq_rng
USER app

CMD ["eq_rng"]
