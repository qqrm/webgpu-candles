# Recorded performance baseline

This is a fixed calibration, not a live production FPS report.

| Source candles | Median completed frames/s |
| --- | --- |
| 1,000 | 160.54 |
| 50,000 | 159.21 |
| 1,000,000 | 161.75 |

Measured on Ubuntu 24.04, Chrome 155.0.8059.79, Rust 1.99.0 and the verified
Google SwiftShader adapter. Each workload has five samples of 30 completed frames
after ten warmup frames, with production LOD, an 800x600 canvas and 4x MSAA.
Readback synchronization overhead is included; display FPS and physical GPU
performance are not measured.

[Calibration run](https://github.com/qqrm/webgpu-candles/actions/runs/38680812974) on head
`1b79e9df3d64542d367a1291273a48dff5ce57a6`. The measurement step passed; the initial
comparison intentionally failed against the invalid legacy zero baseline.

Raw samples and environment provenance are retained in [perf_baseline.json](perf_baseline.json).
Subsequent CI results are in the `completed-frame-performance` artifact of each CI run.
See [the measurement contract](../.github/workflows/PIPELINES.md) before changing the baseline.
