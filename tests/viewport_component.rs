use price_chart_wasm::domain::chart::{Chart, value_objects::ChartType};
use price_chart_wasm::ecs::{
    EcsWorld,
    components::{ChartComponent, ViewportComponent},
};

#[test]
fn viewport_component_updates_on_zoom_pan() {
    let mut world = EcsWorld::new();
    let mut chart = Chart::new("test".into(), ChartType::Candlestick, 10);
    let entity = world.spawn_chart(chart.clone());

    chart.zoom(2.0, 0.5);
    chart.pan(0.1, 0.0);

    {
        let mut comp = world.world.get::<&mut ChartComponent>(entity).unwrap();
        comp.0 = chart.clone();
    }

    world.run_viewport_system();

    let vp = world.world.get::<&ViewportComponent>(entity).unwrap();
    assert_eq!(vp.0, chart.viewport);
}
