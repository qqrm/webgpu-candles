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
        use crate::ecs::components::ChartComponent;
        use leptos::create_rw_signal;

        let signal = create_rw_signal(chart);
        self.world.spawn((ChartComponent(signal),))
    }

    /// Apply all pending candle components to charts.
    pub fn run_candle_system(&mut self) {
        crate::ecs::systems::apply_candles(&mut self.world);
    }

    /// Run the candle system in parallel when supported.
    pub fn run_candle_system_parallel(&mut self) {
        crate::ecs::systems::apply_candles_parallel(&mut self.world);
    }
}
