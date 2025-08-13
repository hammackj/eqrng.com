# ─── FRONTEND BUILD ────────────────────────────────────
FROM node:18-alpine AS frontend-builder
WORKDIR /usr/src/frontend

# Copy frontend package files
COPY frontend/package*.json ./
RUN npm ci

# Copy frontend source and public directory for assets
COPY frontend/ ./
COPY public/ ./public/
ENV DOCKER_BUILD=true
RUN npm run build

# ─── BACKEND BUILD ─────────────────────────────────────
FROM rust:1.87 AS backend-builder
WORKDIR /usr/src/eq_rng

# Copy backend files
COPY Cargo.toml Cargo.lock ./
COPY src/ ./src/
COPY data/ ./data/

# Build the backend without admin features for production (verbose output)
# Adding -v makes cargo print rustc invocations and diagnostics even when non-interactive.
RUN cargo build --release --no-default-features -v

# ─── RUNTIME ───────────────────────────────────────────
FROM debian:bookworm-slim
RUN apt-get update \
    && apt-get install -y ca-certificates sqlite3 jq \
    && rm -rf /var/lib/apt/lists/*

# Create install directory
RUN mkdir -p /opt/eq_rng

# Copy backend binary
COPY --from=backend-builder /usr/src/eq_rng/target/release/eq_rng /opt/eq_rng/eq_rng

# Copy backend data and assets
COPY --from=backend-builder /usr/src/eq_rng/data /opt/eq_rng/data
COPY public/ /opt/eq_rng/public/
COPY config/ /opt/eq_rng/config/

# Copy built frontend assets (fixed source path to match frontend-builder WORKDIR)
COPY --from=frontend-builder /usr/src/frontend/dist /opt/eq_rng/dist

# Copy zone ratings and transaction log management utilities
COPY utils/setup_logs.sh /opt/eq_rng/utils/
COPY utils/docker-entrypoint.sh /opt/eq_rng/

# Make scripts executable
RUN chmod +x /opt/eq_rng/utils/setup_logs.sh \
    && chmod +x /opt/eq_rng/docker-entrypoint.sh || true

# Make binary executable
RUN chmod +x /opt/eq_rng/eq_rng

# Create directories for backups and migrations
RUN mkdir -p /opt/eq_rng/backups/database \
    && mkdir -p /opt/eq_rng/logs

# Create symlink for convenience
RUN ln -s /opt/eq_rng/eq_rng /usr/local/bin/eq_rng

# Set working directory
WORKDIR /opt/eq_rng

# Expose port
EXPOSE 3000

# Use the entrypoint script to handle initialization
ENTRYPOINT ["/opt/eq_rng/docker-entrypoint.sh"]
CMD ["./eq_rng"]
