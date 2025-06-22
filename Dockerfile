# syntax=docker/dockerfile:1

#####################################################
# 1) PLANNER: generate a stub to cache dependencies #
#####################################################
FROM rust:1.87 AS planner
WORKDIR /usr/src/eq_rng

# Copy only Cargo files so changes here bust this layer
COPY Cargo.toml Cargo.lock ./

# Create a tiny stub main, build it, then delete it
RUN mkdir src \
    && printf 'fn main() { }\n' > src/main.rs \
    && cargo build --release \
    && rm -rf src

#####################################################
# 2) BUILDER: reuse cache & build your real binary  #
#####################################################
FROM rust:1.87 AS builder
WORKDIR /usr/src/eq_rng

# Reuse the planner’s compiled dependencies
COPY --from=planner /usr/src/eq_rng/target target

# Bring in your actual source & assets
COPY . .

# Build the real, final binary
RUN cargo build --release

#####################################################
# 3) RUNTIME: minimal image with your server (+assets)
#####################################################
FROM debian:bookworm-slim
RUN apt-get update \
    && apt-get install -y ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Copy in the compiled server and static files
COPY --from=builder /usr/src/eq_rng/target/release/eq_rng  /usr/local/bin/eq_rng
COPY --from=builder /usr/src/eq_rng/zones              /etc/eq_rng/zones
COPY --from=builder /usr/src/eq_rng/public             /etc/eq_rng/public

WORKDIR /etc/eq_rng
EXPOSE 3000

# (Optional) run as an unprivileged user
RUN addgroup --gid 1000 app \
    && adduser  --uid 1000 --gid 1000 --disabled-password --gecos "" app \
    && chown -R app:app /etc/eq_rng
USER app

# Launch your server (Axum’s .serve() blocks forever)
CMD ["eq_rng"]
