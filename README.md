# WebGPU Candles

![screenshot](res/screen.png)

A demonstration Bitcoin candlestick chart built with **WebGPU** for rendering and **Leptos** for the reactive UI. Real-time price data is streamed from Binance via WebSocket and drawn directly to a `<canvas>` using Rust compiled to WebAssembly.
The chart supports zoom levels from roughly `0.2x` up to `32x` with a minimum of one visible candle.
> **Note**: WebGPU must be enabled in your browser. The demo works in Microsoft Edge but is not supported in Firefox.

## Demo

The development version is available at <https://qqrm.github.io/webgpu-candles/dev/>. Release builds are
published at <https://qqrm.github.io/webgpu-candles/>. The files are
stored in the [`docs/`](docs/) directory.

The project requires the `wasm32-unknown-unknown` target, which the build script verifies is installed. Install it with:
`rustup target add wasm32-unknown-unknown`.

## Setup

```bash
# Add the WebAssembly compilation target
rustup target add wasm32-unknown-unknown
# Install Trunk for building and serving
cargo install trunk
```

Install either [Trunk](https://trunkrs.dev/) or [wasm-pack](https://rustwasm.github.io/wasm-pack/) depending on your preferred workflow.

To automatically format and lint the code before each commit, enable the pre-commit hook:

```bash
git config core.hooksPath .githooks
```

## Building with Trunk

Trunk compiles the project and automatically injects the generated WASM into `index.html`:

```bash
trunk serve       # dev server on http://localhost:8080
# or
trunk build --dist dist-local
```

Local builds are saved to `dist-local`. In GitHub Actions the `dist` path is
used and the files are copied to [`docs/`](docs/) to publish the demo.
The `dist/` directory is not stored in the repository, only the contents of
`docs/` are committed. The `docs/version` file stores the SHA of the last
commit.

Both release and development builds are copied into `docs/` by default. To use a different folder, adjust the copy steps in the workflow files:
`.github/workflows/build.yml` lines **51–54** and `.github/workflows/release.yml` lines **51–56**.

When using Trunk, open **`index.html`** (served automatically when using `trunk serve`). The file contains a Trunk hook so the WASM is loaded for you:

```html
<!-- Trunk will automatically inject the WASM here -->

<link data-trunk rel="rust" data-wasm-opt="z" />
```

### Subresource Integrity

Trunk automatically includes integrity hashes for the generated JavaScript
and WebAssembly files.

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
├── simple_shader.wgsl      # WebGPU shaders
├── domain/                 # Core domain logic (chart, market data, logging)
├── infrastructure/         # WebSocket and WebGPU renderer implementations
```

For more architectural details see [ARCHITECTURE.md](DOCS/ARCHITECTURE.md).
Planned features are listed in [FEATURES.md](DOCS/FEATURES.md).
Details on the WebSocket feed are in [WEBSOCKETS.md](DOCS/WEBSOCKETS.md).

## Documentation

All additional documentation lives in the [`DOCS/`](DOCS/) directory:

- [ARCHITECTURE.md](DOCS/ARCHITECTURE.md)
- [FEATURES.md](DOCS/FEATURES.md)
- [WEBSOCKETS.md](DOCS/WEBSOCKETS.md)
- [CONTRIBUTING.md](DOCS/CONTRIBUTING.md)
- [PIPELINES.md](.github/workflows/PIPELINES.md)
- [TESTS.md](DOCS/TESTS.md)
- [PIPELINES.md](DOCS/PIPELINES.md)
- [VOLUME_SYNC_FIXES.md](DOCS/VOLUME_SYNC_FIXES.md)
- [COLORS.md](DOCS/COLORS.md)

## Chart Elements

The chart is composed of several layers:

- Candles with wicks representing OHLC data
- Volume bars below the main chart
- Time and price grid lines
- A highlighted line for the current price
- Technical indicators:
  - Simple Moving Averages (20, 50, 200 periods)
  - Exponential Moving Averages (12, 26 periods)
  - Ichimoku cloud with Tenkan, Kijun, Senkou spans and Chikou line

## Benchmarks

To measure performance use Node:

```bash
wasm-pack test --node
```

You can also benchmark in a browser with:

```bash
wasm-pack test --chrome --headless
```

FPS is printed to the console and the `perf.yml` workflow saves the log as an
artifact. Current metric values are stored in [PIPELINES.md](.github/workflows/PIPELINES.md).
`tests/performance_limit.rs` logs when FPS drops below 30 for large charts.


## Tests

The tests use [`wasm-bindgen-test`](https://docs.rs/wasm-bindgen-test). Run
them with:

```bash
wasm-pack test --node
```

To test in a browser:

```bash
wasm-pack test --chrome --headless
```

Alternatively install Node dependencies and run:

```bash
npm install
npm test
```

See [TESTS.md](DOCS/TESTS.md) for more details about the test suite.

## Native Run

For benchmarking outside the browser you can run the native binary. Parallel ECS
systems powered by Rayon are enabled automatically:

```bash
cargo run --release --features parallel
```
## Docker

Build and run the container with:
```bash
docker build -t webgpu-candles .
docker run --rm -p 8080:80 webgpu-candles
```
(the container uses nginx, so port 80 is mapped to host 8080).


## License
This project is distributed under the [MIT License](LICENSE).

