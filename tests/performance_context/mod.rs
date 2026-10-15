//! Version-specific reactive lifetime adapter; all measurement logic lives in performance_limit.
use leptos::{Owner, ScopedFuture, SignalSet, as_child_of_current_owner};
use price_chart_wasm::domain::market_data::TimeInterval;

pub async fn run(task: impl std::future::Future<Output = ()> + 'static) {
    let mount = as_child_of_current_owner(|()| {
        price_chart_wasm::app_context::AppContext::provide();
        Owner::current().expect("benchmark owner")
    });
    let (owner, dispose) = mount(());
    ScopedFuture::new(owner, task).await.expect("benchmark owner alive");
    drop(dispose);
}

pub fn set_interval(interval: TimeInterval) {
    price_chart_wasm::app::current_interval().set(interval);
}
