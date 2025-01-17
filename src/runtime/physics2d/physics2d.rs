use {
	ecs::{
		Component,
		System,
		World,
	},
	engine::{
		Builder,
		Module,
	},
	math::Vec2,
	rapier2d::prelude::*,
	serde::{
		Deserialize,
		Serialize,
	},
};

pub struct Physics;
impl Module for Physics {
	fn new() -> Self {
		Physics
	}

	fn depends_on(builder: &mut Builder) -> &mut Builder {
		builder
			.register(PhysicsState::variant())
			.register(Collider::variant())
			.register(RigidBody::variant())
	}
}

#[derive(Serialize, Deserialize)]
pub struct PhysicsState {
	integration_parameters: IntegrationParameters,
	#[serde(skip, default = "PhysicsPipeline::new")]
	physics_pipeline: PhysicsPipeline,
	island_manager: IslandManager,
	broad_phase: BroadPhase,
	narrow_phase: NarrowPhase,
	joint_set: JointSet,
	ccd_solver: CCDSolver,
	rigid_body_set: RigidBodySet,
	collider_set: ColliderSet,
}

impl PhysicsState {
	pub fn new() -> Self {
		Self {
			integration_parameters: IntegrationParameters::default(),
			physics_pipeline: PhysicsPipeline::new(),
			island_manager: IslandManager::new(),
			broad_phase: BroadPhase::new(),
			narrow_phase: NarrowPhase::new(),
			joint_set: JointSet::new(),
			ccd_solver: CCDSolver::new(),
			rigid_body_set: RigidBodySet::new(),
			collider_set: ColliderSet::new(),
		}
	}
}

impl Component for PhysicsState {}

impl Default for PhysicsState {
	fn default() -> Self {
		Self::new()
	}
}

impl Clone for PhysicsState {
	fn clone(&self) -> Self {
		Self {
			integration_parameters: self.integration_parameters,
			physics_pipeline: PhysicsPipeline::new(),
			island_manager: self.island_manager.clone(),
			broad_phase: self.broad_phase.clone(),
			narrow_phase: self.narrow_phase.clone(),
			joint_set: self.joint_set.clone(),
			ccd_solver: self.ccd_solver.clone(),
			rigid_body_set: self.rigid_body_set.clone(),
			collider_set: self.collider_set.clone(),
		}
	}
}

#[derive(Serialize, Deserialize, Clone)]
pub enum Shape {
	Circle { radius: f32 },
	Square { half_extents: Vec2 },
	Capsule { half_height: f32, radius: f32 },
}

impl Shape {
	pub fn circle(radius: f32) -> Self {
		Shape::Circle { radius }
	}

	pub fn square(half_extents: impl Into<Vec2>) -> Self {
		Shape::Square {
			half_extents: half_extents.into(),
		}
	}

	pub fn capsule(half_height: f32, radius: f32) -> Self {
		Shape::Capsule {
			half_height,
			radius,
		}
	}
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Collider {
	handle: Option<ColliderHandle>,

	// TODO: Should all this copied builder data just be the builder composed into the struct
	shape: Shape,
	enabled: bool,
	sensor: bool,
	offset: Vec2,
}

impl Collider {
	pub fn builder(shape: Shape) -> ColliderBuilder {
		ColliderBuilder {
			shape,
			enabled: true,
			sensor: false,
			offset: Vec2::ZERO,
		}
	}
}

impl Component for Collider {}

impl Default for Collider {
	fn default() -> Self {
		Self::builder(Shape::square(Vec2::splat(0.5))).build()
	}
}

pub struct ColliderBuilder {
	shape: Shape,
	enabled: bool,
	sensor: bool,
	offset: Vec2,
}

impl ColliderBuilder {
	pub fn enabled(mut self, enabled: bool) -> Self {
		self.enabled = enabled;
		self
	}

	pub fn sensor(mut self, sensor: bool) -> Self {
		self.sensor = sensor;
		self
	}

	pub fn offset(mut self, offset: impl Into<Vec2>) -> Self {
		self.offset = offset.into();
		self
	}

	pub fn build(self) -> Collider {
		Collider {
			handle: None,
			shape: self.shape,
			enabled: self.enabled,
			sensor: self.sensor,
			offset: self.offset,
		}
	}
}

#[derive(Serialize, Deserialize, Clone)]
pub struct RigidBody {
	handle: Option<RigidBodyHandle>,

	// TODO: Should all this copied builder data just be the builder composed into the struct
	linear_velocity: Vec2,
	angular_velocity: f32,
	gravity_scale: f32,
	linear_damping: f32,
	angular_damping: f32,
	can_sleep: bool,
	sleeping: bool,
	ccd_enabled: bool,
	is_kinematic: bool,
}

impl RigidBody {
	pub fn builder() -> RigidBodyBuilder {
		RigidBodyBuilder {
			linear_velocity: Vec2::ZERO,
			angular_velocity: 0.0,
			gravity_scale: 1.0,
			linear_damping: 0.0,
			angular_damping: 0.0,
			can_sleep: true,
			sleeping: false,
			ccd_enabled: false,
			is_kinematic: true,
		}
	}
}

impl Component for RigidBody {}

impl Default for RigidBody {
	fn default() -> Self {
		Self::builder().build()
	}
}

pub struct RigidBodyBuilder {
	linear_velocity: Vec2,
	angular_velocity: f32,
	gravity_scale: f32,
	linear_damping: f32,
	angular_damping: f32,
	can_sleep: bool,
	sleeping: bool,
	ccd_enabled: bool,
	is_kinematic: bool,
}

impl RigidBodyBuilder {
	pub fn build(self) -> RigidBody {
		RigidBody {
			handle: None,

			linear_velocity: self.linear_velocity,
			angular_velocity: self.angular_velocity,
			gravity_scale: self.gravity_scale,
			linear_damping: self.linear_damping,
			angular_damping: self.angular_damping,
			can_sleep: self.can_sleep,
			sleeping: self.sleeping,
			ccd_enabled: self.ccd_enabled,
			is_kinematic: self.is_kinematic,
		}
	}
}

#[derive(Clone)]
pub struct PhysicsStep;
impl System for PhysicsStep {
	fn run(&self, world: &World, dt: f32) {
		let mut physics_states = world.write::<PhysicsState>();
		let PhysicsState {
			integration_parameters,
			physics_pipeline,
			island_manager,
			broad_phase,
			narrow_phase,
			joint_set,
			ccd_solver,
			rigid_body_set,
			collider_set,
		} = &mut *physics_states.get_mut_or_default(world.singleton);

		let physics_hooks = ();
		let event_handler = ();

		let gravity = vector![0.0, -9.81];

		// TODO: Should this just be 60 fps for stability?
		integration_parameters.dt = dt;

		physics_pipeline.step(
			&gravity,
			integration_parameters,
			island_manager,
			broad_phase,
			narrow_phase,
			rigid_body_set,
			collider_set,
			joint_set,
			ccd_solver,
			&physics_hooks,
			&event_handler,
		);
	}
}
