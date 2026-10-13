# GitHub Actions pipelines

- CI runs locked stable Rust checks and strict clippy, Rust browser tests, Node logic tests and deterministic WebGPU end-to-end tests for every pull request and main push. The ci-gate requires all four jobs, including browser tests, to succeed; skipped or failed jobs block merge.
- Workflow Security runs actionlint, zizmor and signed Dependabot commit provenance checks. Its gate requires all three results.
- Weekly Dependabot maintenance batches cover Cargo, npm and Actions. Native auto-merge targets the exact tested head after both gates, with branch protection supplying the final requirements. No review or cron merge sweep is used.
- build.yml publishes the development bundle after a main push; release.yml keeps its published-release/manual triggers. Build jobs have read-only repository access; only the separate Pages deployment jobs can write contents.
- benchmark.yml and perf.yml retain performance artifacts. Benchmark results no longer write directly to the protected main branch. Existing optional Telegram notifications remain restricted to those main/release workflows.

Browser execution runs on hosted CI. The software Vulkan/WebGPU backend validates behavior, not physical-device GPU performance. Geometry fixtures contain no GPU resources and are safe to drop. Actual WebGPU behavior is validated separately by the hosted UI suite. Rust browser tests block Binance DNS; stream abort tests use local handles.
