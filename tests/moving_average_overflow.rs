#![cfg(feature = "render")]
mod support;
wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

use price_chart_wasm::domain::{
    chart::{Chart, value_objects::ChartType},
    market_data::{Candle, OHLCV, Price, TimeInterval, Timestamp, Volume},
};
use price_chart_wasm::infrastructure::rendering::renderer::candle_x_position;
use price_chart_wasm::infrastructure::rendering::renderer::dummy_renderer;

#[wasm_bindgen_test::wasm_bindgen_test]
fn moving_averages_continue_after_capacity_overflow() {
    support::with_app_context(|| {
        let max_candles = 40usize;
        let total_candles = max_candles * 30 + 60;
        let mut chart = Chart::new("ma-overflow".to_string(), ChartType::Candlestick, max_candles);

        let base_interval_ms = TimeInterval::TwoSeconds.duration_ms();
        let minute_interval_ms = TimeInterval::OneMinute.duration_ms();

        let mut minute_closes = Vec::new();
        let mut current_bucket: Option<u64> = None;

        for i in 0..total_candles {
            let timestamp = i as u64 * base_interval_ms;
            let close = i as f64;
            let candle = Candle::new(
                Timestamp::from_millis(timestamp),
                OHLCV::new(
                    Price::from(close),
                    Price::from(close),
                    Price::from(close),
                    Price::from(close),
                    Volume::from(1.0),
                ),
            );

            let bucket_start = timestamp / minute_interval_ms * minute_interval_ms;
            match current_bucket {
                Some(active) if active == bucket_start => {
                    if let Some(last_close) = minute_closes.last_mut() {
                        *last_close = close;
                    }
                }
                _ => {
                    current_bucket = Some(bucket_start);
                    minute_closes.push(close);
                }
            }

            chart.add_candle(candle);
        }

        chart.update_viewport_for_data();

        let base_engine = chart.ma_engines.get(&TimeInterval::TwoSeconds).expect("base engine");
        let base_data = base_engine.data();
        assert_eq!(base_data.ema_12.len(), total_candles);
        let expected_base_sma20_len = total_candles - 20 + 1;
        assert_eq!(base_data.sma_20.len(), expected_base_sma20_len);
        let last_index = total_candles - 1;
        let expected_base_avg = ((last_index - 19) as f64 + last_index as f64) / 2.0;
        let base_last = base_data.sma_20.last().expect("latest SMA20 value for base interval");
        assert!((base_last.value() - expected_base_avg).abs() < 1e-9);

        let minute_engine = chart.ma_engines.get(&TimeInterval::OneMinute).expect("minute engine");
        let minute_data = minute_engine.data();
        // A bucket closes only when the next one starts. The final minute is
        // still open even though this fixture supplied its last two-second bar.
        let closed_minutes = &minute_closes[..minute_closes.len() - 1];
        assert_eq!(minute_data.ema_12.len(), closed_minutes.len());
        let expected_minute_sma_len = closed_minutes.len() - 20 + 1;
        assert_eq!(minute_data.sma_20.len(), expected_minute_sma_len);
        let minute_avg: f64 =
            closed_minutes[closed_minutes.len() - 20..].iter().copied().sum::<f64>() / 20.0;
        let minute_last =
            minute_data.sma_20.last().expect("latest SMA20 value for minute interval");
        assert!((minute_last.value() - minute_avg).abs() < 1e-9);

        let base = chart.get_series(TimeInterval::TwoSeconds).unwrap().get_candles();
        assert_eq!(base.len(), max_candles);
        let minute_series = chart.get_series(TimeInterval::OneMinute).unwrap();
        assert_eq!(minute_series.count(), max_candles);
        assert_eq!(
            minute_series.latest().unwrap().ohlcv.close.value(),
            *minute_closes.last().unwrap()
        );

        let (zoom, pan) = price_chart_wasm::app::viewport_zoom_pan(base, &chart.viewport);
        let mut renderer = dummy_renderer();
        renderer.set_zoom_params(zoom, pan);
        let (instances, vertices, uniforms) = renderer.create_geometry_for_test(&chart);
        assert_eq!(instances.len(), max_candles);
        for (color, period) in [(2, 20), (3, 50), (4, 200), (5, 12), (6, 26)] {
            let line: Vec<_> = vertices.iter().filter(|v| v.color_type == color as f32).collect();
            assert_eq!(line.len(), (max_candles - 1) * 6, "indicator {color}");
            // Independent closed-form references for a unit price ramp. Check
            // both ends: old retained-prefix indexing and EMA warmup shifts
            // must not render an apparently present but stale indicator line.
            for (display_index, a, b) in
                [(0, 0, 2), (max_candles - 1, line.len() - 2, line.len() - 1)]
            {
                let absolute_index = total_candles - max_candles + display_index;
                let lag = (period - 1) as f64 / 2.0;
                let expected_price = if color <= 4 {
                    absolute_index as f64 - lag
                } else {
                    let decay = 1.0 - 2.0 / (period as f64 + 1.0);
                    absolute_index as f64 - lag * (1.0 - decay.powi(absolute_index as i32))
                };
                let expected_y = (expected_price as f32 - uniforms.viewport[2])
                    / (uniforms.viewport[3] - uniforms.viewport[2])
                    * 2.0
                    - 1.0;
                let actual_x = (line[a].position_x + line[b].position_x) / 2.0;
                let actual_y = (line[a].position_y + line[b].position_y) / 2.0;
                assert!((actual_x - candle_x_position(display_index, max_candles)).abs() < 1e-6);
                assert!(
                    (actual_y - expected_y).abs() < 1e-5,
                    "indicator {color} at {display_index}"
                );
            }
        }

        // The retained aggregate series also overflowed, but its final bucket
        // is still open. No committed MA value may be shifted onto that candle.
        use leptos::SignalSet;
        price_chart_wasm::app::current_interval().set(TimeInterval::OneMinute);
        chart.update_viewport_for_series(TimeInterval::OneMinute);
        let minutes = chart.get_series(TimeInterval::OneMinute).unwrap().get_candles();
        let (zoom, pan) = price_chart_wasm::app::viewport_zoom_pan(minutes, &chart.viewport);
        renderer.set_zoom_params(zoom, pan);
        let (_, vertices, uniforms) = renderer.create_geometry_for_test(&chart);
        for (color, period) in [(2, 20), (5, 12), (6, 26)] {
            let line: Vec<_> = vertices.iter().filter(|v| v.color_type == color as f32).collect();
            let points =
                if color == 2 { closed_minutes.len() - period + 1 } else { max_candles - 1 };
            assert_eq!(line.len(), (points - 1) * 6, "minute indicator {color}");
            let last = closed_minutes.len() - 1;
            let lag = (period - 1) as f64 / 2.0;
            let reference = if color == 2 {
                29.0 + 30.0 * (last as f64 - lag)
            } else {
                let decay = 1.0 - 2.0 / (period as f64 + 1.0);
                29.0 + 30.0 * (last as f64 - lag * (1.0 - decay.powi(last as i32)))
            };
            let expected_y = (reference as f32 - uniforms.viewport[2])
                / (uniforms.viewport[3] - uniforms.viewport[2])
                * 2.0
                - 1.0;
            let last_x = (line[line.len() - 2].position_x + line[line.len() - 1].position_x) / 2.0;
            let last_y = (line[line.len() - 2].position_y + line[line.len() - 1].position_y) / 2.0;
            assert!((last_x - candle_x_position(max_candles - 2, max_candles)).abs() < 1e-6);
            assert!((last_y - expected_y).abs() < 1e-5, "minute indicator {color}");
        }
    });
}
