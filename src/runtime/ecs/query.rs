use crate::{
	Component,
	Entity,
	EntityInfo,
	ReadStorage,
	World,
	WriteStorage,
};

#[derive(Default, Clone)]
pub struct Query {
	info: EntityInfo,
}

impl Query {
	pub fn new() -> Self {
		Self {
			info: EntityInfo::default(),
		}
	}

	#[must_use]
	pub fn read<T: Component>(mut self, _: &ReadStorage<'_, T>) -> Self {
		self.info.components |= T::VARIANT_ID.to_mask();
		self
	}

	#[must_use]
	pub fn write<T: Component>(mut self, _: &WriteStorage<'_, T>) -> Self {
		self.info.components |= T::VARIANT_ID.to_mask();
		self
	}

	pub fn execute(self, world: &World) -> Vec<Entity> {
		let entities = world.entities.lock().unwrap();
		entities
			.iter()
			.filter(|(_, info)| (info.components & self.info.components) == self.info.components)
			.map(|(id, _)| *id)
			.collect()
	}
}
