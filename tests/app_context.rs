use futures::future::AbortHandle;
use leptos::prelude::*;
use price_chart_wasm::app_context::{AppContext, bind_context};
use price_chart_wasm::domain::market_data::{Candle, OHLCV, Price, Symbol, Timestamp, Volume};
use price_chart_wasm::global_state::{ensure_chart, globals};
use price_chart_wasm::ui_state::TooltipData;
use wasm_bindgen_test::*;

wasm_bindgen_test_configure!(run_in_browser);

#[wasm_bindgen_test]
fn independent_roots_dispose_and_remount_without_stale_signals_or_charts() {
    let mount = || {
        let owner = Owner::new();
        let result = owner.with(|| {
            let context = AppContext::provide();
            let chart = ensure_chart(&Symbol::from("BTCUSDT"));
            context.signals.current_price.set(123.0);
            (context, chart)
        });
        (result, owner)
    };
    let ((first, chart), dispose_first) = mount();
    let ((second, _), dispose_second) = mount();
    assert!(!std::sync::Arc::ptr_eq(&first.ecs_world(), &second.ecs_world()));
    first.signals.current_price.set(456.0);
    assert_eq!(second.signals.current_price.get_untracked(), 123.0);
    let (handle, _) = AbortHandle::new_pair();
    first.signals.stream_abort_handles.update(|handles| {
        handles.insert(Symbol::from("BTCUSDT"), handle.clone());
    });
    dispose_first.cleanup();
    assert!(handle.is_aborted());
    assert!(chart.try_get_untracked().is_none());
    assert!(first.ecs_world().lock().unwrap().world.is_empty());
    assert_eq!(second.signals.current_price.get_untracked(), 123.0);
    let ((third, _), dispose_third) = mount();
    assert_eq!(third.ecs_world().lock().unwrap().world.len(), 1);
    dispose_second.cleanup();
    dispose_third.cleanup();
}

#[wasm_bindgen_test]
fn charts_outlive_requesting_child_and_events_stay_with_their_root() {
    let mount = || {
        let owner = Owner::new();
        let result = owner.with(|| {
            let context = AppContext::provide();
            let child = Owner::new();
            let chart = child.with(|| ensure_chart(&Symbol::from("ETHUSDT")));
            child.cleanup();
            assert!(chart.try_get_untracked().is_some());
            let callback = bind_context(|value| globals().current_price.set(value));
            (context, callback)
        });
        (result, owner)
    };
    let ((first, mut callback), dispose_first) = mount();
    let ((second, _), dispose_second) = mount();
    callback(42.0);
    assert_eq!(first.signals.current_price.get_untracked(), 42.0);
    assert_eq!(second.signals.current_price.get_untracked(), 0.0);
    dispose_first.cleanup();
    callback(100.0);
    assert_eq!(second.signals.current_price.get_untracked(), 0.0);
    dispose_second.cleanup();
}

#[wasm_bindgen_test]
fn tooltip_uses_explicit_symbol_without_application_state() {
    let candle = Candle::new(
        Timestamp::from_millis(0),
        OHLCV::new(
            Price::from(10.0),
            Price::from(12.0),
            Price::from(9.0),
            Price::from(11.0),
            Volume::from(2.0),
        ),
    );
    let tooltip = TooltipData::new(&Symbol::from("ETHUSDT"), candle, 10.0, 20.0);
    assert!(tooltip.formatted_text.starts_with("ETHUSDT  UP"));
    assert!(tooltip.formatted_text.contains("10.00%"));
}
