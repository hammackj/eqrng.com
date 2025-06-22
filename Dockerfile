### ─── 1) PLANNER STAGE ───────────────────────────────────────────
# Builds a tiny “dummy” binary just to populate Cargo’s dependency cache.

FROM rust:1.87 AS planner
WORKDIR /usr/src/eq_rng

# Copy over Cargo files only
COPY Cargo.toml Cargo.lock ./
# Create a dummy main.rs so cargo will download & compile your deps
RUN mkdir src \
    && echo 'fn main() {}' > src/main.rs \
    && cargo build --release \
    && rm -rf src

### ─── 2) BUILDER STAGE ───────────────────────────────────────────
# Now copy in your real source *and* reuse the planner’s target folder.

FROM rust:1.87 AS builder
WORKDIR /usr/src/eq_rng

# Copy in the cached build outputs from planner
COPY --from=planner /usr/src/eq_rng/target target

# Copy your real source + assets
COPY . .

# Build your real binary
RUN cargo build --release

### ─── 3) RUNTIME STAGE ───────────────────────────────────────────
# A minimal image that just ships your compiled binary & files.

FROM debian:bookworm-slim
RUN apt-get update \
    && apt-get install -y ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Copy the release binary and your zones & public folders
COPY --from=builder /usr/src/eq_rng/target/release/eq_rng /usr/local/bin/eq_rng
COPY --from=builder /usr/src/eq_rng/zones /etc/eq_rng/zones
COPY --from=builder /usr/src/eq_rng/public /etc/eq_rng/public

WORKDIR /etc/eq_rng
EXPOSE 3000

# If you want to run as non-root, create/apply an app user:
RUN addgroup --gid 1000 app \
    && adduser  --uid 1000 --gid 1000 --disabled-password --gecos "" app \
    && chown -R app:app /etc/eq_rng
USER app

CMD ["eq_rng", "--port", "3000", "--host", "0.0.0.0"]
