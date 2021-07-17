#[cfg(feature = "editor")]
use crate::editor::GameEditor;
use crate::{
	asset,
	engine::{
		Builder,
		Engine,
		Module,
	},
	graphics,
	GameState,
	RenderState,
};

use graphics::Graphics;

use asset::AssetManager;

use std::sync::{
	Mutex,
	RwLock,
};

pub struct Game {
	pub game_state: Mutex<GameState>,
	pub render_state: RwLock<Option<RenderState>>,
}

impl Module for Game {
	fn new() -> Self {
		Self {
			game_state: Mutex::new(GameState::new()),
			render_state: RwLock::new(None),
		}
	}

	fn depends_on(builder: Builder) -> Builder {
		let result = builder
			.tick(|engine: &Engine, dt: f32| {
				let game = engine.module::<Game>().unwrap();

				// Simualte the game state and then build the render state
				let new_render_state = {
					let mut game_state = game.game_state.lock().unwrap();
					game_state.simulate(dt)
				};

				{
					let mut render_state = game.render_state.write().unwrap();
					*render_state = Some(new_render_state);
				}
			})
			.module::<Graphics>()
			.module::<AssetManager>();

		#[cfg(feature = "editor")]
		let result = result.module::<GameEditor>();

		result
	}
}
