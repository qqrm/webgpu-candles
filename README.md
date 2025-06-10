# WebGPU Candles

A demonstration Bitcoin candlestick chart built with **WebGPU** for rendering and **Leptos** for the reactive UI. Real-time price data is streamed from Binance via WebSocket and drawn directly to a `<canvas>` using Rust compiled to WebAssembly.

## Setup

```bash
# Add the WebAssembly compilation target
rustup target add wasm32-unknown-unknown
# Install Trunk for building and serving
cargo install trunk
```

Install either [Trunk](https://trunkrs.dev/) or [wasm-pack](https://rustwasm.github.io/wasm-pack/) depending on your preferred workflow.

## Building with Trunk

Trunk compiles the project and automatically injects the generated WASM into `index.html`:

```bash
trunk serve       # dev server on http://localhost:8080
# или
trunk build --dist dist-local
```

When using Trunk, open **`index.html`** (served automatically when using `trunk serve`). The file contains a Trunk hook so the WASM is loaded for you:

```html
<!-- Trunk автоматически подключит WASM здесь -->
<link data-trunk rel="rust" data-wasm-opt="z" />
```

## Building with wasm-pack

Alternatively, you can build using wasm-pack:

```bash
wasm-pack build --target web --release
```

This produces a `pkg/` directory with the compiled `price_chart_wasm.js`. After running wasm-pack, open **`leptos-index.html`**, which manually imports the generated file:

```html
<script type="module">
    import init, { hydrate } from './pkg/price_chart_wasm.js';
    // ...
</script>
```

## Directory Structure

Key folders are under `src/`:

```text
src/
├── app.rs                  # Leptos UI components and reactivity
├── lib.rs                  # WASM exports (entry points)
├── candle_shader.wgsl      # WebGPU shaders
├── domain/                 # Core domain logic (chart, market data, logging)
├── infrastructure/         # WebSocket and WebGPU renderer implementations
```

For more architectural details see [ARCHITECTURE.md](ARCHITECTURE.md).

## Демо

Актуальная статическая версия собирается через GitHub Actions и доступна
по адресу <https://qqrm.github.io/webgpu-candles/>. Файлы публикуются в
каталог [`docs/`](docs/).

## Бенчмарки

Для оценки производительности используйте:

```bash
wasm-pack test --headless --chrome -- --nocapture
```

FPS выводится в консоль, а workflow `perf.yml` сохраняет лог артефактом.
Актуальные значения метрик сохраняются в [docs/perf.md](docs/perf.md).

## Тесты

Тесты написаны с использованием [`wasm-bindgen-test`](https://docs.rs/wasm-bindgen-test). Запустить их можно так:

```bash
wasm-pack test
```

Подробнее о наборе тестов можно прочитать в [TESTS.md](TESTS.md).

## Docker

Собрать и запустить образ можно так:
```bash
docker build -t webgpu-candles .
docker run --rm -p 8080:80 webgpu-candles
```
(в контейнере используется nginx, поэтому порт 80 маппится на 8080 хоста).


## License
Этот проект распространяется по лицензии [MIT License](LICENSE).

