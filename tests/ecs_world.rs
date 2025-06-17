use price_chart_wasm::ecs::EcsWorld;

#[test]
fn world_starts_empty() {
    let world = EcsWorld::new();
    assert_eq!(world.world.len(), 0);
}
