use hecs::World;

pub mod components;
pub mod systems;

/// Shared ECS world for the application.
/// Currently only manages entities and components without systems.
#[derive(Default)]
pub struct EcsWorld {
    pub world: World,
}

impl EcsWorld {
    /// Create a new empty ECS world.
    pub fn new() -> Self {
        Self { world: World::new() }
    }

    /// Spawn a new chart entity with its component.
    pub fn spawn_chart(&mut self, chart: crate::domain::chart::Chart) -> hecs::Entity {
        use crate::ecs::components::{ChartComponent, ViewportComponent};
        let viewport = chart.viewport.clone();
        self.world.spawn((ChartComponent(chart), ViewportComponent(viewport)))
    }

    /// Apply all pending candle components to charts.
    pub fn run_candle_system(&mut self) {
        crate::ecs::systems::apply_candles(&mut self.world);
        crate::ecs::systems::update_viewports(&mut self.world);
    }

    /// Update viewport components from their charts.
    pub fn run_viewport_system(&mut self) {
        crate::ecs::systems::update_viewports(&mut self.world);
    }
}
