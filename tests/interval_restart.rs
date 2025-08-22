use gloo_timers::future::sleep;
use leptos::*;
use price_chart_wasm::app::{
    current_interval, current_symbol, start_websocket_stream, stream_abort_handles,
};
use price_chart_wasm::domain::market_data::{
    Candle, OHLCV, Price, TimeInterval, Timestamp, Volume, value_objects::Symbol,
};
use price_chart_wasm::global_state::{ensure_chart, get_chart_signal, push_realtime_candle};
use std::time::Duration;
use wasm_bindgen_test::*;

#[wasm_bindgen_test(async)]
async fn candle_after_interval_change() {
    current_symbol().set(Symbol::from("BTCUSDT"));
    ensure_chart(&current_symbol().get_untracked());
    current_interval().set(TimeInterval::OneMinute);
    let (_, set_status) = create_signal(String::new());
    start_websocket_stream(set_status).await;
    sleep(Duration::from_millis(10)).await;
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
    start_websocket_stream(set_status).await;
    sleep(Duration::from_millis(10)).await;

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
    sleep(Duration::from_millis(10)).await;
    assert!(chart_signal.with(|c| c.get_candle_count()) > before);

    if let Some(handle) =
        stream_abort_handles().with(|m| m.get(&current_symbol().get_untracked()).cloned())
    {
        handle.abort();
        stream_abort_handles().update(|m| {
            m.remove(&current_symbol().get_untracked());
        });
    }
}
