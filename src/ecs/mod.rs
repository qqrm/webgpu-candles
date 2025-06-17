use hecs::World;

pub mod components;

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
        self.world.spawn((ChartComponent(chart),))
    }
}
