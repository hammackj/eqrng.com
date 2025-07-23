# ─── FRONTEND BUILD ────────────────────────────────────
FROM node:18-alpine AS builder
WORKDIR /usr/src/frontend

# Copy package files
COPY package*.json ./
RUN npm ci

# Copy source and build
COPY . ./
RUN npm run build

# ─── RUNTIME ───────────────────────────────────────────
FROM nginx:alpine

# Copy built assets to nginx
COPY --from=builder /usr/src/frontend/dist /usr/share/nginx/html

# Create nginx config for SPA routing and API proxying
RUN echo 'server { \
    listen 80; \
    server_name localhost; \
    root /usr/share/nginx/html; \
    index index.html; \
    \
    # Handle static assets \
    location ~* \.(js|css|png|jpg|jpeg|gif|ico|svg|woff|woff2|ttf|eot)$ { \
    expires 1y; \
    add_header Cache-Control "public, immutable"; \
    } \
    \
    # Proxy API calls to backend \
    location /random_zone { proxy_pass ${BACKEND_URL}; } \
    location /random_race { proxy_pass ${BACKEND_URL}; } \
    location /random_class { proxy_pass ${BACKEND_URL}; } \
    location /version { proxy_pass ${BACKEND_URL}; } \
    location /zones { proxy_pass ${BACKEND_URL}; } \
    \
    # Handle SPA routing \
    location / { \
    try_files $uri $uri/ /index.html; \
    } \
    }' > /etc/nginx/conf.d/default.conf

# Expose port
EXPOSE 80

# Start nginx
CMD ["nginx", "-g", "daemon off;"]
