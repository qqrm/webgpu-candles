# GitHub Actions pipelines

- CI runs locked stable Rust checks and strict clippy, Rust browser tests, Node logic tests, deterministic WebGPU end-to-end tests, measured completed-frame performance and a current RustSec audit of the committed lockfile for every pull request and main push. The ci-gate requires all six jobs to succeed; skipped or failed jobs block merge. The audit rejects vulnerabilities and unsoundness advisories; informational maintenance/yanked warnings remain visible.
- Workflow Security runs actionlint, zizmor and signed Dependabot commit provenance checks. Its gate requires all three results.
- Weekly Dependabot maintenance batches cover Cargo, npm and Actions. Native auto-merge targets the exact tested head after both gates, with branch protection supplying the final requirements. No review or cron merge sweep is used.
- build.yml publishes the development bundle after a main push; release.yml keeps its published-release/manual triggers. Build jobs have read-only repository access; only the separate Pages deployment jobs can write contents.
- benchmark.yml is the reusable read-only performance job required by CI. It retains measurements, comparison and environment artifacts, including on failures. The duplicate unmeasured perf.yml has been removed.

Browser execution runs on hosted CI. The software Vulkan/WebGPU backend validates behavior, not physical-device GPU performance. Geometry fixtures contain no GPU resources and are safe to drop. Actual WebGPU behavior is validated separately by the hosted UI suite. Rust browser tests block Binance DNS; stream abort tests use local handles.

## Measured performance guard

The fixed release-mode workload measures five samples for each of three chart sizes, waiting for GPU completion after every frame. Missing/invalid samples fail closed; medians are compared separately against a pinned reference measured on the same runner, with a 30% tolerance. Cross-host calibration values remain historical evidence only. This is cached-chart serialized throughput on SwiftShader, not physical-device or display FPS. See [the workload and calibration contract](../.github/workflows/PIPELINES.md).
