FROM rust:latest AS builder

# Install wasm target and wasm-pack
RUN rustup target add wasm32-unknown-unknown && \
    apt-get update && apt-get install -y --no-install-recommends curl && \
    curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh && \
    rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY . .

RUN wasm-pack build --target web --release

FROM nginx:alpine
WORKDIR /usr/share/nginx/html
COPY index.html .
COPY --from=builder /app/pkg ./pkg

CMD ["nginx", "-g", "daemon off;"]
