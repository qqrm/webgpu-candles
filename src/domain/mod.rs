/// **CLEAN DOMAIN LAYER** - 100% чистые бизнес абстракции
/// Соответствует принципам ARCHITECTURE.md v3.0
// === CORE AGGREGATES ===
pub mod market_data;  // Агрегат: Рыночные данные
pub mod chart;        // Агрегат: Графики

// === DOMAIN INFRASTRUCTURE ===
pub mod logging;      // 🆕 Абстракции логирования (Logger, TimeProvider traits)
pub mod errors;       // 🆕 Типизированные ошибки (DomainError hierarchy)

// === CLEAN EXPORTS ===
pub use logging::*; 
pub use errors::*; 