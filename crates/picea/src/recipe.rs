//! Declarative world recipes and transactional command wrappers.
//!
//! M9 keeps this layer above the low-level `World::create_*` methods. The
//! tradeoff is a cloned scratch world for batch commands: it costs memory, but
//! it preserves the existing lifecycle contracts and gives callers an atomic
//! "all commands applied or no visible mutation" boundary.

use serde::{Deserialize, Serialize};

use crate::{
    body::{BodyDesc, BodyPatch, BodyType, Pose},
    collider::{ColliderDesc, ColliderPatch, CollisionFilter, Material, SharedShape},
    handles::{BodyHandle, ColliderHandle, JointHandle},
    joint::{DistanceJointDesc, JointDesc, JointPatch, WorldAnchorJointDesc},
    math::{point::Point, FloatNum},
    world::{HandleError, ValidationError, World, WorldDesc, WorldError},
};

/// Material presets for scenario recipes and examples.
///
/// These are named defaults, not physical constants. They keep common scenes
/// readable while still allowing direct `Material` values when a test needs
/// precise friction or restitution.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MaterialPreset {
    /// Matches `Material::default()`.
    #[default]
    Default,
    /// Low-friction surface useful for slide-heavy broadphase/solver scenes.
    Ice,
    /// Higher-friction surface for stack and resting-contact scenes.
    Rough,
    /// Restitution-heavy material for bounce examples.
    Bouncy,
    /// Very high friction and no bounce.
    Sticky,
}

impl Material {
    /// Converts a named material preset into concrete material coefficients.
    pub const fn preset(preset: MaterialPreset) -> Self {
        match preset {
            MaterialPreset::Default => Self {
                friction: 0.2,
                restitution: 0.0,
            },
            MaterialPreset::Ice => Self {
                friction: 0.02,
                restitution: 0.0,
            },
            MaterialPreset::Rough => Self {
                friction: 0.8,
                restitution: 0.0,
            },
            MaterialPreset::Bouncy => Self {
                friction: 0.15,
                restitution: 0.8,
            },
            MaterialPreset::Sticky => Self {
                friction: 1.2,
                restitution: 0.0,
            },
        }
    }
}

impl From<MaterialPreset> for Material {
    fn from(value: MaterialPreset) -> Self {
        Self::preset(value)
    }
}

/// Common collision layer bits used by recipe presets.
pub struct CollisionLayers;

impl CollisionLayers {
    /// Static level geometry such as floors and walls.
    pub const STATIC_GEOMETRY: u64 = 1 << 0;
    /// Dynamic simulated bodies.
    pub const DYNAMIC_BODY: u64 = 1 << 1;
    /// Sensor-only trigger colliders.
    pub const SENSOR: u64 = 1 << 2;
    /// Query-only helper colliders.
    pub const QUERY_ONLY: u64 = 1 << 3;
    /// All current and future layer bits.
    pub const ALL: u64 = u64::MAX;
}

/// Named collision-layer presets for recipes and example scenes.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CollisionLayerPreset {
    /// Fully permissive filter, matching `CollisionFilter::default()`.
    #[default]
    Default,
    /// Static geometry collides with dynamic bodies and sensors.
    StaticGeometry,
    /// Dynamic bodies collide with static and dynamic geometry.
    DynamicBody,
    /// Filter bits commonly used by trigger colliders; pair with `with_sensor(true)`
    /// when the collider should skip solver response.
    Sensor,
    /// Query helpers do not participate in default simulation pairs.
    QueryOnly,
}

impl CollisionFilter {
    /// Builds a filter from explicit bitmasks.
    pub const fn from_bits(memberships: u64, collides_with: u64) -> Self {
        Self {
            memberships,
            collides_with,
        }
    }

    /// Converts a named layer preset into concrete collision bitmasks.
    pub const fn preset(preset: CollisionLayerPreset) -> Self {
        match preset {
            CollisionLayerPreset::Default => Self {
                memberships: u64::MAX,
                collides_with: u64::MAX,
            },
            CollisionLayerPreset::StaticGeometry => Self {
                memberships: CollisionLayers::STATIC_GEOMETRY,
                collides_with: CollisionLayers::DYNAMIC_BODY | CollisionLayers::SENSOR,
            },
            CollisionLayerPreset::DynamicBody => Self {
                memberships: CollisionLayers::DYNAMIC_BODY,
                collides_with: CollisionLayers::STATIC_GEOMETRY
                    | CollisionLayers::DYNAMIC_BODY
                    | CollisionLayers::SENSOR,
            },
            CollisionLayerPreset::Sensor => Self {
                memberships: CollisionLayers::SENSOR,
                collides_with: CollisionLayers::STATIC_GEOMETRY
                    | CollisionLayers::DYNAMIC_BODY
                    | CollisionLayers::SENSOR,
            },
            CollisionLayerPreset::QueryOnly => Self {
                memberships: CollisionLayers::QUERY_ONLY,
                collides_with: 0,
            },
        }
    }
}

impl From<CollisionLayerPreset> for CollisionFilter {
    fn from(value: CollisionLayerPreset) -> Self {
        Self::preset(value)
    }
}

/// Collider creation bundle used by recipes and batch commands.
#[derive(Clone, Debug, PartialEq)]
pub struct ColliderBundle {
    /// Low-level collider descriptor passed through unchanged on success.
    pub desc: ColliderDesc,
}

impl ColliderBundle {
    /// Creates a collider bundle from a shape and default descriptor values.
    pub fn new(shape: SharedShape) -> Self {
        Self {
            desc: ColliderDesc {
                shape,
                ..ColliderDesc::default()
            },
        }
    }

    /// Creates a circle collider bundle with default descriptor values.
    pub fn circle(radius: FloatNum) -> Self {
        Self::new(SharedShape::circle(radius))
    }

    /// Creates an axis-aligned rectangle collider bundle with default descriptor values.
    pub fn rect(width: FloatNum, height: FloatNum) -> Self {
        Self::new(SharedShape::rect(width, height))
    }

    /// Wraps a fully specified collider descriptor.
    pub fn from_desc(desc: ColliderDesc) -> Self {
        Self { desc }
    }

    /// Sets the local pose relative to the parent body.
    pub fn with_local_pose(mut self, local_pose: Pose) -> Self {
        self.desc.local_pose = local_pose;
        self
    }

    /// Sets the collider density used for mass-property derivation.
    pub fn with_density(mut self, density: crate::math::FloatNum) -> Self {
        self.desc.density = density;
        self
    }

    /// Sets material coefficients or a named material preset.
    pub fn with_material(mut self, material: impl Into<Material>) -> Self {
        self.desc.material = material.into();
        self
    }

    /// Sets collision filtering bits or a named layer preset.
    pub fn with_filter(mut self, filter: impl Into<CollisionFilter>) -> Self {
        self.desc.filter = filter.into();
        self
    }

    /// Sets whether this collider skips impulse generation.
    pub fn with_sensor(mut self, is_sensor: bool) -> Self {
        self.desc.is_sensor = is_sensor;
        self
    }
}

impl Default for ColliderBundle {
    fn default() -> Self {
        Self {
            desc: ColliderDesc::default(),
        }
    }
}

impl From<ColliderDesc> for ColliderBundle {
    fn from(value: ColliderDesc) -> Self {
        Self::from_desc(value)
    }
}

/// Body creation bundle with any colliders that should be attached to it.
#[derive(Clone, Debug, PartialEq)]
pub struct BodyBundle {
    /// Low-level body descriptor passed through unchanged on success.
    pub desc: BodyDesc,
    /// Colliders attached after the body handle is allocated.
    pub colliders: Vec<ColliderBundle>,
}

impl BodyBundle {
    /// Creates a body bundle from an explicit body descriptor.
    pub fn new(desc: BodyDesc) -> Self {
        Self {
            desc,
            colliders: Vec::new(),
        }
    }

    /// Creates a dynamic body bundle.
    pub fn dynamic() -> Self {
        Self::new(BodyDesc {
            body_type: BodyType::Dynamic,
            ..BodyDesc::default()
        })
    }

    /// Creates a static body bundle.
    pub fn static_body() -> Self {
        Self::new(BodyDesc {
            body_type: BodyType::Static,
            ..BodyDesc::default()
        })
    }

    /// Creates a kinematic body bundle.
    pub fn kinematic() -> Self {
        Self::new(BodyDesc {
            body_type: BodyType::Kinematic,
            ..BodyDesc::default()
        })
    }

    /// Sets the initial world-space pose.
    pub fn with_pose(mut self, pose: Pose) -> Self {
        self.desc.pose = pose;
        self
    }

    /// Adds one collider to this body bundle.
    pub fn with_collider(mut self, collider: impl Into<ColliderBundle>) -> Self {
        self.colliders.push(collider.into());
        self
    }

    /// Adds several colliders to this body bundle.
    pub fn with_colliders<I>(mut self, colliders: I) -> Self
    where
        I: IntoIterator<Item = ColliderBundle>,
    {
        self.colliders.extend(colliders);
        self
    }
}

impl Default for BodyBundle {
    fn default() -> Self {
        Self::new(BodyDesc::default())
    }
}

impl From<BodyDesc> for BodyBundle {
    fn from(value: BodyDesc) -> Self {
        Self::new(value)
    }
}

/// Reusable body-and-collider asset for scene recipes.
///
/// A body asset is still just recipe data: placing it in a scene clones the
/// stored [`BodyBundle`] and writes a pose before `WorldCommands` validates the
/// low-level descriptors on a scratch world. That keeps this layer convenient
/// for examples and fixtures without turning it into a separate runtime object
/// model.
#[derive(Clone, Debug, PartialEq)]
pub struct BodyAsset {
    /// Template bundle cloned whenever the asset is placed into a scene.
    pub bundle: BodyBundle,
}

impl BodyAsset {
    /// Wraps any body bundle as a reusable scene asset.
    pub fn from_bundle(bundle: impl Into<BodyBundle>) -> Self {
        Self {
            bundle: bundle.into(),
        }
    }

    /// Creates a static rectangle body asset with one rectangle collider.
    pub fn static_rect(width: FloatNum, height: FloatNum) -> Self {
        Self::from_bundle(
            BodyBundle::static_body().with_collider(ColliderBundle::rect(width, height)),
        )
    }

    /// Creates a dynamic rectangle body asset with one rectangle collider.
    pub fn dynamic_rect(width: FloatNum, height: FloatNum) -> Self {
        Self::from_bundle(BodyBundle::dynamic().with_collider(ColliderBundle::rect(width, height)))
    }

    /// Creates a static circle body asset with one circle collider.
    pub fn static_circle(radius: FloatNum) -> Self {
        Self::from_bundle(BodyBundle::static_body().with_collider(ColliderBundle::circle(radius)))
    }

    /// Creates a dynamic circle body asset with one circle collider.
    pub fn dynamic_circle(radius: FloatNum) -> Self {
        Self::from_bundle(BodyBundle::dynamic().with_collider(ColliderBundle::circle(radius)))
    }

    /// Applies material coefficients or a named material preset to every collider in the asset.
    pub fn with_material(mut self, material: impl Into<Material>) -> Self {
        let material = material.into();
        for collider in &mut self.bundle.colliders {
            collider.desc.material = material;
        }
        self
    }

    /// Applies collision filtering bits or a named layer preset to every collider in the asset.
    pub fn with_filter(mut self, filter: impl Into<CollisionFilter>) -> Self {
        let filter = filter.into();
        for collider in &mut self.bundle.colliders {
            collider.desc.filter = filter;
        }
        self
    }

    /// Applies the collider density to every collider in the asset.
    pub fn with_density(mut self, density: FloatNum) -> Self {
        for collider in &mut self.bundle.colliders {
            collider.desc.density = density;
        }
        self
    }

    /// Applies the sensor flag to every collider in the asset.
    pub fn with_sensor(mut self, is_sensor: bool) -> Self {
        for collider in &mut self.bundle.colliders {
            collider.desc.is_sensor = is_sensor;
        }
        self
    }

    /// Places this asset at a world-space pose and returns a concrete body bundle.
    pub fn at(&self, pose: Pose) -> BodyBundle {
        self.bundle.clone().with_pose(pose)
    }

    /// Consumes the asset and returns its underlying body bundle template.
    pub fn into_bundle(self) -> BodyBundle {
        self.bundle
    }
}

impl From<BodyBundle> for BodyAsset {
    fn from(value: BodyBundle) -> Self {
        Self::from_bundle(value)
    }
}

/// Joint creation bundle whose body endpoints refer to recipe body indices.
///
/// Recipes allocate real `BodyHandle`s only during instantiation. Keeping joints
/// in index space makes scene setup declarative while the command layer still
/// resolves and validates concrete handles on a scratch world before commit.
#[derive(Clone, Debug, PartialEq)]
pub enum JointBundle {
    /// Distance joint between two recipe bodies.
    Distance {
        /// Index of the first body bundle in `WorldRecipe::bodies`.
        body_a: usize,
        /// Index of the second body bundle in `WorldRecipe::bodies`.
        body_b: usize,
        /// Low-level descriptor fields not including resolved body handles.
        desc: DistanceJointDesc,
    },
    /// World-anchor joint attached to one recipe body.
    WorldAnchor {
        /// Index of the body bundle in `WorldRecipe::bodies`.
        body: usize,
        /// Low-level descriptor fields not including the resolved body handle.
        desc: WorldAnchorJointDesc,
    },
}

impl JointBundle {
    /// Creates a distance joint between two recipe body indices.
    pub fn distance(body_a: usize, body_b: usize) -> Self {
        Self::Distance {
            body_a,
            body_b,
            desc: DistanceJointDesc::default(),
        }
    }

    /// Creates a world-anchor joint attached to one recipe body index.
    pub fn world_anchor(body: usize) -> Self {
        Self::WorldAnchor {
            body,
            desc: WorldAnchorJointDesc::default(),
        }
    }

    /// Sets the distance joint's rest length when this bundle is a distance joint.
    pub fn with_rest_length(mut self, rest_length: crate::math::FloatNum) -> Self {
        if let Self::Distance { desc, .. } = &mut self {
            desc.rest_length = rest_length;
        }
        self
    }

    /// Sets the world-space anchor when this bundle is a world-anchor joint.
    pub fn with_world_anchor(mut self, world_anchor: Point) -> Self {
        if let Self::WorldAnchor { desc, .. } = &mut self {
            desc.world_anchor = world_anchor;
        }
        self
    }

    fn resolve(&self, body_handles: &[BodyHandle]) -> Result<JointDesc, RecipeBodyResolveError> {
        match self {
            Self::Distance {
                body_a,
                body_b,
                desc,
            } => {
                let mut desc = desc.clone();
                desc.body_a = resolve_recipe_body(*body_a, body_handles, "body_a")?;
                desc.body_b = resolve_recipe_body(*body_b, body_handles, "body_b")?;
                Ok(JointDesc::Distance(desc))
            }
            Self::WorldAnchor { body, desc } => {
                let mut desc = desc.clone();
                desc.body = resolve_recipe_body(*body, body_handles, "body")?;
                Ok(JointDesc::WorldAnchor(desc))
            }
        }
    }
}

/// Declarative world recipe for tests, examples, and benchmarks.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct WorldRecipe {
    /// Immutable world configuration.
    pub desc: WorldDesc,
    /// Body bundles created in deterministic order.
    pub bodies: Vec<BodyBundle>,
    /// Joint bundles resolved after recipe bodies are allocated.
    pub joints: Vec<JointBundle>,
}

impl WorldRecipe {
    /// Starts a recipe with the given world description.
    pub fn new(desc: WorldDesc) -> Self {
        Self {
            desc,
            bodies: Vec::new(),
            joints: Vec::new(),
        }
    }

    /// Adds one body bundle to the recipe.
    pub fn with_body(mut self, body: impl Into<BodyBundle>) -> Self {
        self.bodies.push(body.into());
        self
    }

    /// Adds one placed scene body to the recipe.
    ///
    /// This is an ergonomic alias for `with_body`: scene bodies still become
    /// normal body bundles before instantiation, so low-level world creation
    /// remains the single validation and allocation path.
    pub fn with_scene_body(self, body: impl Into<BodyBundle>) -> Self {
        self.with_body(body)
    }

    /// Adds one joint bundle to the recipe.
    pub fn with_joint(mut self, joint: JointBundle) -> Self {
        self.joints.push(joint);
        self
    }

    /// Instantiates the recipe into a concrete world.
    pub fn instantiate(self) -> Result<WorldRecipeResult, WorldCommandError> {
        self.instantiate_with_context().map_err(|error| error.error)
    }

    /// Instantiates the recipe and returns a nested recipe path when setup fails.
    pub fn instantiate_with_context(self) -> Result<WorldRecipeResult, WorldCommandContextError> {
        let mut world = World::new(self.desc);
        let created = world
            .commands()
            .create_recipe_with_context(self.bodies, self.joints)?;
        Ok(WorldRecipeResult { world, created })
    }
}

/// Successful recipe instantiation result.
#[derive(Clone, Debug)]
pub struct WorldRecipeResult {
    /// Created world.
    pub world: World,
    /// Structured handles and command events emitted while creating the recipe.
    pub created: WorldCommandReport,
}

/// Batch command facade for `World`.
pub struct WorldCommands<'a> {
    world: &'a mut World,
}

impl<'a> WorldCommands<'a> {
    /// Creates a command facade over a mutable world reference.
    pub fn new(world: &'a mut World) -> Self {
        Self { world }
    }

    /// Creates several body bundles atomically.
    pub fn create_bodies<I>(&mut self, bodies: I) -> Result<WorldCommandReport, WorldCommandError>
    where
        I: IntoIterator<Item = BodyBundle>,
    {
        self.create_bodies_with_context(bodies)
            .map_err(|error| error.error)
    }

    /// Creates several body bundles atomically and returns a nested path on failure.
    pub fn create_bodies_with_context<I>(
        &mut self,
        bodies: I,
    ) -> Result<WorldCommandReport, WorldCommandContextError>
    where
        I: IntoIterator<Item = BodyBundle>,
    {
        self.create_body_bundles(bodies, "bodies")
    }

    /// Creates placed scene bodies atomically.
    ///
    /// This is a readability wrapper for setup code. It uses the same
    /// clone-and-commit transaction boundary as `create_bodies`, so callers do
    /// not accidentally get a hot-path mutation API with different semantics.
    pub fn create_scene_bodies<I, B>(
        &mut self,
        bodies: I,
    ) -> Result<WorldCommandReport, WorldCommandError>
    where
        I: IntoIterator<Item = B>,
        B: Into<BodyBundle>,
    {
        self.create_scene_bodies_with_context(bodies)
            .map_err(|error| error.error)
    }

    /// Creates placed scene bodies atomically and returns a nested path on failure.
    pub fn create_scene_bodies_with_context<I, B>(
        &mut self,
        bodies: I,
    ) -> Result<WorldCommandReport, WorldCommandContextError>
    where
        I: IntoIterator<Item = B>,
        B: Into<BodyBundle>,
    {
        self.create_body_bundles(bodies.into_iter().map(Into::into), "scene.bodies")
    }

    /// Creates recipe bodies and recipe-indexed joints atomically.
    pub fn create_recipe<I, J>(
        &mut self,
        bodies: I,
        joints: J,
    ) -> Result<WorldCommandReport, WorldCommandError>
    where
        I: IntoIterator<Item = BodyBundle>,
        J: IntoIterator<Item = JointBundle>,
    {
        self.create_recipe_with_context(bodies, joints)
            .map_err(|error| error.error)
    }

    /// Creates recipe bodies and recipe-indexed joints atomically with nested error paths.
    pub fn create_recipe_with_context<I, J>(
        &mut self,
        bodies: I,
        joints: J,
    ) -> Result<WorldCommandReport, WorldCommandContextError>
    where
        I: IntoIterator<Item = BodyBundle>,
        J: IntoIterator<Item = JointBundle>,
    {
        let mut scratch = self.world.clone();
        let mut report = WorldCommandReport::default();
        let mut command_index = 0;

        for (body_index, body) in bodies.into_iter().enumerate() {
            let path = format!("recipe.bodies[{body_index}]");
            apply_command(
                &mut scratch,
                &mut report,
                command_index,
                WorldCommand::CreateBody(body),
                &path,
            )?;
            command_index += 1;
        }

        for (joint_index, joint) in joints.into_iter().enumerate() {
            let path = format!("recipe.joints[{joint_index}]");
            let desc = joint.resolve(&report.body_handles).map_err(|error| {
                command_context_error(
                    command_index,
                    None,
                    WorldCommandKind::CreateJoint,
                    error.error,
                    format!("{path}.desc.{}", error.field),
                )
            })?;
            apply_command(
                &mut scratch,
                &mut report,
                command_index,
                WorldCommand::CreateJoint { desc },
                &path,
            )?;
            command_index += 1;
        }

        *self.world = scratch;
        Ok(report)
    }

    /// Applies one command atomically.
    pub fn apply_one(
        &mut self,
        command: WorldCommand,
    ) -> Result<WorldCommandReport, WorldCommandError> {
        self.apply_one_with_context(command)
            .map_err(|error| error.error)
    }

    /// Applies one command atomically and returns a nested path on failure.
    pub fn apply_one_with_context(
        &mut self,
        command: WorldCommand,
    ) -> Result<WorldCommandReport, WorldCommandContextError> {
        self.apply_with_context([command])
    }

    /// Applies a batch atomically.
    ///
    /// Validation and handle resolution are delegated to the existing low-level
    /// world APIs, but they run on a cloned scratch world. The real world is
    /// replaced only after every command succeeds, so a rejected batch cannot
    /// leak earlier creates, patches, or destroys.
    pub fn apply<I>(&mut self, commands: I) -> Result<WorldCommandReport, WorldCommandError>
    where
        I: IntoIterator<Item = WorldCommand>,
    {
        self.apply_with_context(commands)
            .map_err(|error| error.error)
    }

    /// Applies a batch atomically and returns a nested path on failure.
    pub fn apply_with_context<I>(
        &mut self,
        commands: I,
    ) -> Result<WorldCommandReport, WorldCommandContextError>
    where
        I: IntoIterator<Item = WorldCommand>,
    {
        let mut scratch = self.world.clone();
        let mut report = WorldCommandReport::default();
        for (command_index, command) in commands.into_iter().enumerate() {
            let path = format!("commands[{command_index}]");
            apply_command(&mut scratch, &mut report, command_index, command, &path)?;
        }
        *self.world = scratch;
        Ok(report)
    }

    fn create_body_bundles<I>(
        &mut self,
        bodies: I,
        path_root: &str,
    ) -> Result<WorldCommandReport, WorldCommandContextError>
    where
        I: IntoIterator<Item = BodyBundle>,
    {
        let mut scratch = self.world.clone();
        let mut report = WorldCommandReport::default();
        for (body_index, body) in bodies.into_iter().enumerate() {
            let path = format!("{path_root}[{body_index}]");
            apply_command(
                &mut scratch,
                &mut report,
                body_index,
                WorldCommand::CreateBody(body),
                &path,
            )?;
        }
        *self.world = scratch;
        Ok(report)
    }
}

impl World {
    /// Returns the transactional command wrapper for this world.
    pub fn commands(&mut self) -> WorldCommands<'_> {
        WorldCommands::new(self)
    }
}

/// World mutation command accepted by `WorldCommands`.
#[derive(Clone, Debug, PartialEq)]
pub enum WorldCommand {
    /// Create one body and all nested collider bundles.
    CreateBody(BodyBundle),
    /// Create one collider attached to an existing body.
    CreateCollider {
        body: BodyHandle,
        collider: ColliderBundle,
    },
    /// Create one joint after validating referenced body handles.
    CreateJoint { desc: JointDesc },
    /// Patch an existing body.
    PatchBody { body: BodyHandle, patch: BodyPatch },
    /// Patch an existing collider.
    PatchCollider {
        collider: ColliderHandle,
        patch: ColliderPatch,
    },
    /// Patch an existing joint.
    PatchJoint {
        joint: JointHandle,
        patch: JointPatch,
    },
    /// Destroy an existing body and its dependent colliders/joints.
    DestroyBody { body: BodyHandle },
    /// Destroy an existing collider.
    DestroyCollider { collider: ColliderHandle },
    /// Destroy an existing joint.
    DestroyJoint { joint: JointHandle },
}

/// Stable command kind attached to command errors.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WorldCommandKind {
    /// Body creation failed.
    CreateBody,
    /// Nested collider creation failed.
    CreateCollider,
    /// Joint creation failed.
    CreateJoint,
    /// Body patch failed.
    PatchBody,
    /// Collider patch failed.
    PatchCollider,
    /// Joint patch failed.
    PatchJoint,
    /// Body destroy failed.
    DestroyBody,
    /// Collider destroy failed.
    DestroyCollider,
    /// Joint destroy failed.
    DestroyJoint,
}

/// Structured command failure.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorldCommandError {
    /// Index of the top-level command in the rejected batch.
    pub command_index: usize,
    /// Nested collider index for `BodyBundle` collider failures.
    pub collider_index: Option<usize>,
    /// Command kind that failed.
    pub kind: WorldCommandKind,
    /// Underlying world validation, handle, or topology error.
    pub error: WorldError,
}

/// Structured command failure with a nested recipe or command path.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorldCommandContextError {
    /// Human-readable path to the nested recipe, command, or descriptor value that failed.
    pub path: String,
    /// Existing command failure payload kept intact for compatibility.
    pub error: WorldCommandError,
}

/// Structured command event returned by successful batch commands.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WorldCommandEvent {
    /// A body was created.
    BodyCreated { body: BodyHandle },
    /// A collider was created and attached to a body.
    ColliderCreated {
        body: BodyHandle,
        collider: ColliderHandle,
    },
    /// A joint was created.
    JointCreated { joint: JointHandle },
    /// A body was patched.
    BodyPatched { body: BodyHandle },
    /// A collider was patched.
    ColliderPatched { collider: ColliderHandle },
    /// A joint was patched.
    JointPatched { joint: JointHandle },
    /// A body was destroyed.
    BodyDestroyed { body: BodyHandle },
    /// A collider was destroyed.
    ColliderDestroyed { collider: ColliderHandle },
    /// A joint was destroyed.
    JointDestroyed { joint: JointHandle },
}

/// Structured success report returned by `WorldCommands`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct WorldCommandReport {
    /// Body handles created by the batch, in creation order.
    pub body_handles: Vec<BodyHandle>,
    /// Collider handles created by nested bundles, in creation order.
    pub collider_handles: Vec<ColliderHandle>,
    /// Joint handles created by the batch, in creation order.
    pub joint_handles: Vec<JointHandle>,
    /// High-level command events in application order.
    pub events: Vec<WorldCommandEvent>,
}

fn apply_command(
    world: &mut World,
    report: &mut WorldCommandReport,
    command_index: usize,
    command: WorldCommand,
    path: &str,
) -> Result<(), WorldCommandContextError> {
    match command {
        WorldCommand::CreateBody(bundle) => {
            apply_create_body(world, report, command_index, bundle, path)
        }
        WorldCommand::CreateCollider { body, collider } => {
            let collider = world
                .create_collider(body, collider.desc)
                .map_err(|error| {
                    let error_path = if matches!(error, WorldError::Validation(_)) {
                        validation_path(&format!("{path}.collider.desc"), &error)
                    } else {
                        format!("{path}.body")
                    };
                    command_context_error(
                        command_index,
                        None,
                        WorldCommandKind::CreateCollider,
                        error,
                        error_path,
                    )
                })?;
            report.collider_handles.push(collider);
            report
                .events
                .push(WorldCommandEvent::ColliderCreated { body, collider });
            Ok(())
        }
        WorldCommand::CreateJoint { desc } => {
            let joint = world.create_joint(desc).map_err(|error| {
                let error_path = validation_path(&format!("{path}.desc"), &error);
                command_context_error(
                    command_index,
                    None,
                    WorldCommandKind::CreateJoint,
                    error,
                    error_path,
                )
            })?;
            report.joint_handles.push(joint);
            report
                .events
                .push(WorldCommandEvent::JointCreated { joint });
            Ok(())
        }
        WorldCommand::PatchBody { body, patch } => {
            world.apply_body_patch(body, patch).map_err(|error| {
                let error_path = if matches!(error, WorldError::Validation(_)) {
                    validation_path(&format!("{path}.patch"), &error)
                } else {
                    format!("{path}.body")
                };
                command_context_error(
                    command_index,
                    None,
                    WorldCommandKind::PatchBody,
                    error,
                    error_path,
                )
            })?;
            report.events.push(WorldCommandEvent::BodyPatched { body });
            Ok(())
        }
        WorldCommand::PatchCollider { collider, patch } => {
            world
                .apply_collider_patch(collider, patch)
                .map_err(|error| {
                    let error_path = if matches!(error, WorldError::Validation(_)) {
                        validation_path(&format!("{path}.patch"), &error)
                    } else {
                        format!("{path}.collider")
                    };
                    command_context_error(
                        command_index,
                        None,
                        WorldCommandKind::PatchCollider,
                        error,
                        error_path,
                    )
                })?;
            report
                .events
                .push(WorldCommandEvent::ColliderPatched { collider });
            Ok(())
        }
        WorldCommand::PatchJoint { joint, patch } => {
            world.apply_joint_patch(joint, patch).map_err(|error| {
                let error_path = if matches!(error, WorldError::Validation(_)) {
                    validation_path(&format!("{path}.patch"), &error)
                } else {
                    format!("{path}.joint")
                };
                command_context_error(
                    command_index,
                    None,
                    WorldCommandKind::PatchJoint,
                    error,
                    error_path,
                )
            })?;
            report
                .events
                .push(WorldCommandEvent::JointPatched { joint });
            Ok(())
        }
        WorldCommand::DestroyBody { body } => {
            world.destroy_body(body).map_err(|error| {
                command_context_error(
                    command_index,
                    None,
                    WorldCommandKind::DestroyBody,
                    error,
                    format!("{path}.body"),
                )
            })?;
            report
                .events
                .push(WorldCommandEvent::BodyDestroyed { body });
            Ok(())
        }
        WorldCommand::DestroyCollider { collider } => {
            world.destroy_collider(collider).map_err(|error| {
                command_context_error(
                    command_index,
                    None,
                    WorldCommandKind::DestroyCollider,
                    error,
                    format!("{path}.collider"),
                )
            })?;
            report
                .events
                .push(WorldCommandEvent::ColliderDestroyed { collider });
            Ok(())
        }
        WorldCommand::DestroyJoint { joint } => {
            world.destroy_joint(joint).map_err(|error| {
                command_context_error(
                    command_index,
                    None,
                    WorldCommandKind::DestroyJoint,
                    error,
                    format!("{path}.joint"),
                )
            })?;
            report
                .events
                .push(WorldCommandEvent::JointDestroyed { joint });
            Ok(())
        }
    }
}

fn apply_create_body(
    world: &mut World,
    report: &mut WorldCommandReport,
    command_index: usize,
    bundle: BodyBundle,
    path: &str,
) -> Result<(), WorldCommandContextError> {
    let body = world.create_body(bundle.desc).map_err(|error| {
        let error_path = validation_path(&format!("{path}.desc"), &error);
        command_context_error(
            command_index,
            None,
            WorldCommandKind::CreateBody,
            error,
            error_path,
        )
    })?;
    report.body_handles.push(body);
    report.events.push(WorldCommandEvent::BodyCreated { body });

    for (collider_index, collider) in bundle.colliders.into_iter().enumerate() {
        let collider = world
            .create_collider(body, collider.desc)
            .map_err(|error| {
                let error_path =
                    validation_path(&format!("{path}.colliders[{collider_index}].desc"), &error);
                command_context_error(
                    command_index,
                    Some(collider_index),
                    WorldCommandKind::CreateCollider,
                    error,
                    error_path,
                )
            })?;
        report.collider_handles.push(collider);
        report
            .events
            .push(WorldCommandEvent::ColliderCreated { body, collider });
    }

    Ok(())
}

fn command_error(
    command_index: usize,
    collider_index: Option<usize>,
    kind: WorldCommandKind,
    error: WorldError,
) -> WorldCommandError {
    WorldCommandError {
        command_index,
        collider_index,
        kind,
        error,
    }
}

fn command_context_error(
    command_index: usize,
    collider_index: Option<usize>,
    kind: WorldCommandKind,
    error: WorldError,
    path: impl Into<String>,
) -> WorldCommandContextError {
    WorldCommandContextError {
        path: path.into(),
        error: command_error(command_index, collider_index, kind, error),
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct RecipeBodyResolveError {
    field: &'static str,
    error: WorldError,
}

fn resolve_recipe_body(
    index: usize,
    body_handles: &[BodyHandle],
    field: &'static str,
) -> Result<BodyHandle, RecipeBodyResolveError> {
    body_handles
        .get(index)
        .copied()
        .ok_or(RecipeBodyResolveError {
            field,
            error: WorldError::Handle(HandleError::MissingBody {
                handle: BodyHandle::INVALID,
            }),
        })
}

fn validation_path(base: &str, error: &WorldError) -> String {
    validation_field(error)
        .map(|field| format!("{base}.{field}"))
        .unwrap_or_else(|| base.to_owned())
}

fn validation_field(error: &WorldError) -> Option<&'static str> {
    match error {
        WorldError::Validation(ValidationError::BodyDesc { field })
        | WorldError::Validation(ValidationError::BodyPatch { field })
        | WorldError::Validation(ValidationError::ColliderDesc { field })
        | WorldError::Validation(ValidationError::ColliderPatch { field })
        | WorldError::Validation(ValidationError::JointDesc { field })
        | WorldError::Validation(ValidationError::JointPatch { field }) => Some(field),
        WorldError::Handle(_) | WorldError::Topology(_) => None,
    }
}
