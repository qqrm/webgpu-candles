use super::*;
use crate::log_info;

impl WebGpuRenderer {
    pub(super) fn create_geometry(&self, chart: &Chart) -> (Vec<CandleVertex>, ChartUniforms) {
        let candles = chart.data.get_candles();
        if candles.is_empty() {
            log_info!(LogComponent::Infrastructure("WebGpuRenderer"), "⚠️ No candles to render");
            return (vec![], ChartUniforms::new());
        }

        // ⚡ Производительность: логируем реже
        if candles.len() % 100 == 0 {
            log_info!(
                LogComponent::Infrastructure("WebGpuRenderer"),
                "🔧 Creating optimized geometry for {} candles",
                candles.len()
            );
        }

        let candle_count = candles.len();
        let chart_width = 2.0; // NDC width (-1 to 1)
        let _chart_height = 2.0; // NDC height (-1 to 1)

        // 🔍 Применяем зум - показываем меньше свечей при увеличении зума
        let base_candles = 300.0;
        let visible_count = (base_candles / self.zoom_level)
            .max(10.0)
            .min(candle_count as f64) as usize;
        let start_index = if candle_count > visible_count {
            candle_count - visible_count
        } else {
            0
        };
        let visible_candles: Vec<Candle> = candles
            .iter()
            .skip(start_index)
            .cloned()
            .collect();

        let mut vertices = Vec::with_capacity(visible_candles.len() * 24);

        // Используем значения из viewport для вертикальной панорамировки
        let mut min_price = chart.viewport.min_price;
        let mut max_price = chart.viewport.max_price;
        if (max_price - min_price).abs() < f32::EPSILON {
            // Если диапазон равен нулю, вычисляем по данным
            for candle in &visible_candles {
                min_price = min_price.min(candle.ohlcv.low.value() as f32);
                max_price = max_price.max(candle.ohlcv.high.value() as f32);
            }

            let price_range = max_price - min_price;
            min_price -= price_range * 0.05;
            max_price += price_range * 0.05;
        }

        // Calculate visible candle width and spacing
        let spacing_ratio = 0.2; // 20% spacing between candles
        let step_size = chart_width / candle_count as f64;
        let max_candle_width = step_size * (1.0 - spacing_ratio);
        let _candle_width = max_candle_width.max(0.01).min(0.06); // Reasonable width limits

        log_info!(
            LogComponent::Infrastructure("WebGpuRenderer"),
            "📏 Price range: {:.2} - {:.2}, Candle width: {:.4}, step:{:.4}",
            min_price,
            max_price,
            _candle_width,
            step_size
        );

        // Ensure we have a valid price range
        if (max_price - min_price).abs() < 0.01 {
            get_logger().error(
                LogComponent::Infrastructure("WebGpuRenderer"),
                "❌ Invalid price range!",
            );
            return (vec![], ChartUniforms::new());
        }



        // Логируем реже для производительности
        if visible_candles.len() % 50 == 0 {
            log_info!(
                LogComponent::Infrastructure("WebGpuRenderer"),
                "🔧 Rendering {} candles (showing last {} of {}) [zoom: {:.2}x]",
                visible_candles.len(),
                visible_count,
                candles.len(),
                self.zoom_level
            );
        }

        // Create vertices for each visible candle
        let chart_width = 2.0; // NDC width (-1 to 1)

        // 🔍 Применяем зум к размеру свечей
        let base_step_size = chart_width / visible_candles.len() as f32;
        let zoom_factor = self.zoom_level.max(0.1).min(10.0) as f32; // Ограничиваем зум
        let step_size = base_step_size * zoom_factor; // При зуме > 1.0 свечи шире
        let candle_width = (step_size * 0.8).max(0.002).min(0.1); // Увеличиваем максимальную ширину

        for (i, candle) in visible_candles.iter().enumerate() {
            // Position X in NDC space [-1, 1] - новые свечи справа
            let x = -1.0 + (i as f32 + 0.5) * step_size;

            // Нормализация Y - используем верхнюю часть экрана [-0.5, 0.8] для свечей
            let price_range = max_price - min_price;
            let price_norm = |price: f64| -> f32 {
                let normalized = (price as f32 - min_price) / price_range;
                -0.5 + normalized * 1.3 // Map to [-0.5, 0.8] - освобождаем место для volume
            };

            let open_y = price_norm(candle.ohlcv.open.value());
            let high_y = price_norm(candle.ohlcv.high.value());
            let low_y = price_norm(candle.ohlcv.low.value());
            let close_y = price_norm(candle.ohlcv.close.value());

            // Логируем только первые 3 и последние 3 свечи
            if i < 3 || i >= visible_count - 3 {
                log_info!(
                    LogComponent::Infrastructure("WebGpuRenderer"),
                    "🕯️ Candle {}: x={:.3}, Y=({:.3},{:.3},{:.3},{:.3}) width={:.4}",
                    i,
                    x,
                    open_y,
                    high_y,
                    low_y,
                    close_y,
                    candle_width
                );
            }

            let half_width = candle_width * 0.5;
            let body_top = open_y.max(close_y);
            let body_bottom = open_y.min(close_y);

            // Минимальная высота для видимости
            let min_height = 0.005;
            let actual_body_top = if (body_top - body_bottom).abs() < min_height {
                body_bottom + min_height
            } else {
                body_top
            };

            let is_bullish = close_y >= open_y;

            // Тело свечи
            let body_vertices = vec![
                CandleVertex::body_vertex(x - half_width, body_bottom, is_bullish),
                CandleVertex::body_vertex(x + half_width, body_bottom, is_bullish),
                CandleVertex::body_vertex(x - half_width, actual_body_top, is_bullish),
                CandleVertex::body_vertex(x + half_width, body_bottom, is_bullish),
                CandleVertex::body_vertex(x + half_width, actual_body_top, is_bullish),
                CandleVertex::body_vertex(x - half_width, actual_body_top, is_bullish),
            ];
            vertices.extend_from_slice(&body_vertices);

            // Добавляем фитили (верхний и нижний)
            let wick_width = candle_width * 0.1; // Тонкие фитили
            let wick_half = wick_width * 0.5;

            // Верхний фитиль
            if high_y > actual_body_top {
                let upper_wick = vec![
                    CandleVertex::wick_vertex(x - wick_half, actual_body_top),
                    CandleVertex::wick_vertex(x + wick_half, actual_body_top),
                    CandleVertex::wick_vertex(x - wick_half, high_y),
                    CandleVertex::wick_vertex(x + wick_half, actual_body_top),
                    CandleVertex::wick_vertex(x + wick_half, high_y),
                    CandleVertex::wick_vertex(x - wick_half, high_y),
                ];
                vertices.extend_from_slice(&upper_wick);
            }

            // Нижний фитиль
            if low_y < body_bottom {
                let lower_wick = vec![
                    CandleVertex::wick_vertex(x - wick_half, low_y),
                    CandleVertex::wick_vertex(x + wick_half, low_y),
                    CandleVertex::wick_vertex(x - wick_half, body_bottom),
                    CandleVertex::wick_vertex(x + wick_half, low_y),
                    CandleVertex::wick_vertex(x + wick_half, body_bottom),
                    CandleVertex::wick_vertex(x - wick_half, body_bottom),
                ];
                vertices.extend_from_slice(&lower_wick);
            }
        }

        // Добавляем сплошную линию текущей цены
        if let Some(last_candle) = visible_candles.last() {
            let current_price = last_candle.ohlcv.close.value() as f32;
            let price_range = max_price - min_price;
            let price_y = -0.5 + ((current_price - min_price) / price_range) * 1.3; // Та же область что и свечи

            // Сплошная горизонтальная линия через весь экран
            let line_thickness = 0.002;
            let price_line = vec![
                CandleVertex::current_price_vertex(-1.0, price_y - line_thickness),
                CandleVertex::current_price_vertex(1.0, price_y - line_thickness),
                CandleVertex::current_price_vertex(-1.0, price_y + line_thickness),
                CandleVertex::current_price_vertex(1.0, price_y - line_thickness),
                CandleVertex::current_price_vertex(1.0, price_y + line_thickness),
                CandleVertex::current_price_vertex(-1.0, price_y + line_thickness),
            ];
            vertices.extend_from_slice(&price_line);
        }

        // 📊 Добавляем сетку графика для профессионального вида
        vertices.extend(self.create_grid_lines(min_price, max_price, visible_candles.len()));

        // 📊 Добавляем volume bars под графиком
        vertices.extend(self.create_volume_bars(&visible_candles));

        // 📈 Добавляем скользящие средние (SMA20 и EMA12)
        vertices.extend(self.create_moving_averages(&visible_candles, min_price, max_price));

        // Логируем только если много вершин
        if vertices.len() > 1000 {
            log_info!(
                LogComponent::Infrastructure("WebGpuRenderer"),
                "✅ Generated {} vertices for {} visible candles + indicators",
                vertices.len(),
                visible_candles.len()
            );
        }

        // Identity matrix - vertices are already in NDC coordinates [-1, 1]
        let view_proj_matrix = [
            [1.0, 0.0, 0.0, 0.0],
            [0.0, 1.0, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
            [0.0, 0.0, 0.0, 1.0],
        ];

        // Create uniforms with corrected parameters
        let uniforms = ChartUniforms {
            view_proj_matrix,
            viewport: [self.width as f32, self.height as f32, min_price, max_price],
            time_range: [
                0.0,
                visible_candles.len() as f32,
                visible_candles.len() as f32,
                0.0,
            ],
            bullish_color: [0.447, 0.776, 0.522, 1.0], // #72c685 - зеленый
            bearish_color: [0.882, 0.420, 0.282, 1.0], // #e16b48 - красный
            wick_color: [0.6, 0.6, 0.6, 0.9],          // Светло-серый
            sma20_color: [1.0, 0.2, 0.2, 0.9],         // Ярко-красный
            sma50_color: [1.0, 0.8, 0.0, 0.9],         // Желтый
            sma200_color: [0.2, 0.4, 0.8, 0.9],        // Синий
            ema12_color: [0.8, 0.2, 0.8, 0.9],         // Фиолетовый
            ema26_color: [0.0, 0.8, 0.8, 0.9],         // Голубой
            current_price_color: [1.0, 1.0, 0.0, 0.8], // 💰 Ярко-желтый
            render_params: [candle_width as f32, spacing_ratio as f32, 0.004, 0.0],
        };

        (vertices, uniforms)
    }

    /// 📈 Создать геометрию для скользящих средних
    fn create_moving_averages(
        &self,
        candles: &[crate::domain::market_data::Candle],
        min_price: f32,
        max_price: f32,
    ) -> Vec<CandleVertex> {
        use crate::infrastructure::rendering::gpu_structures::{CandleGeometry, IndicatorType};

        if candles.len() < 20 {
            return Vec::new(); // Недостаточно данных для SMA20
        }

        let mut vertices = Vec::with_capacity(candles.len() * 6);
        let candle_count = candles.len();
        let step_size = 2.0 / candle_count as f32;
        let price_range = max_price - min_price;

        // Функция для нормализации цены в NDC координаты
        let price_to_ndc = |price: f32| -> f32 { -0.8 + ((price - min_price) / price_range) * 1.6 };

        // Расчёт SMA20 (Simple Moving Average 20)
        let mut sma20_points = Vec::with_capacity(candles.len().saturating_sub(19));
        for i in 19..candle_count {
            // Начинаем с 20-й свечи
            let sum: f32 = candles[i - 19..=i]
                .iter()
                .map(|c| c.ohlcv.close.value() as f32)
                .sum();
            let sma20 = sum / 20.0;
            let x = -1.0 + (i as f32 + 0.5) * step_size;
            let y = price_to_ndc(sma20);
            sma20_points.push((x, y));
        }

        // Расчёт EMA12 (Exponential Moving Average 12)
        let mut ema12_points = Vec::with_capacity(candles.len().saturating_sub(11));
        if candle_count >= 12 {
            let multiplier = 2.0 / (12.0 + 1.0); // EMA multiplier
            let mut ema = candles[0].ohlcv.close.value() as f32; // Начальное значение

            for i in 1..candle_count {
                let close = candles[i].ohlcv.close.value() as f32;
                ema = (close * multiplier) + (ema * (1.0 - multiplier));

                if i >= 11 {
                    // Показываем EMA только после 12 свечей
                    let x = -1.0 + (i as f32 + 0.5) * step_size;
                    let y = price_to_ndc(ema);
                    ema12_points.push((x, y));
                }
            }
        }

        // Создаём геометрию для линий
        if !sma20_points.is_empty() {
            let sma20_vertices = CandleGeometry::create_indicator_line_vertices(
                &sma20_points,
                IndicatorType::SMA20,
                0.003, // Толщина линии
            );
            vertices.extend(sma20_vertices);
        }

        if !ema12_points.is_empty() {
            let ema12_vertices = CandleGeometry::create_indicator_line_vertices(
                &ema12_points,
                IndicatorType::EMA12,
                0.003, // Толщина линии
            );
            vertices.extend(ema12_vertices);
        }

        if !vertices.is_empty() {
            log_info!(
                LogComponent::Infrastructure("WebGpuRenderer"),
                "📈 Generated {} SMA20 points, {} EMA12 points, {} total MA vertices",
                sma20_points.len(),
                ema12_points.len(),
                vertices.len()
            );
        }

        vertices
    }

    /// 📊 Создать сетку графика (горизонтальные и вертикальные линии)
    fn create_grid_lines(
        &self,
        min_price: f32,
        max_price: f32,
        candle_count: usize,
    ) -> Vec<CandleVertex> {
        let num_price_lines = 8; // 8 горизонтальных линий
        let num_vertical_lines = 10; // 10 вертикальных линий
        let mut vertices = Vec::with_capacity((num_price_lines + num_vertical_lines) * 6);
        let line_thickness = 0.001; // Тонкие линии сетки

        // Горизонтальные линии сетки (ценовые уровни)
        let price_range = max_price - min_price;

        for i in 1..num_price_lines {
            let price_level = min_price + (price_range * i as f32 / num_price_lines as f32);
            let y = -0.5 + ((price_level - min_price) / price_range) * 1.3; // Та же область что и свечи

            // Горизонтальная линия через весь график
            let horizontal_line = vec![
                CandleVertex::grid_vertex(-1.0, y - line_thickness),
                CandleVertex::grid_vertex(1.0, y - line_thickness),
                CandleVertex::grid_vertex(-1.0, y + line_thickness),
                CandleVertex::grid_vertex(1.0, y - line_thickness),
                CandleVertex::grid_vertex(1.0, y + line_thickness),
                CandleVertex::grid_vertex(-1.0, y + line_thickness),
            ];
            vertices.extend_from_slice(&horizontal_line);
        }

        // Вертикальные линии сетки (временные интервалы) - покрывают весь график
        if candle_count > 0 {
            let step_size = 2.0 / candle_count as f32;
            let num_vertical_lines = 10; // 10 вертикальных линий
            let vertical_step = candle_count / num_vertical_lines;

            for i in 1..num_vertical_lines {
                let candle_index = i * vertical_step;
                if candle_index < candle_count {
                    let x = -1.0 + (candle_index as f32 + 0.5) * step_size;

                    // Вертикальная линия через весь график (включая volume область)
                    let vertical_line = vec![
                        CandleVertex::grid_vertex(x - line_thickness, -1.0), //От самого низа
                        CandleVertex::grid_vertex(x + line_thickness, -1.0),
                        CandleVertex::grid_vertex(x - line_thickness, 0.8), //До верха свечей
                        CandleVertex::grid_vertex(x + line_thickness, -1.0),
                        CandleVertex::grid_vertex(x + line_thickness, 0.8),
                        CandleVertex::grid_vertex(x - line_thickness, 0.8),
                    ];
                    vertices.extend_from_slice(&vertical_line);
                }
            }
        }

        log_info!(
            LogComponent::Infrastructure("WebGpuRenderer"),
            "📊 Generated {} grid vertices",
            vertices.len()
        );

        vertices
    }

    /// 📊 Создать volume bars под основным графиком
    fn create_volume_bars(
        &self,
        candles: &[crate::domain::market_data::Candle],
    ) -> Vec<CandleVertex> {
        if candles.is_empty() {
            return Vec::new();
        }

        let candle_count = candles.len();
        let mut vertices = Vec::with_capacity(candle_count * 6);

        // Находим максимальный объем для нормализации
        let max_volume = candles
            .iter()
            .map(|c| c.ohlcv.volume.value() as f32)
            .fold(0.0f32, |a, b| a.max(b));

        if max_volume <= 0.0 {
            return Vec::new();
        }

        // Volume область занимает нижнюю часть экрана [-1.0, -0.6]
        let volume_top = -0.6;
        let volume_bottom = -1.0;
        let volume_height = volume_top - volume_bottom;

        let step_size = 2.0 / candle_count as f32;
        let bar_width = (step_size * 0.8).max(0.002); // 80% от step_size

        for (i, candle) in candles.iter().enumerate() {
            let x = -1.0 + (i as f32 + 0.5) * step_size;
            let volume_normalized = (candle.ohlcv.volume.value() as f32) / max_volume;
            let bar_height = volume_height * volume_normalized;
            let bar_top = volume_bottom + bar_height;

            let half_width = bar_width * 0.5;

            // Определяем цвет volume bar: зеленый если цена выросла, красный если упала
            let is_bullish = candle.ohlcv.close.value() >= candle.ohlcv.open.value();

            // Volume bar как прямоугольник (2 треугольника)
            let volume_bar = vec![
                CandleVertex::volume_vertex(x - half_width, volume_bottom, is_bullish),
                CandleVertex::volume_vertex(x + half_width, volume_bottom, is_bullish),
                CandleVertex::volume_vertex(x - half_width, bar_top, is_bullish),
                CandleVertex::volume_vertex(x + half_width, volume_bottom, is_bullish),
                CandleVertex::volume_vertex(x + half_width, bar_top, is_bullish),
                CandleVertex::volume_vertex(x - half_width, bar_top, is_bullish),
            ];
            vertices.extend_from_slice(&volume_bar);
        }

        log_info!(
            LogComponent::Infrastructure("WebGpuRenderer"),
            "📊 Generated {} volume vertices for {} candles (max volume: {:.2})",
            vertices.len(),
            candles.len(),
            max_volume
        );

        vertices
    }

    fn create_candles(&self, candles: &[Candle]) -> Vec<CandleVertex> {
        let mut vertices = Vec::with_capacity(candles.len() * 12);
        if candles.is_empty() {
            return vertices;
        }

        // 🔍 Применяем зум - показываем меньше свечей при увеличении зума
        let visible_count = (300.0 / self.zoom_level).max(10.0) as usize;
        let start_idx = if candles.len() > visible_count {
            candles.len() - visible_count
        } else {
            0
        };
        let visible_candles = &candles[start_idx..];

        if visible_candles.is_empty() {
            return vertices;
        }

        // Находим мин/макс цены для нормализации
        let (min_price, max_price) =
            visible_candles
                .iter()
                .fold((f64::MAX, f64::MIN), |(min, max), candle| {
                    let low = candle.ohlcv.low.value();
                    let high = candle.ohlcv.high.value();
                    (min.min(low), max.max(high))
                });

        let price_range = max_price - min_price;
        if price_range == 0.0 {
            return vertices;
        }

        // 🔍 Учитываем панорамирование при расчете step_size
        let base_step_size = 2.0 / visible_candles.len() as f64;
        let step_size = base_step_size * self.zoom_level;

        // 🔍 Применяем панорамирование
        let pan_factor = self.pan_offset * 0.001; // Чувствительность панорамирования

        for (i, candle) in visible_candles.iter().enumerate() {
            // 🔍 Позиция X с учетом зума и панорамирования
            let base_x = -1.0 + (i as f64 + 0.5) * base_step_size;
            let x = (base_x + pan_factor).clamp(-1.0, 1.0);

            // Нормализуем цены в диапазон [-0.5, 0.8] (освобождаем место для volume bars)
            let normalize_price = |price: f64| -> f32 {
                let normalized = (price - min_price) / price_range;
                (-0.5 + normalized * 1.3) as f32
            };

            let open_y = normalize_price(candle.ohlcv.open.value());
            let high_y = normalize_price(candle.ohlcv.high.value());
            let low_y = normalize_price(candle.ohlcv.low.value());
            let close_y = normalize_price(candle.ohlcv.close.value());

            // 🔍 Ширина свечи с учетом зума
            let candle_width = (step_size * 0.6) as f32;

            // Цвет свечи (зеленый для роста, красный для падения)
            let _color = if candle.ohlcv.close.value() >= candle.ohlcv.open.value() {
                [0.0, 0.8, 0.0, 1.0]
            } else {
                [0.8, 0.0, 0.0, 1.0]
            };

            // Создаем геометрию свечи (body + wicks)
            let x_f32 = x as f32;

            // High-Low wick (тонкая линия)
            vertices.push(CandleVertex::wick_vertex(x_f32, high_y));
            vertices.push(CandleVertex::wick_vertex(x_f32, low_y));

            // Open-Close body (толстый прямоугольник)
            let body_top = open_y.max(close_y);
            let body_bottom = open_y.min(close_y);
            let is_bullish = candle.ohlcv.close.value() >= candle.ohlcv.open.value();

            // Левая сторона body
            vertices.push(CandleVertex::body_vertex(
                x_f32 - candle_width / 2.0,
                body_top,
                is_bullish,
            ));
            vertices.push(CandleVertex::body_vertex(
                x_f32 - candle_width / 2.0,
                body_bottom,
                is_bullish,
            ));

            // Правая сторона body
            vertices.push(CandleVertex::body_vertex(
                x_f32 + candle_width / 2.0,
                body_top,
                is_bullish,
            ));
            vertices.push(CandleVertex::body_vertex(
                x_f32 + candle_width / 2.0,
                body_bottom,
                is_bullish,
            ));
        }

        vertices
    }
}
