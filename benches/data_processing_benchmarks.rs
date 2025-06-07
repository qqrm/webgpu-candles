use criterion::{criterion_group, criterion_main, Criterion, BenchmarkId};
use price_chart_wasm::domain::{
    market_data::{
        entities::{Candle, CandleSeries},
        value_objects::{Price, Volume, Timestamp, OHLCV},
        services::{MarketAnalysisService, DataValidationService},
    },
};
use std::time::Duration;

/// Генерирует тестовые данные свечей для бенчмарков обработки данных
fn generate_test_candles(count: usize) -> Vec<Candle> {
    let mut candles = Vec::with_capacity(count);
    let mut base_price = 50000.0;
    
    for i in 0..count {
        let timestamp = Timestamp::from(1640000000 + (i as u64 * 60));
        
        // Реалистичные колебания цены
        let trend = (i as f32 * 0.001).sin() * 1000.0; // Долгосрочный тренд
        let volatility = (i as f32 * 0.1).sin() * 200.0; // Волатильность
        let noise = ((i as f32 * 0.5).sin() + (i as f32 * 1.2).cos()) * 50.0; // Шум
        
        let open = base_price + trend + volatility + noise;
        let close = open + ((i as f32 * 0.3).cos() * 100.0);
        let high = open.max(close) + ((i as f32 * 0.7).sin().abs() * 150.0);
        let low = open.min(close) - ((i as f32 * 0.9).cos().abs() * 120.0);
        let volume = 1000.0 + ((i as f32 * 0.4).sin().abs() * 2000.0);
        
        let ohlcv = OHLCV::new(
            Price::from(open),
            Price::from(high),
            Price::from(low),
            Price::from(close),
            Volume::from(volume),
        );
        
        candles.push(Candle::new(timestamp, ohlcv));
        base_price = close * 0.999 + open * 0.001; // Медленный дрейф цены
    }
    
    candles
}

/// Benchmark добавления свечей в серию
fn bench_candle_series_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("candle_series_operations");
    group.measurement_time(Duration::from_secs(10));
    
    for candle_count in [100, 500, 1000, 5000, 10000].iter() {
        let candles = generate_test_candles(*candle_count);
        
        // Benchmark добавления свечей
        group.bench_with_input(
            BenchmarkId::new("add_candles", candle_count),
            candle_count,
            |b, _| {
                b.iter(|| {
                    let mut series = CandleSeries::new(*candle_count);
                    for candle in &candles {
                        series.add_candle(candle.clone());
                    }
                    series
                });
            },
        );
        
        // Benchmark получения последних свечей
        let series = {
            let mut series = CandleSeries::new(*candle_count);
            for candle in &candles {
                series.add_candle(candle.clone());
            }
            series
        };
        
        group.bench_with_input(
            BenchmarkId::new("get_latest_candle", candle_count),
            candle_count,
            |b, _| {
                b.iter(|| {
                    series.latest()
                });
            },
        );
        
        // Benchmark поиска ценового диапазона
        group.bench_with_input(
            BenchmarkId::new("price_range", candle_count),
            candle_count,
            |b, _| {
                b.iter(|| {
                    series.price_range()
                });
            },
        );
    }
    group.finish();
}

/// Benchmark аналитических операций
fn bench_market_analysis(c: &mut Criterion) {
    let mut group = c.benchmark_group("market_analysis");
    group.measurement_time(Duration::from_secs(15));
    
    let analysis_service = MarketAnalysisService::new();
    
    for candle_count in [100, 500, 1000, 2000, 5000].iter() {
        let candles = generate_test_candles(*candle_count);
        
        // Benchmark Simple Moving Average
        for period in [10, 20, 50, 100].iter() {
            if *period <= *candle_count {
                group.bench_with_input(
                    BenchmarkId::new("sma", format!("{}c_{}p", candle_count, period)),
                    &(candle_count, period),
                    |b, _| {
                        b.iter(|| {
                            analysis_service.calculate_sma(&candles, *period)
                        });
                    },
                );
            }
        }
        
        // Benchmark волатильности
        group.bench_with_input(
            BenchmarkId::new("volatility", candle_count),
            candle_count,
            |b, _| {
                b.iter(|| {
                    analysis_service.calculate_volatility(&candles, 20.min(*candle_count))
                });
            },
        );
        
        // Benchmark поиска экстремумов
        group.bench_with_input(
            BenchmarkId::new("extremes", candle_count),
            candle_count,
            |b, _| {
                b.iter(|| {
                    analysis_service.find_extremes(&candles, 5) // window size = 5
                });
            },
        );
    }
    group.finish();
}

/// Benchmark валидации данных
fn bench_data_validation(c: &mut Criterion) {
    let mut group = c.benchmark_group("data_validation");
    
    let validation_service = DataValidationService::new();
    
    for candle_count in [100, 1000, 5000, 10000].iter() {
        let candles = generate_test_candles(*candle_count);
        
        // Benchmark валидации отдельных свечей
        group.bench_with_input(
            BenchmarkId::new("validate_individual_candles", candle_count),
            candle_count,
            |b, _| {
                b.iter(|| {
                    let mut valid_count = 0;
                    for candle in &candles {
                        if validation_service.validate_candle(candle).is_ok() {
                            valid_count += 1;
                        }
                    }
                    valid_count
                });
            },
        );
        
        // Benchmark валидации последовательности
        group.bench_with_input(
            BenchmarkId::new("validate_sequence", candle_count),
            candle_count,
            |b, _| {
                b.iter(|| {
                    validation_service.validate_candle_sequence(&candles)
                });
            },
        );
    }
    group.finish();
}

/// Benchmark обработки больших объемов данных
fn bench_large_data_processing(c: &mut Criterion) {
    let mut group = c.benchmark_group("large_data_processing");
    group.measurement_time(Duration::from_secs(20));
    
    // Тест с очень большими датасетами
    for candle_count in [10000, 50000, 100000].iter() {
        let candles = generate_test_candles(*candle_count);
        
        // Sequential price aggregation
        group.bench_with_input(
            BenchmarkId::new("sequential_price_avg", candle_count),
            candle_count,
            |b, _| {
                b.iter(|| {
                    let mut sum = 0.0;
                    for candle in &candles {
                        sum += candle.ohlcv.close.value();
                    }
                    sum / candles.len() as f32
                });
            },
        );
        
        // Parallel price aggregation
        group.bench_with_input(
            BenchmarkId::new("parallel_price_avg", candle_count),
            candle_count,
            |b, _| {
                b.iter(|| {
                    use rayon::prelude::*;
                    let sum: f32 = candles
                        .par_iter()
                        .map(|candle| candle.ohlcv.close.value())
                        .sum();
                    sum / candles.len() as f32
                });
            },
        );
        
        // Sequential max/min search
        group.bench_with_input(
            BenchmarkId::new("sequential_extremes", candle_count),
            candle_count,
            |b, _| {
                b.iter(|| {
                    let mut min_price = f32::INFINITY;
                    let mut max_price = f32::NEG_INFINITY;
                    for candle in &candles {
                        min_price = min_price.min(candle.ohlcv.low.value());
                        max_price = max_price.max(candle.ohlcv.high.value());
                    }
                    (min_price, max_price)
                });
            },
        );
        
        // Parallel max/min search
        group.bench_with_input(
            BenchmarkId::new("parallel_extremes", candle_count),
            candle_count,
            |b, _| {
                b.iter(|| {
                    use rayon::prelude::*;
                    
                    let min_price = candles
                        .par_iter()
                        .map(|candle| candle.ohlcv.low.value())
                        .reduce(|| f32::INFINITY, |a, b| a.min(b));
                        
                    let max_price = candles
                        .par_iter()
                        .map(|candle| candle.ohlcv.high.value())
                        .reduce(|| f32::NEG_INFINITY, |a, b| a.max(b));
                        
                    (min_price, max_price)
                });
            },
        );
    }
    group.finish();
}

/// Memory allocation patterns benchmark
fn bench_memory_patterns(c: &mut Criterion) {
    let mut group = c.benchmark_group("memory_patterns");
    
    for candle_count in [1000, 10000, 50000].iter() {
        // Pre-allocated vs dynamic allocation
        group.bench_with_input(
            BenchmarkId::new("vec_push_no_reserve", candle_count),
            candle_count,
            |b, _| {
                b.iter(|| {
                    let mut vec = Vec::new();
                    for i in 0..*candle_count {
                        vec.push(i as f32);
                    }
                    vec
                });
            },
        );
        
        group.bench_with_input(
            BenchmarkId::new("vec_push_with_reserve", candle_count),
            candle_count,
            |b, _| {
                b.iter(|| {
                    let mut vec = Vec::with_capacity(*candle_count);
                    for i in 0..*candle_count {
                        vec.push(i as f32);
                    }
                    vec
                });
            },
        );
        
        // Memory copying patterns
        let test_data: Vec<f32> = (0..*candle_count).map(|i| i as f32).collect();
        
        group.bench_with_input(
            BenchmarkId::new("vec_clone", candle_count),
            candle_count,
            |b, _| {
                b.iter(|| {
                    test_data.clone()
                });
            },
        );
        
        group.bench_with_input(
            BenchmarkId::new("vec_iter_collect", candle_count),
            candle_count,
            |b, _| {
                b.iter(|| {
                    test_data.iter().cloned().collect::<Vec<f32>>()
                });
            },
        );
    }
    group.finish();
}

criterion_group!(
    data_benches,
    bench_candle_series_operations,
    bench_market_analysis,
    bench_data_validation,
    bench_large_data_processing,
    bench_memory_patterns
);

criterion_main!(data_benches); 