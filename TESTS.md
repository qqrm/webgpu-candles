# Test Suite

The `tests/` folder contains unit and integration tests for the main components. Key files include:

- `viewport.rs` — verifies `Viewport` methods (coordinate conversions, panning and zoom)
- `geometry.rs` — generates candle vertices and compares them with a snapshot
- `offset.rs` — checks candle positioning by index and count
- `indicator_vertices.rs` — validates vertices for indicator and current price lines

Snapshot fixtures are stored in `tests/fixtures`.

Internal module tests for the renderer can be found in `src/infrastructure/rendering/renderer/render_loop.rs`.
