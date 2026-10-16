//! Version-specific reactive lifetime adapter; all measurement logic lives in performance_limit.
use leptos::prelude::{Owner, Set};
use price_chart_wasm::domain::market_data::TimeInterval;

pub async fn run(task: impl std::future::Future<Output = ()> + 'static) {
    let owner = Owner::new();
    owner
        .with(|| {
            price_chart_wasm::app_context::AppContext::provide();
            leptos::reactive::computed::ScopedFuture::new(task)
        })
        .await;
    owner.cleanup();
}

pub fn set_interval(interval: TimeInterval) {
    price_chart_wasm::app::current_interval().set(interval);
}
