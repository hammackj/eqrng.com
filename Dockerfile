# ─── 1) PLANNER: cache deps in its own folder ─────────────────────
FROM rust:1.87 AS planner
WORKDIR /usr/src/planner

# Only copy Cargo.toml & Cargo.lock → this layer only busts when deps change
COPY Cargo.toml Cargo.lock ./

# Create stub project, build it, then remove stub code
RUN mkdir src \
    && printf 'fn main() { println!("stub"); }\n' > src/main.rs \
    && cargo build --release \
    && rm -rf src

# ─── 2) BUILDER: pull in real code ───────────────────────────────
FROM rust:1.87 AS builder
WORKDIR /usr/src/eq_rng

# 1) bring in all the cached dependencies
COPY --from=planner /usr/src/planner/target target

# 2) copy your actual project (src/, zones/, public/, Cargo.toml, etc)
COPY . .

# 3) delete the stub executable so Cargo must rebuild your code
RUN rm -f target/release/eq_rng \
    && cargo build --release

# ─── 3) RUNTIME: minimal image ────────────────────────────────────
FROM debian:bookworm-slim
RUN apt-get update \
    && apt-get install -y ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Copy server + assets into place
COPY --from=builder /usr/src/eq_rng/target/release/eq_rng  /usr/local/bin/eq_rng
COPY --from=builder /usr/src/eq_rng/zones              /etc/eq_rng/zones
COPY --from=builder /usr/src/eq_rng/public             /etc/eq_rng/public

WORKDIR /etc/eq_rng
EXPOSE 3000

# Unprivileged user
RUN addgroup --gid 1000 app \
    && adduser  --uid 1000 --gid 1000 --disabled-password --gecos "" app \
    && chown -R app:app /etc/eq_rng
USER app

CMD ["eq_rng"]
