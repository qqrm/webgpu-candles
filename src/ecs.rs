use hecs::World;

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
}
