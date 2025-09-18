# ARCHITECTURE REFACTORING ROADMAP

## OBJECTIVES
- Stabilize domain logic so it can compile without Leptos, WebGPU, or WASM-only dependencies.
- Introduce clear seams between user interface, rendering, data access, and core computations.
- Prepare the repository for a multi-crate workspace without circular dependencies.
- Reduce reliance on ad-hoc global state in favor of explicit application services and state containers.
- Maintain current functionality (real-time stream, indicators, rendering) while refactoring incrementally with tests.

## CURRENT COUPLING HOTSPOTS
- `src/global_state.rs` stitches together Leptos UI types (`TooltipData`), renderer configuration (`LineVisibility`), and ECS accessors, so any change in UI or rendering cascades through the entire app.
- ECS components (`src/ecs/components.rs`) embed `RwSignal<Chart>`, preventing the ECS layer from being reused without Leptos.
- Rendering queue helpers (`src/app.rs` and `src/infrastructure/rendering`) pull values directly from global signals, hiding side effects and making it difficult to test rendering decisions.
- Binance clients under `src/infrastructure` push updates straight into Leptos signals rather than exposing a domain-friendly API.
- `DomainState` mixes historical candle storage with Leptos-driven refresh cadence, but lacks a dedicated repository service that other layers can depend on.

## TARGET WORKSPACE LAYOUT
| Crate | Responsibilities | Depends On |
| --- | --- | --- |
| `core` | Entities, value objects, indicators, pure services, deterministic tests. | — |
| `data` | Binance REST/WebSocket clients, DTO translation, retry/backoff policies, mocked providers for tests. | `core` |
| `rendering` | GPU primitives, buffer packing, line/tooltip layout independent from Leptos. | `core` |
| `app-services` | Application state container, ECS orchestration, traits for streaming/backfill, adapter from Leptos signals to core models. | `core`, `rendering`, optionally `data` via traits |
| `wasm-app` | Leptos components, hydration entry points, wiring between UI, `app-services`, and platform APIs. | `app-services` |

The first four crates compile natively; only `wasm-app` requires the `wasm32` target.

## ITERATIVE PLAN
### ITERATION 0 – BASELINE AND SAFETY NETS
- Document the current refactoring roadmap (this file) and list required invariants.
- Add integration tests for indicator calculations and chart viewport math to guard key behaviors.
- Introduce feature flags or conditional compilation hooks to run the app without WebGPU during tests.

### ITERATION 1 – HARDEN DOMAIN AND ECS BOUNDARIES
- Move `TooltipData`, `LineVisibility`, and similar UI concepts into a dedicated `app::ui_state` module so `global_state` depends only on that module instead of the entire `app.rs`.
- Replace `RwSignal<Chart>` in ECS components with a small handle (`ChartHandle`) that hides Leptos behind trait bounds; provide a synchronous implementation for tests.
- Extract `DomainState` and `ViewState` constructors into functions that do not require Leptos, and ensure no domain module imports anything from `app` or `infrastructure`.

### ITERATION 2 – INTRODUCE APPLICATION SERVICES
- Create an `AppContext` struct that owns all signals and exposes typed methods for updates (e.g., `set_streaming`, `update_tooltip`).
- Move Binance websocket/rest callbacks to call through the new context instead of touching globals directly.
- Define traits (`MarketStream`, `HistoryProvider`) that describe the data services. Provide adapters for current Binance implementations.
- Cover the service layer with unit tests using fake implementations of the traits.

### ITERATION 3 – ISOLATE RENDERING
- Wrap the render queue helpers behind a `RenderScheduler` trait implemented by the WebGPU renderer.
- Migrate `LineVisibility` and GPU configuration types into `rendering` so the UI layer only depends on traits.
- Provide a dummy renderer used in tests to verify scheduling without invoking WebGPU.
- Ensure rendering-specific state is no longer stored in `global_state`, but owned by the scheduler or renderer crate.

### ITERATION 4 – PREPARE CRATE EXTRACTION
- Split the repository into a Cargo workspace with `core`, `app-services`, and `wasm-app` crates; move source files accordingly while keeping `rendering` and `data` inside `app-services` temporarily via modules.
- Fix imports, adjust `Cargo.toml` dependencies, and ensure `cargo check` succeeds for the new workspace.
- Update documentation (`ARCHITECTURE.md`, `README.md`) to reflect the new structure.

### ITERATION 5 – FINALIZE MULTI-CRATE STRUCTURE
- Extract `data` and `rendering` as standalone crates consumed by `app-services`.
- Provide integration tests that run the full pipeline using mock data streams to validate crate boundaries.
- Audit for any remaining `OnceCell` globals; replace them with context-owned instances passed through dependency injection.
- Conduct final cleanup: remove dead code, ensure all binaries and WASM entry points build, update CI workflows.

## RISKS AND MITIGATIONS
- **Risk:** Increased compilation times due to workspace split. **Mitigation:** Share features and enable incremental builds; start by creating crates without additional dependencies.
- **Risk:** Hard-to-test asynchronous flows. **Mitigation:** Introduce trait-based abstractions and synchronous mocks in early iterations.
- **Risk:** UI regressions caused by refactored state handling. **Mitigation:** Add snapshot tests for tooltip rendering and viewport calculations before moving logic.

## DONE CRITERIA PER ITERATION
- ✅ All commands `cargo fmt --all`, `cargo check --tests --benches`, `cargo clippy --tests --benches`, `cargo test`, and `cargo machete` (if available) succeed.
- ✅ Documentation updated to mirror the new module ownership.
- ✅ Tests cover new seams introduced in each iteration.
- ✅ No module (or crate, once extracted) depends on layers above it.
