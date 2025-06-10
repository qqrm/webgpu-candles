pub mod chart;
/// **CLEAN DOMAIN LAYER** - 100% чистые бизнес абстракции
/// Соответствует принципам ARCHITECTURE.md v3.0
// === CORE AGGREGATES ===
pub mod market_data; // Агрегат: Рыночные данные // Агрегат: Графики

// === DOMAIN INFRASTRUCTURE ===
pub mod errors;
pub mod logging; // 🆕 Абстракции логирования (Logger, TimeProvider traits) // 🆕 Типизированные ошибки (DomainError hierarchy)

// === CLEAN EXPORTS ===
pub use errors::*;
pub use logging::*;
