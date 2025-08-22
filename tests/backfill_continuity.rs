use price_chart_wasm::domain::market_data::{
    Candle, CandleSeries, OHLCV, Price, Timestamp, Volume,
};

fn make_candle(i: u64) -> Candle {
    Candle::new(
        Timestamp::from_millis(i * 60_000),
        OHLCV::new(
            Price::from(i as f64),
            Price::from(i as f64),
            Price::from(i as f64),
            Price::from(i as f64),
            Volume::from(1.0),
        ),
    )
}

fn merge_batch(series: &mut CandleSeries, mut batch: Vec<Candle>) {
    batch.sort_by(|a, b| a.timestamp.value().cmp(&b.timestamp.value()));
    batch.dedup_by_key(|c| c.timestamp.value());
    for c in batch {
        series.add_candle(c);
    }
}

#[test]
fn backfill_three_batches_no_gaps() {
    let mut series = CandleSeries::new(5000);
    for i in 3000..4000 {
        series.add_candle(make_candle(i));
    }

    merge_batch(&mut series, (2000..3000).rev().map(make_candle).collect());
    merge_batch(&mut series, (1000..2000).map(make_candle).collect());
    merge_batch(&mut series, (0..1000).map(make_candle).collect());

    assert_eq!(series.count(), 4000);
    let mut prev = None;
    for c in series.get_candles() {
        if let Some(p) = prev {
            assert_eq!(c.timestamp.value() - p, 60_000);
        }
        prev = Some(c.timestamp.value());
    }
}
