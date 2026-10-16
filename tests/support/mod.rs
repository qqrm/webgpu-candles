use leptos::prelude::{Owner, Set};
use price_chart_wasm::app_context::AppContext;
use price_chart_wasm::domain::market_data::TimeInterval;

/// A disposable reactive owner with explicit base-series defaults for each test.
pub struct AppTestScope {
    owner: Owner,
}

impl AppTestScope {
    pub fn new() -> Self {
        let owner = Owner::new();
        owner.with(|| {
            let context = AppContext::provide();
            context.signals.current_interval.set(TimeInterval::TwoSeconds);
        });
        Self { owner }
    }
    pub fn run<T>(&self, test: impl FnOnce() -> T) -> T {
        self.owner.with(test)
    }
}
impl Drop for AppTestScope {
    fn drop(&mut self) {
        self.owner.cleanup();
    }
}

pub fn with_app_context<T>(test: impl FnOnce() -> T) -> T {
    AppTestScope::new().run(test)
}
