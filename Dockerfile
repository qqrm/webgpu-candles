FROM rust:latest AS builder

# Install target platform and Trunk
RUN rustup target add wasm32-unknown-unknown && \
    cargo install --locked trunk

WORKDIR /app
COPY . .

# Build the project with Trunk for production (Docker settings overridden)
RUN trunk build --release --dist dist --public-url / && \
    git rev-parse --short HEAD > dist/version && \
    TZ=Europe/Moscow date '+%Y-%m-%d %H:%M:%S %Z' > dist/build_time

FROM nginx:alpine
WORKDIR /usr/share/nginx/html
# Copy the prepared dist directory
COPY --from=builder /app/dist/ ./

EXPOSE 80
CMD ["nginx", "-g", "daemon off;"]
