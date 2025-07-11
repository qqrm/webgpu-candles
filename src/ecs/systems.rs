use hecs::World;

use super::components::{CandleComponent, ChartComponent};
use leptos::SignalUpdate;

/// Apply new candles to all charts and remove processed candle entities.
pub fn apply_candles(world: &mut World) {
    let mut candle_entities = Vec::new();
    let candles: Vec<(hecs::Entity, CandleComponent)> =
        world.query::<&CandleComponent>().iter().map(|(e, c)| (e, c.clone())).collect();

    if candles.is_empty() {
        return;
    }

    for (_, candle) in &candles {
        for (_, chart) in world.query::<&mut ChartComponent>().iter() {
            chart.0.update(|c| c.add_realtime_candle(candle.0.clone()));
        }
    }

    candle_entities.extend(candles.into_iter().map(|(e, _)| e));
    for e in candle_entities {
        let _ = world.despawn(e);
    }
}
