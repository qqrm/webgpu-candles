# Performance Tracking

The required CI performance job runs a release build of `tests/performance_limit.rs`
on Ubuntu 24.04 with Rust 1.99.0, Chrome 155.0.8059.79 and SwiftShader.
It uses deterministic 1,000 / 50,000 / 1,000,000-candle base datasets, an
800x600 canvas and the production renderer's LOD and 4x MSAA.

Each dataset receives 10 completed warmup frames followed by five samples of
30 frames. Every frame calls the real renderer and awaits the WebGPU queue
completion via a four-byte readback fence. This includes fence and mapping
overhead; wgpu 25 does not implement its queue-completion callback on WebGPU.
A missing adapter, failed render, empty geometry, missing
sample, duplicate sample, zero, negative or non-finite value fails the job.

This measures serialized, cached-chart throughput including CPU submission,
browser scheduling and GPU completion latency. It does not measure display FPS,
initial bulk-load cost, live updates, physical GPU speed or memory consumption.
The million source candles are reduced by the production LOD; they are not a
million individual GPU draw calls.

`scripts/parse_perf_log.py` keeps the three workloads separate. The comparator
requires five positive measurements for every workload in both the result and
the reference. The reference is pinned to reviewed commit
`1b79e9df3d64542d367a1291273a48dff5ce57a6` and measured first in a separate
checkout on the same runner, with the same compiler, browser and profile as the
candidate. Any workload median more than 30% below that reference fails. Missing
reference measurements fail too; a fast workload cannot hide a slower one.

An absolute cross-host baseline was unsuitable: identical renderer code measured
about 160 and 91-109 completed frames/s on different hosted machines. Paired
measurements reduce that host variance without increasing the threshold. The
broad guard still cannot establish a 5% guarantee or eliminate within-run load
changes. `DOCS/perf_baseline.json` retains the original calibration as historical
evidence; its absolute numbers are not the CI acceptance threshold.

Run a candidate measurement in a disposable checkout with the pinned compiler/browser,
a matching wasm-bindgen runner and ChromeDriver on PATH. Set `CHROME_BINARY` to
the browser executable; the preparation replaces only this checkout's profile:

```bash
set -o pipefail
python3 - <<'PY'
import json
import os
from pathlib import Path
config = json.loads(Path('webdriver-perf.json').read_text())
config['goog:chromeOptions']['binary'] = os.environ['CHROME_BINARY']
Path('webdriver.json').write_text(json.dumps(config))
PY
WASM_BINDGEN_TEST_ONLY_WEB=1 WASM_BINDGEN_TEST_TIMEOUT=180 \
CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=wasm-bindgen-test-runner \
cargo test --release --locked --features performance --test performance_limit -- --nocapture 2>&1 | tee perf.log
python3 scripts/parse_perf_log.py perf.log benchmark_result.json
```

The reusable workflow performs the complete paired comparison, using
`reference_result.json` from its same-runner reference measurement. Dispatch that
workflow to reproduce the CI gate instead of comparing unrelated host results.

`benchmark.yml` is called by CI for every PR and main push; it can also be
dispatched manually. It always retains raw samples, comparison and environment
metadata as `completed-frame-performance`. The duplicate unmeasured `perf.yml`
workflow has been removed. These measurement jobs have read-only permissions
and no notification credentials.

Changing the pinned reference requires review with raw paired samples, both
tested SHAs, browser/compiler versions and software backend. Review changes to
workload or environment before recalibrating; do not advance the reference to
a regressed implementation just to make CI green.

The dedicated `webdriver-perf.json` selects SwiftShader for both ANGLE and Dawn
WebGPU/Vulkan. Selecting only ANGLE allowed a browser adapter to be created but
its device was destroyed before readback. A raw browser preflight now requires
successful mapping and verifies the SwiftShader adapter before measuring Rust.
The job writes that profile to `webdriver.json`, the only filename supported by
the locked wasm-bindgen 0.2.100 runner. Other jobs have independent checkouts.

Provenance records both `tested_sha` (the checked-out workflow SHA, a synthetic
merge commit for pull requests) and `tested_head_sha` (the PR head; identical to
`tested_sha` for pushes/manual runs), the pinned reference SHA and CPU details.
Reference/candidate logs and complete sample sets are retained separately.
Baseline calibration preserves its original tested workflow SHA and PR head.
