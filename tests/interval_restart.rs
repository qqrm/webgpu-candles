#![cfg(feature = "render")]
use leptos::prelude::*;
use price_chart_wasm::app::{
    abort_all_streams, current_interval, current_symbol, stream_abort_handles,
};
use price_chart_wasm::domain::market_data::{
    Candle, OHLCV, Price, TimeInterval, Timestamp, Volume, value_objects::Symbol,
};
use price_chart_wasm::global_state::{ensure_chart, get_chart_signal, push_realtime_candle};
use wasm_bindgen_test::*;

wasm_bindgen_test_configure!(run_in_browser);

#[wasm_bindgen_test]
fn candle_after_interval_change() {
    let owner = leptos::prelude::Owner::new();
    owner.with(|| {
        current_symbol().set(Symbol::from("BTCUSDT"));
        ensure_chart(&current_symbol().get_untracked());
        current_interval().set(TimeInterval::OneMinute);
        let (handle, _) = futures::future::AbortHandle::new_pair();
        stream_abort_handles().update(|handles| {
            handles.insert(Symbol::from("BTCUSDT"), handle.clone());
        });
        let chart_signal = get_chart_signal(&current_symbol().get_untracked()).unwrap();
        let before = chart_signal.with(|c| c.get_candle_count());

        current_interval().set(TimeInterval::TwoSeconds);
        if let Some(handle) =
            stream_abort_handles().with(|m| m.get(&current_symbol().get_untracked()).cloned())
        {
            handle.abort();
            stream_abort_handles().update(|m| {
                m.remove(&current_symbol().get_untracked());
            });
        }
        abort_all_streams();
        assert!(handle.is_aborted());

        let candle = Candle::new(
            Timestamp::from_millis(1),
            OHLCV::new(
                Price::from(1.0),
                Price::from(1.0),
                Price::from(1.0),
                Price::from(1.0),
                Volume::from(1.0),
            ),
        );
        push_realtime_candle(candle);
        assert!(chart_signal.with(|c| c.get_candle_count()) > before);

        if let Some(handle) =
            stream_abort_handles().with(|m| m.get(&current_symbol().get_untracked()).cloned())
        {
            handle.abort();
            stream_abort_handles().update(|m| {
                m.remove(&current_symbol().get_untracked());
            });
        }
    });
}
