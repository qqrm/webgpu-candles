pub mod chart;
/// **CLEAN DOMAIN LAYER** - 100% pure business abstractions
/// Follows the principles of ARCHITECTURE.md v3.0
// === CORE AGGREGATES ===
pub mod market_data; // Aggregate: market data and charts

// === DOMAIN INFRASTRUCTURE ===
pub mod errors;
pub mod logging; // 🆕 Logging abstractions (Logger, TimeProvider traits) // 🆕 Typed errors (DomainError hierarchy)

// === CLEAN EXPORTS ===
pub use errors::*;
pub use logging::*;
