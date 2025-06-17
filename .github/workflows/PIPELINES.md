# Performance Tracking

## Improvement Checklist
- [ ] Profile GPU memory usage
- [x] Reduce vertex buffer updates
- [x] Cache computations on the GPU side
- [ ] Minimize CPU/GPU synchronization

### Additional Changes
- [x] `CandleSeries` switched to `VecDeque`
- [x] Store `Chart` in state
 - [x] Log vector on `VecDeque`

## WebGPU Performance Benchmark
- [x] Measure FPS for different data volumes
- [x] Log render pass times through `wgpu::CommandEncoder::finish`
- [x] Use `Performance.now()` in the browser for frame timing
- [x] Automate running a fixed-candle test scene

The benchmark is run with:

```bash
wasm-pack test --headless --chrome
```

Workflow `perf.yml` stores the `perf-log` file.
