use engine::{Engine, EngineBuilder, Module};
use game::Game;
use math::Color;
use newport::*;

struct GameExample;

// Implement the module trait
impl Module for GameExample {
    fn new() -> Self {
        Self
    }

    fn depends_on(builder: EngineBuilder) -> EngineBuilder {
        builder.module::<Game>()
    }
}

// Start the app runner
fn main() {
    let builder = EngineBuilder::new()
        .module::<GameExample>()
        .name("Game Example");
    Engine::run(builder);
}
