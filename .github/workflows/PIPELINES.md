# Performance Tracking

The required CI performance job runs a release build of `tests/performance_limit.rs`
on Ubuntu 24.04 with Rust 1.99.0, Chrome 155.0.8059.79 and SwiftShader.
It uses deterministic 1,000 / 50,000 / 1,000,000-candle base datasets, an
800x600 canvas and the production renderer's LOD and 4x MSAA.

Each dataset receives 10 completed warmup frames followed by five samples of
30 frames. Every frame calls the real renderer and awaits the WebGPU queue
completion callback. A missing adapter, failed render, empty geometry, missing
sample, duplicate sample, zero, negative or non-finite value fails the job.

This measures serialized, cached-chart throughput including CPU submission,
browser scheduling and GPU completion latency. It does not measure display FPS,
initial bulk-load cost, live updates, physical GPU speed or memory consumption.
The million source candles are reduced by the production LOD; they are not a
million individual GPU draw calls.

`scripts/parse_perf_log.py` keeps the three workloads separate. The comparator
requires five positive measurements for every workload in both the result and
the reviewed baseline. Any workload median more than 30% below its baseline
fails. This deliberately broad guard detects large regressions on variable
shared CI hosts; it is not a 5% performance guarantee. The baseline is never
created or updated automatically, and a fast workload cannot hide a slower one.

Run the same workload with a matching wasm-bindgen runner and ChromeDriver:

```bash
WASM_BINDGEN_TEST_ONLY_WEB=1 WASM_BINDGEN_TEST_TIMEOUT=180 \
WASM_BINDGEN_TEST_WEBDRIVER_JSON=webdriver.json \
CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=wasm-bindgen-test-runner \
cargo test --release --locked --features performance --test performance_limit -- --nocapture 2>&1 | tee perf.log
python3 scripts/parse_perf_log.py perf.log benchmark_result.json
python3 scripts/compare_fps.py benchmark_result.json DOCS/perf_baseline.json perf_result.json 30
```

`benchmark.yml` is called by CI for every PR and main push; it can also be
dispatched manually. It always retains raw samples, comparison and environment
metadata as `completed-frame-performance`. The duplicate unmeasured `perf.yml`
workflow has been removed. These measurement jobs have read-only permissions
and no notification credentials.

A baseline update must retain raw samples and link the successful measurement
run, tested SHA, browser/compiler versions and software backend. Review changes
to workload or environment before recalibrating; do not copy a regressed result
into the baseline just to make CI green.
