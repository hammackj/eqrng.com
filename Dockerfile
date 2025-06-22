# ─── 1) BUILD ─────────────────────────────────────────────
FROM rust:1.87 AS builder
WORKDIR /usr/src/eq_rng

# 1a) Copy manifest & make a stub so Cargo has something to build
COPY Cargo.toml Cargo.lock ./
RUN mkdir src \
    && printf 'fn main() { println!("Hello, Stub!"); }' > src/main.rs \
    && cargo build --release \
    && rm -rf src

# 1b) Now copy your real project and build it
COPY . .
RUN cargo build --release

# ─── 2) RUNTIME ───────────────────────────────────────────
FROM debian:bookworm-slim
RUN apt-get update \
    && apt-get install -y ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Pull in your compiled server + assets
COPY --from=builder /usr/src/eq_rng/target/release/eq_rng /usr/local/bin/eq_rng
COPY --from=builder /usr/src/eq_rng/zones            /etc/eq_rng/zones
COPY --from=builder /usr/src/eq_rng/public           /etc/eq_rng/public

WORKDIR /etc/eq_rng
EXPOSE 3000

CMD ["eq_rng", "--port", "3000", "--host", "0.0.0.0"]
