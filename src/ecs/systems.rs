use hecs::World;

use super::components::ViewportComponent;
use super::components::{CandleComponent, ChartComponent};
use leptos::{SignalUpdate, SignalWith};

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

/// Apply candles using Rayon for parallel chart updates on native targets.
#[cfg(all(not(target_arch = "wasm32"), feature = "parallel"))]
pub fn apply_candles_parallel(world: &mut World) {
    use rayon::prelude::*;

    let candles: Vec<CandleComponent> =
        world.query::<&CandleComponent>().iter().map(|(_, c)| c.clone()).collect();

    if candles.is_empty() {
        return;
    }

    let chart_entities: Vec<hecs::Entity> =
        world.query::<&ChartComponent>().iter().map(|(e, _)| e).collect();
    let mut charts: Vec<ChartComponent> = chart_entities
        .iter()
        .filter_map(|&e| world.get::<&ChartComponent>(e).ok().map(|c| c.clone()))
        .collect();

    charts.par_iter_mut().for_each(|comp| {
        for candle in &candles {
            comp.0.add_realtime_candle(candle.0.clone());
        }
    });

    for (entity, updated) in chart_entities.into_iter().zip(charts.into_iter()) {
        if let Ok(mut comp) = world.get::<&mut ChartComponent>(entity) {
            comp.0 = updated.0;
        }
    }

    let candle_entities: Vec<hecs::Entity> =
        world.query::<&CandleComponent>().iter().map(|(e, _)| e).collect();
    for e in candle_entities {
        let _ = world.despawn(e);
    }
}

#[cfg(any(target_arch = "wasm32", not(feature = "parallel")))]
pub fn apply_candles_parallel(world: &mut World) {
    apply_candles(world);
}

/// Update `ViewportComponent` to match the chart's viewport.
pub fn sync_viewports(world: &mut World) {
    let mut query = world.query::<(&ChartComponent, &mut ViewportComponent)>();
    for (_, (chart, viewport)) in query.iter() {
        let vp = chart.0.with(|c| c.viewport.clone());
        viewport.0 = vp;
    }
}
