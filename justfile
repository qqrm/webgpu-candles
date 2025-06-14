# Docker commands
build:
    docker build -t candles .

run:
    docker run --rm -p 9999:80 candles

# Stop and rebuild everything
rebuild:
    docker stop $(docker ps -q --filter ancestor=candles) || true
    docker build -t candles .
    docker run --rm -p 9999:80 candles
