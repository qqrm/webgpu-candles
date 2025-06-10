FROM rust:latest AS builder

# Устанавливаем целевую платформу и Trunk
RUN rustup target add wasm32-unknown-unknown && \
    cargo install --locked trunk

WORKDIR /app
COPY . .

# Сборка проекта через Trunk для production (переопределяем настройки для Docker)
RUN trunk build --release --dist dist --public-url /

FROM nginx:alpine
WORKDIR /usr/share/nginx/html
# Копируем готовый dist каталог
COPY --from=builder /app/dist/ ./

EXPOSE 80
CMD ["nginx", "-g", "daemon off;"]
