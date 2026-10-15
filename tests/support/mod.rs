use leptos::{Disposer, Owner, SignalSet, as_child_of_current_owner, with_owner};
use price_chart_wasm::app_context::AppContext;
use price_chart_wasm::domain::market_data::TimeInterval;

/// A disposable reactive owner with explicit base-series defaults for each test.
pub struct AppTestScope {
    owner: Owner,
    _dispose: Disposer,
}

impl AppTestScope {
    pub fn new() -> Self {
        let mount = as_child_of_current_owner(|()| {
            let context = AppContext::provide();
            context.signals.current_interval.set(TimeInterval::TwoSeconds);
            Owner::current().expect("test owner")
        });
        let (owner, dispose) = mount(());
        Self { owner, _dispose: dispose }
    }

    pub fn run<T>(&self, test: impl FnOnce() -> T) -> T {
        with_owner(self.owner, test)
    }
}

pub fn with_app_context<T>(test: impl FnOnce() -> T) -> T {
    AppTestScope::new().run(test)
}
