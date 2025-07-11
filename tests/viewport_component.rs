use leptos::SignalUpdate;
use price_chart_wasm::domain::chart::{Chart, value_objects::ChartType};
use price_chart_wasm::ecs::{
    EcsWorld,
    components::{ChartComponent, ViewportComponent},
};

#[test]
fn spawn_chart_has_viewport_component() {
    let mut world = EcsWorld::new();
    let chart = Chart::new("test".into(), ChartType::Candlestick, 100);
    let entity = world.spawn_chart(chart.clone());

    let vp = world.world.get::<&ViewportComponent>(entity).expect("viewport component exists");
    assert_eq!(vp.0, chart.viewport);
}

#[test]
fn viewport_updates_after_zoom() {
    let mut world = EcsWorld::new();
    let chart = Chart::new("test".into(), ChartType::Candlestick, 100);
    let entity = world.spawn_chart(chart);

    {
        let chart_comp = world.world.get::<&mut ChartComponent>(entity).unwrap();
        chart_comp.0.update(|c| c.zoom(2.0, 0.5));
    }

    world.run_viewport_system();

    let vp = world.world.get::<&ViewportComponent>(entity).unwrap();
    assert!((vp.0.end_time - vp.0.start_time - 50.0).abs() < 1e-6);
}
