use leptos::*;
use price_chart_wasm::domain::market_data::{Candle, OHLCV, Price, Symbol, Timestamp, Volume};
use price_chart_wasm::global_state::ensure_chart;

#[test]
fn charts_accumulate_independently() {
    let btc = Symbol::from("BTCUSDT");
    let eth = Symbol::from("ETHUSDT");
    let btc_chart = ensure_chart(&btc);
    let eth_chart = ensure_chart(&eth);

    let candle_btc = Candle::new(
        Timestamp::from_millis(0),
        OHLCV::new(
            Price::from(1.0),
            Price::from(1.0),
            Price::from(1.0),
            Price::from(1.0),
            Volume::from(1.0),
        ),
    );
    btc_chart.update(|ch| ch.add_candle(candle_btc));

    let candle_eth = Candle::new(
        Timestamp::from_millis(60_000),
        OHLCV::new(
            Price::from(2.0),
            Price::from(2.0),
            Price::from(2.0),
            Price::from(2.0),
            Volume::from(2.0),
        ),
    );
    eth_chart.update(|ch| ch.add_candle(candle_eth));

    assert_eq!(btc_chart.with(|c| c.get_candle_count()), 1);
    assert_eq!(eth_chart.with(|c| c.get_candle_count()), 1);
}
