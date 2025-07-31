# GitHub Actions Pipelines

This document outlines the CI pipelines used in this repository.

- `build.yml` — compiles the project for multiple platforms.
- `test.yml` — runs unit and integration tests.
- `benchmark.yml` — measures performance against baseline.
- `perf.yml` — tracks WebAssembly size and rendering speed.
- `release.yml` — publishes release builds and artifacts.

## Security check

Automated checks run only when a pull request is opened by `qqrm`.
Pull requests from other users are ignored by CI.
