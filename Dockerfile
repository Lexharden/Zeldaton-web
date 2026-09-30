# syntax=docker/dockerfile:1
# Website: builds the Vue app and serves the static files (host nginx proxies to this container).
FROM node:24-alpine AS build
WORKDIR /app
COPY package.json package-lock.json ./
RUN npm ci
COPY . .
# VITE_* values are baked in at build time.
ARG VITE_MOCK_MODE=false
ARG VITE_DEMO_MODE=false
ARG VITE_API_URL=/api
ARG VITE_WS_URL=/ws
ENV VITE_MOCK_MODE=$VITE_MOCK_MODE VITE_DEMO_MODE=$VITE_DEMO_MODE \
    VITE_API_URL=$VITE_API_URL VITE_WS_URL=$VITE_WS_URL
RUN npx vite build

FROM nginxinc/nginx-unprivileged:stable-alpine
COPY deploy/frontend-nginx.conf /etc/nginx/conf.d/default.conf
COPY --from=build /app/dist /usr/share/nginx/html
EXPOSE 8080
HEALTHCHECK --interval=30s --timeout=3s CMD wget -qO- http://127.0.0.1:8080/ >/dev/null || exit 1
