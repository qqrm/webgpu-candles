use price_chart_wasm::domain::{
    chart::{Chart, value_objects::ChartType},
    market_data::{Candle, OHLCV, Price, TimeInterval, Timestamp, Volume},
};

fn make_candle(ts: u64) -> Candle {
    Candle::new(
        Timestamp::new(ts),
        OHLCV::new(
            Price::new(1.0),
            Price::new(1.0),
            Price::new(1.0),
            Price::new(1.0),
            Volume::new(1.0),
        ),
    )
}

fn merge_batch(chart: &mut Chart, mut batch: Vec<Candle>) {
    batch.sort_by_key(|c| c.timestamp.value());
    batch.dedup_by_key(|c| c.timestamp.value());
    for candle in batch {
        chart.add_candle(candle);
    }
}

#[test]
fn backfill_three_batches() {
    let mut chart = Chart::new("TST".to_string(), ChartType::Candlestick, 100);
    for ts in 1000..=1002 {
        chart.add_candle(make_candle(ts));
    }

    merge_batch(&mut chart, (997..=999).map(make_candle).collect());
    merge_batch(&mut chart, (994..=996).map(make_candle).collect());
    merge_batch(&mut chart, (991..=993).map(make_candle).collect());

    let series = chart.get_series(TimeInterval::TwoSeconds).unwrap();
    let candles = series.get_candles();
    assert_eq!(candles.len(), 12);
    for (i, c) in candles.iter().enumerate() {
        assert_eq!(c.timestamp.value(), 991 + i as u64);
    }
}
