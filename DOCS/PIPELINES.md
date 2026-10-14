# GitHub Actions pipelines

- CI runs locked stable Rust checks and strict clippy, Rust browser tests, Node logic tests, deterministic WebGPU end-to-end tests and a current RustSec audit of the committed lockfile for every pull request and main push. The ci-gate requires all five jobs to succeed; skipped or failed jobs block merge. The audit rejects vulnerabilities and unsoundness advisories; informational maintenance/yanked warnings remain visible.
- Workflow Security runs actionlint, zizmor and signed Dependabot commit provenance checks. Its gate requires all three results.
- Weekly Dependabot maintenance batches cover Cargo, npm and Actions. Native auto-merge targets the exact tested head after both gates, with branch protection supplying the final requirements. No review or cron merge sweep is used.
- build.yml publishes the development bundle after a main push; release.yml keeps its published-release/manual triggers. Build jobs have read-only repository access; only the separate Pages deployment jobs can write contents.
- benchmark.yml and perf.yml retain performance artifacts. Benchmark results no longer write directly to the protected main branch. Existing optional Telegram notifications remain restricted to those main/release workflows.

Browser execution runs on hosted CI. The software Vulkan/WebGPU backend validates behavior, not physical-device GPU performance. Geometry fixtures contain no GPU resources and are safe to drop. Actual WebGPU behavior is validated separately by the hosted UI suite. Rust browser tests block Binance DNS; stream abort tests use local handles.

## Known performance reporting issue

The legacy `perf` and `benchmark` workflows currently prove smoke-test/pipeline execution only. Missing FPS samples are converted to zero by `parse_perf_log.py`, and the zero baseline does not enforce performance. Their green status must not be interpreted as a performance guarantee. A follow-up needs a representative measured workload and failure on missing samples; alternatively, these workflows should explicitly become smoke tests without an FPS gate.
