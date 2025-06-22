#####################################################
# 1) BUILD WITH CACHING                             #
#####################################################
FROM rust:1.87 AS build
WORKDIR /usr/src/eq_rng

# 1a) Copy just manifests, fetch deps & prime registry cache
COPY Cargo.toml Cargo.lock ./
RUN cargo fetch

# 1b) Copy your real code into place
COPY . .

# 1c) Build with two cache mounts:
#    - crates.io registry downloads
#    - cargo build artifacts under target/
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/src/eq_rng/target \
    cargo build --release

#####################################################
# 2) RUNTIME                                       #
#####################################################
FROM debian:bookworm-slim
RUN apt-get update \
    && apt-get install -y ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Copy only the final binary + assets
COPY --from=build /usr/src/eq_rng/target/release/eq_rng  /usr/local/bin/eq_rng
COPY --from=build /usr/src/eq_rng/zones              /etc/eq_rng/zones
COPY --from=build /usr/src/eq_rng/public             /etc/eq_rng/public

WORKDIR /etc/eq_rng
EXPOSE 3000

CMD ["eq_rng"]
