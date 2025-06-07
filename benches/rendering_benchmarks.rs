use criterion::{criterion_group, criterion_main, Criterion, BenchmarkId};
use price_chart_wasm::domain::{
    chart::{Chart, value_objects::ChartType},
    market_data::{
        entities::{Candle, CandleSeries},
        value_objects::{Price, Volume, Timestamp, OHLCV},
    },
};
use std::time::Duration;

/// Генерирует тестовые данные свечей
fn generate_test_candles(count: usize) -> Vec<Candle> {
    let mut candles = Vec::with_capacity(count);
    let mut base_price = 50000.0;
    
    for i in 0..count {
        let timestamp = Timestamp::from(1640000000 + (i as u64 * 60)); // 1 минута интервал
        
        // Случайные колебания цены
        let variation = (i as f32 * 0.1).sin() * 100.0;
        let open = base_price + variation;
        let close = open + ((i as f32 * 0.2).cos() * 50.0);
        let high = open.max(close) + ((i as f32 * 0.3).sin().abs() * 25.0);
        let low = open.min(close) - ((i as f32 * 0.4).cos().abs() * 25.0);
        let volume = 1000.0 + ((i as f32 * 0.5).sin().abs() * 500.0);
        
        let ohlcv = OHLCV::new(
            Price::from(open),
            Price::from(high),
            Price::from(low),
            Price::from(close),
            Volume::from(volume),
        );
        
        candles.push(Candle::new(timestamp, ohlcv));
        base_price = close; // Следующая свеча начинается с закрытия предыдущей
    }
    
    candles
}

/// Генерирует тестовый график
fn generate_test_chart(candle_count: usize) -> Chart {
    let candles = generate_test_candles(candle_count);
    let mut chart = Chart::new(
        format!("test_chart_{}", candle_count),
        ChartType::Candlestick,
        candle_count
    );
    
    for candle in candles {
        chart.add_candle(candle);
    }
    
    chart
}

/// Benchmark для расчета координат свечей (sequential)
fn bench_calculate_coordinates_sequential(c: &mut Criterion) {
    let mut group = c.benchmark_group("coordinate_calculation_sequential");
    group.measurement_time(Duration::from_secs(10));
    
    for candle_count in [10, 50, 100, 200, 500, 1000].iter() {
        let chart = generate_test_chart(*candle_count);
        let candles = chart.data.get_candles();
        
        group.bench_with_input(
            BenchmarkId::new("sequential_coords", candle_count),
            candle_count,
            |b, _| {
                b.iter(|| {
                    // Имитируем расчет координат (без Canvas API)
                    let mut results = Vec::with_capacity(candles.len());
                    for (i, candle) in candles.iter().enumerate() {
                        let x = i as f64 * 5.0; // Ширина свечи
                        let high_y = (60000.0 - candle.ohlcv.high.value() as f64) * 0.01;
                        let low_y = (60000.0 - candle.ohlcv.low.value() as f64) * 0.01;
                        let open_y = (60000.0 - candle.ohlcv.open.value() as f64) * 0.01;
                        let close_y = (60000.0 - candle.ohlcv.close.value() as f64) * 0.01;
                        results.push((x, high_y, low_y, open_y, close_y));
                    }
                    results
                });
            },
        );
    }
    group.finish();
}

/// Benchmark для расчета координат свечей (parallel)
fn bench_calculate_coordinates_parallel(c: &mut Criterion) {
    let mut group = c.benchmark_group("coordinate_calculation_parallel");
    group.measurement_time(Duration::from_secs(10));
    
    for candle_count in [100, 200, 500, 1000, 2000, 5000].iter() {
        let chart = generate_test_chart(*candle_count);
        let candles = chart.data.get_candles();
        
        group.bench_with_input(
            BenchmarkId::new("parallel_coords", candle_count),
            candle_count,
            |b, _| {
                b.iter(|| {
                    // Имитируем параллельный расчет координат
                    use rayon::prelude::*;
                    let results: Vec<_> = candles
                        .par_iter()
                        .enumerate()
                        .map(|(i, candle)| {
                            let x = i as f64 * 5.0; // Ширина свечи
                            let high_y = (60000.0 - candle.ohlcv.high.value() as f64) * 0.01;
                            let low_y = (60000.0 - candle.ohlcv.low.value() as f64) * 0.01;
                            let open_y = (60000.0 - candle.ohlcv.open.value() as f64) * 0.01;
                            let close_y = (60000.0 - candle.ohlcv.close.value() as f64) * 0.01;
                            (x, high_y, low_y, open_y, close_y)
                        })
                        .collect();
                    results
                });
            },
        );
    }
    group.finish();
}

/// Benchmark сравнения sequential vs parallel
fn bench_sequential_vs_parallel(c: &mut Criterion) {
    let mut group = c.benchmark_group("sequential_vs_parallel");
    group.measurement_time(Duration::from_secs(15));
    
    for candle_count in [100, 500, 1000, 2000].iter() {
        let chart = generate_test_chart(*candle_count);
        let candles = chart.data.get_candles();
        
        // Sequential
        group.bench_with_input(
            BenchmarkId::new("sequential", candle_count),
            candle_count,
            |b, _| {
                b.iter(|| {
                    let mut results = Vec::with_capacity(candles.len());
                    for (i, candle) in candles.iter().enumerate() {
                        // Более сложные вычисления (имитация реального рендеринга)
                        let x = i as f64 * 5.0;
                        let price_scale = 0.01;
                        let high_y = (60000.0 - candle.ohlcv.high.value() as f64) * price_scale;
                        let low_y = (60000.0 - candle.ohlcv.low.value() as f64) * price_scale;
                        let open_y = (60000.0 - candle.ohlcv.open.value() as f64) * price_scale;
                        let close_y = (60000.0 - candle.ohlcv.close.value() as f64) * price_scale;
                        
                        // Дополнительные вычисления (цвет, тип свечи)
                        let is_bullish = candle.ohlcv.close.value() >= candle.ohlcv.open.value();
                        let body_height = (open_y - close_y).abs() as f64;
                        let wick_height = high_y - low_y;
                        
                        results.push((x, high_y, low_y, open_y, close_y, is_bullish, body_height, wick_height));
                    }
                    results
                });
            },
        );
        
        // Parallel
        group.bench_with_input(
            BenchmarkId::new("parallel", candle_count),
            candle_count,
            |b, _| {
                b.iter(|| {
                    use rayon::prelude::*;
                    let results: Vec<_> = candles
                        .par_iter()
                        .enumerate()
                        .map(|(i, candle)| {
                            // Те же вычисления, но параллельно
                            let x = i as f64 * 5.0;
                            let price_scale = 0.01;
                            let high_y = (60000.0 - candle.ohlcv.high.value() as f64) * price_scale;
                            let low_y = (60000.0 - candle.ohlcv.low.value() as f64) * price_scale;
                            let open_y = (60000.0 - candle.ohlcv.open.value() as f64) * price_scale;
                            let close_y = (60000.0 - candle.ohlcv.close.value() as f64) * price_scale;
                            
                            let is_bullish = candle.ohlcv.close.value() >= candle.ohlcv.open.value();
                            let body_height = (open_y - close_y).abs() as f64;
                            let wick_height = high_y - low_y;
                            
                            (x, high_y, low_y, open_y, close_y, is_bullish, body_height, wick_height)
                        })
                        .collect();
                    results
                });
            },
        );
    }
    group.finish();
}

/// Benchmark для тестирования скорости доступа к данным
fn bench_data_access_patterns(c: &mut Criterion) {
    let mut group = c.benchmark_group("data_access_patterns");
    
    for candle_count in [1000, 5000, 10000].iter() {
        let chart = generate_test_chart(*candle_count);
        let candles = chart.data.get_candles();
        
        // Последовательный доступ
        group.bench_with_input(
            BenchmarkId::new("sequential_access", candle_count),
            candle_count,
            |b, _| {
                b.iter(|| {
                    let mut sum = 0.0;
                    for candle in candles.iter() {
                        sum += candle.ohlcv.close.value();
                    }
                    sum
                });
            },
        );
        
        // Параллельный доступ
        group.bench_with_input(
            BenchmarkId::new("parallel_access", candle_count),
            candle_count,
            |b, _| {
                b.iter(|| {
                    use rayon::prelude::*;
                    candles
                        .par_iter()
                        .map(|candle| candle.ohlcv.close.value())
                        .sum::<f32>()
                });
            },
        );
    }
    group.finish();
}

/// Memory allocation benchmark
fn bench_memory_allocation(c: &mut Criterion) {
    let mut group = c.benchmark_group("memory_allocation");
    
    for candle_count in [100, 1000, 10000].iter() {
        group.bench_with_input(
            BenchmarkId::new("vec_allocation", candle_count),
            candle_count,
            |b, _| {
                b.iter(|| {
                    let mut vec = Vec::with_capacity(*candle_count);
                    for i in 0..*candle_count {
                        vec.push((i as f64, i as f64 * 2.0, i as f64 * 3.0));
                    }
                    vec
                });
            },
        );
        
        group.bench_with_input(
            BenchmarkId::new("vec_pre_allocation", candle_count),
            candle_count,
            |b, _| {
                b.iter(|| {
                    let mut vec = Vec::with_capacity(*candle_count);
                    for i in 0..*candle_count {
                        vec.push((i as f64, i as f64 * 2.0, i as f64 * 3.0));
                    }
                    vec
                });
            },
        );
    }
    group.finish();
}

criterion_group!(
    rendering_benches,
    bench_calculate_coordinates_sequential,
    bench_calculate_coordinates_parallel,
    bench_sequential_vs_parallel,
    bench_data_access_patterns,
    bench_memory_allocation
);

criterion_main!(rendering_benches); 