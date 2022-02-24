use {
	ecs::{
		Component,
		Named,
		Query,
		ScheduleBlock,
		System,
		World,
	},
	engine::{
		define_run_module,
		Builder,
		Engine,
		Module,
	},
	game3d::*,
	input::*,
	math::{
		Quat,
		Vec3,
	},
	physics3d::*,
	resources::Handle,
	serde::{
		Deserialize,
		Serialize,
	},
};

pub struct Orchard;
impl Module for Orchard {
	fn new() -> Self {
		let game: &Game = Engine::module().unwrap();
		{
			let mut schedule = game.schedule.lock().unwrap();
			*schedule = ScheduleBlock::new()
				.system(InputSystem)
				.system(DebugSystem)
				.system(PlayerCharacterControllerSystem)
				.system(CharacterMovementSystem)
				.system(PhysicsSystem)
				.system(EditorCameraSystem);
		}

		let world = &game.world;

		let mut transforms = world.write::<Transform>();
		let mut filters = world.write::<MeshFilter>();
		let mut cameras = world.write::<Camera>();
		let mut names = world.write::<Named>();
		let mut camera_controllers = world.write::<EditorCameraController>();
		let mut colliders = world.write::<Collider>();
		let mut rigid_bodies = world.write::<RigidBody>();
		let mut character_movements = world.write::<CharacterMovement>();
		let mut player_character_controllers = world.write::<PlayerCharacterController>();

		let pipeline = Handle::find_or_load("{D0FAF8AC-0650-48D1-AAC2-E1C01E1C93FC}").unwrap();

		// Character Body
		let character = world
			.spawn(world.persistent)
			.with(Named::new("Character"), &mut names)
			.with(
				Transform::builder().location([0.0, -5.0, 1.0]).finish(),
				&mut transforms,
			)
			.with(
				Collider::builder(Shape::capsule(1.0, 0.3)).build(),
				&mut colliders,
			)
			.with(
				RigidBody::builder(RigidBodyVariant::Kinematic).build(),
				&mut rigid_bodies,
			)
			.with(CharacterMovement::default(), &mut character_movements)
			.with(
				PlayerCharacterController::default(),
				&mut player_character_controllers,
			)
			.finish();

		world
			.spawn(world.persistent)
			.with(Named::new("Camera"), &mut names)
			.with(
				Transform::builder()
					.parent(character)
					.location([0.0, 0.0, 1.0])
					.finish(),
				&mut transforms,
			)
			.with(Camera::default(), &mut cameras)
			.finish();

		for x in 0..10 {
			for y in 0..10 {
				let z = ((x + y) * 2) as f32;
				let x = x as f32 / 2.0;
				let y = y as f32 / 2.0;
				world
					.spawn(world.persistent)
					.with(Named::new("Block"), &mut names)
					.with(
						Transform::builder()
							.location(Vec3::new(x, y, z + 5.0))
							.finish(),
						&mut transforms,
					)
					.with(
						MeshFilter {
							mesh: Handle::find_or_load("{03383b92-566f-4036-aeb4-850b61685ea6}")
								.unwrap(),
							pipeline: pipeline.clone(),
						},
						&mut filters,
					)
					.with(
						Collider::builder(Shape::cube(Vec3::ONE / 2.0)).build(),
						&mut colliders,
					)
					.with(
						RigidBody::builder(RigidBodyVariant::Dynamic).build(),
						&mut rigid_bodies,
					)
					.finish();
			}
		}

		let floor_size = Vec3::new(10000.0, 10000.0, 0.1);
		world
			.spawn(world.persistent)
			.with(Named::new("Floor"), &mut names)
			.with(
				Transform::builder().scale(floor_size).finish(),
				&mut transforms,
			)
			.with(
				MeshFilter {
					mesh: Handle::find_or_load("{03383b92-566f-4036-aeb4-850b61685ea6}").unwrap(),
					pipeline,
				},
				&mut filters,
			)
			.with(
				Collider::builder(Shape::cube(floor_size / 2.0)).build(),
				&mut colliders,
			)
			// .with(
			// 	RigidBody::builder(RigidBodyVariant::Static).build(),
			// 	&mut rigid_bodies,
			// )
			.finish();

		Self
	}

	fn depends_on(builder: &mut Builder) -> &mut Builder {
		builder
			.module::<Game>()
			.module::<Physics>()
			.register(CharacterMovement::variant())
			.register(PlayerCharacterController::variant())
	}
}

#[derive(Default, Serialize, Deserialize, Clone)]
pub struct CharacterMovement {
	pub input: Vec3,
	pub jump_pressed: bool,

	pub velocity: Vec3,
}

impl Component for CharacterMovement {}

#[derive(Clone)]
pub struct CharacterMovementSystem;
impl System for CharacterMovementSystem {
	fn run(&self, world: &World, dt: f32) {}
}

#[derive(Default, Serialize, Deserialize, Clone)]
pub struct PlayerCharacterController {
	pub yaw: f32,
	pub pitch: f32,
}

impl Component for PlayerCharacterController {}

#[derive(Clone)]
pub struct PlayerCharacterControllerSystem;
impl System for PlayerCharacterControllerSystem {
	fn run(&self, world: &World, dt: f32) {
		let input = world.read::<InputManager>();
		let input = input.get(world.singleton).unwrap();

		// let physics = world.write::<PhysicsManager>();
		// let physics = physics.get_mut(world.singleton).unwrap();

		// Query for all controllers that could be functioning
		let transforms = world.write::<Transform>();
		let controllers = world.write::<PlayerCharacterController>();
		let entities = Query::new()
			.write(&transforms)
			.write(&controllers)
			.execute(world);

		// Essentially all we're doing is handling inputs and updating transforms
		for e in entities.iter().copied() {
			let mut transform = transforms.get_mut(e).unwrap();
			let mut controller = controllers.get_mut(e).unwrap();

			// Update the camera controller rotation only when mouse input is being consumed
			const SENSITIVITY: f32 = 0.3;
			controller.pitch -= input.current_axis1d(MOUSE_AXIS_Y) * SENSITIVITY;
			controller.yaw += input.current_axis1d(MOUSE_AXIS_X) * SENSITIVITY;
			transform.set_local_rotation(Quat::from_euler([0.0, controller.yaw, 0.0]));

			// TODO: Update the camera local pitch

			// Determine the current movement speed
			const WALK_SPEED: f32 = 6.0;
			// const SPRINT_SPEED: f32 = 20.0;
			let speed = WALK_SPEED;

			// Move camera forward and right axis. Up and down on world UP
			let rotation = transform.local_rotation();
			let forward = rotation.forward();
			let right = rotation.right();

			let mut delta = Vec3::ZERO;
			if input.is_button_down(KEY_W) {
				delta += forward * dt * speed;
			}
			if input.is_button_down(KEY_S) {
				delta -= forward * dt * speed;
			}
			if input.is_button_down(KEY_D) {
				delta += right * dt * speed;
			}
			if input.is_button_down(KEY_A) {
				delta -= right * dt * speed;
			}
			let new_location = transform.local_location() + delta;
			transform.set_local_location(new_location);
		}
	}
}

define_run_module!(Orchard, "Orchard");