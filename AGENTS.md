# Instructions for Codex Agents

- All replies must be in Russian, short and to the point.
- Before committing run the following commands in sequence:
  - `cargo fmt --all`
  - `cargo check --tests --benches`
  - `cargo clippy --tests --benches --fix --allow-dirty -- -D warnings`
- Running tests (e.g. `wasm-pack test`) won't work due to wasm; they will run in GitHub Actions.
- One of the readiness criteria is the absence of formatting issues and linter warnings.
- Do not suppress warnings about unused code via `#[allow(dead_code)]`. Remove dead code instead.
- When adding new functionality you must write tests.
- Before tackling any task consult [ARCHITECTURE.md](ARCHITECTURE.md) and ensure proposed changes match the current architecture.
- When working with colors consult [colors.md](colors.md).
- Decisions on acceleration and optimization are made after studying [PERFORMANCE.md](PERFORMANCE.md).
- Keep pull requests concise and clear: list changes, provide references like `F:path#L1-L2`, and attach test results.
- Markdown in the project must be consistent: use `#` for headers, specify languages for code blocks, and maintain clear structure.
- Comments in the code and all documentation are written only in English.
- A summary of available tests and snapshot locations is found in [TESTS.md](TESTS.md).
- Translate branch and task names into English even if they were originally given in Russian.
- If a task description is in Russian, use its English translation when naming branches and in task descriptions.
