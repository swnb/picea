//! Explicit single-step simulation pipeline for the v1 world API.
//!
//! The stable contract here is intentionally narrow: the pipeline owns
//! cadence/configuration, while the concrete world owns state mutation,
//! contact generation, and event production.

pub(crate) mod broadphase;
pub(crate) mod contacts;
pub(crate) mod integrate;
pub(crate) mod joints;
pub(crate) mod narrowphase;
pub(crate) mod sleep;
pub(crate) mod step;

use serde::{Deserialize, Serialize};

use crate::{events::WorldEvent, handles::WorldRevision, math::FloatNum};

const DEFAULT_STEP_DT: FloatNum = 1.0 / 60.0;
const DEFAULT_VELOCITY_ITERATIONS: u16 = 10;
const DEFAULT_POSITION_ITERATIONS: u16 = 20;

/// Stable step configuration owned by the simulation pipeline.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct StepConfig {
    /// Fixed delta time for one simulation step.
    pub dt: FloatNum,
    /// Number of velocity solver iterations requested from the world.
    pub velocity_iterations: u16,
    /// Number of position solver iterations requested from the world.
    pub position_iterations: u16,
    /// Enables or disables world sleep evaluation for this step.
    pub enable_sleep: bool,
}

impl Default for StepConfig {
    fn default() -> Self {
        Self {
            dt: DEFAULT_STEP_DT,
            velocity_iterations: DEFAULT_VELOCITY_ITERATIONS,
            position_iterations: DEFAULT_POSITION_ITERATIONS,
            enable_sleep: true,
        }
    }
}

/// Stable high-level counters returned after each simulation step.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct StepStats {
    /// Monotonic step number assigned by the pipeline.
    pub step_index: u64,
    /// Total number of bodies visible to the world.
    pub body_count: usize,
    /// Total number of colliders visible to the world.
    pub collider_count: usize,
    /// Total number of joints visible to the world.
    pub joint_count: usize,
    /// Number of active bodies processed during the step.
    pub active_body_count: usize,
    /// Number of broadphase candidate pairs considered during the step.
    pub broadphase_candidate_count: usize,
    /// Number of broadphase proxy insert/remove/reinsert updates during the step.
    pub broadphase_update_count: usize,
    /// Number of stale broadphase proxies dropped because their collider handle disappeared.
    pub broadphase_stale_proxy_drop_count: usize,
    /// Number of broadphase candidates dropped because both colliders belong to one body.
    pub broadphase_same_body_drop_count: usize,
    /// Number of broadphase candidates dropped by collision filters.
    pub broadphase_filter_drop_count: usize,
    /// Number of broadphase candidates rejected by narrowphase geometry.
    pub broadphase_narrowphase_drop_count: usize,
    /// Number of broadphase tree rebuilds used to restore a bounded tree depth.
    pub broadphase_rebuild_count: usize,
    /// Current broadphase tree depth after proxy synchronization.
    pub broadphase_tree_depth: usize,
    /// Number of active contacts emitted during the step.
    pub contact_count: usize,
    /// Number of active manifolds after refresh.
    pub manifold_count: usize,
    /// Number of velocity iterations used for the step.
    pub velocity_iterations: u16,
    /// Number of position iterations used for the step.
    pub position_iterations: u16,
    /// Number of sleep-state transitions observed during the step.
    pub sleep_transition_count: usize,
    /// Number of internal non-finite situations detected and contained.
    pub numeric_warnings: usize,
}

/// Result of exactly one `SimulationPipeline::step` call.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct StepReport {
    /// Monotonic step number assigned by the pipeline.
    pub step_index: u64,
    /// Total simulated time accumulated by the pipeline.
    pub simulated_time: f64,
    /// Fixed step delta used by this report.
    pub dt: FloatNum,
    /// Monotonic world revision after the step completed.
    pub revision: WorldRevision,
    /// Stable counters captured for this step.
    pub stats: StepStats,
    /// Ordered event stream emitted by the world for this step.
    pub events: Vec<WorldEvent>,
}

/// Adapter output produced by a concrete world implementation.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct StepOutcome {
    /// Monotonic world revision after the world mutated.
    pub revision: WorldRevision,
    /// Stable counters gathered while mutating the world.
    pub stats: StepStats,
    /// Ordered event stream for the completed step.
    pub events: Vec<WorldEvent>,
}

/// Minimal world-facing contract needed by the stable simulation pipeline.
pub trait SimulationWorld {
    /// Advances the authoritative world state by one explicit simulation step.
    fn simulate_step(&mut self, config: &StepConfig) -> StepOutcome;
}

/// The stable v1 simulation entrypoint.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct SimulationPipeline {
    config: StepConfig,
    next_step_index: u64,
    simulated_time: f64,
}

impl SimulationPipeline {
    /// Creates a pipeline with explicit step semantics.
    pub fn new(config: StepConfig) -> Self {
        validate_step_config(&config);
        Self {
            config,
            next_step_index: 0,
            simulated_time: 0.0,
        }
    }

    /// Returns the fixed-step configuration used by this pipeline.
    pub fn config(&self) -> &StepConfig {
        &self.config
    }

    /// Advances the world by exactly one configured simulation step.
    pub fn step<W>(&mut self, world: &mut W) -> StepReport
    where
        W: SimulationWorld,
    {
        let step_index = self.next_step_index + 1;
        self.simulated_time += f64::from(self.config.dt);

        let StepOutcome {
            revision,
            mut stats,
            events,
        } = world.simulate_step(&self.config);

        self.next_step_index = step_index;
        stats.step_index = step_index;
        stats.velocity_iterations = self.config.velocity_iterations;
        stats.position_iterations = self.config.position_iterations;

        StepReport {
            step_index,
            simulated_time: self.simulated_time,
            dt: self.config.dt,
            revision,
            stats,
            // Downstream consumers rely on the world's causal ordering here.
            events,
        }
    }
}

fn validate_step_config(config: &StepConfig) {
    assert!(
        config.dt.is_finite() && config.dt > 0.0,
        "step dt must be finite and positive"
    );
}

#[cfg(test)]
mod tests {
    use std::{collections::VecDeque, panic::catch_unwind};

    use crate::{
        events::{ContactEvent, SleepEvent, WorldEvent},
        handles::{BodyHandle, ColliderHandle, ContactId, ManifoldId, WorldRevision},
        math::{point::Point, vector::Vector},
        pipeline::{SimulationPipeline, SimulationWorld, StepConfig, StepOutcome, StepStats},
    };

    #[derive(Default)]
    struct FakeWorld {
        seen_configs: Vec<StepConfig>,
        outcomes: VecDeque<StepOutcome>,
    }

    impl FakeWorld {
        fn with_outcomes(outcomes: impl IntoIterator<Item = StepOutcome>) -> Self {
            Self {
                seen_configs: Vec::new(),
                outcomes: outcomes.into_iter().collect(),
            }
        }
    }

    impl SimulationWorld for FakeWorld {
        fn simulate_step(&mut self, config: &StepConfig) -> StepOutcome {
            self.seen_configs.push(*config);
            self.outcomes.pop_front().unwrap_or_default()
        }
    }

    #[test]
    fn default_step_config_matches_single_step_contract() {
        let config = StepConfig::default();

        assert_eq!(config.dt, 1.0 / 60.0);
        assert_eq!(config.velocity_iterations, 10);
        assert_eq!(config.position_iterations, 20);
        assert!(config.enable_sleep);
    }

    #[test]
    fn new_rejects_non_positive_or_non_finite_dt() {
        for dt in [0.0, -0.1, f32::NAN, f32::INFINITY] {
            let config = StepConfig {
                dt,
                ..StepConfig::default()
            };

            let result = catch_unwind(|| SimulationPipeline::new(config));
            assert!(result.is_err(), "expected dt {dt:?} to be rejected");
        }
    }

    #[test]
    fn step_advances_pipeline_and_preserves_world_event_order() {
        let contact_started = WorldEvent::ContactStarted(ContactEvent {
            contact_id: ContactId::from_raw_parts(11, 0),
            manifold_id: ManifoldId::from_raw_parts(7, 0),
            body_a: BodyHandle::from_raw_parts(1, 0),
            body_b: BodyHandle::from_raw_parts(2, 0),
            collider_a: ColliderHandle::from_raw_parts(3, 0),
            collider_b: ColliderHandle::from_raw_parts(4, 0),
            point: Point::new(2.0, -1.0),
            normal: Vector::new(0.0, 1.0),
            depth: 0.25,
        });
        let sleep_changed = WorldEvent::SleepChanged(SleepEvent {
            body: BodyHandle::from_raw_parts(2, 0),
            is_sleeping: true,
        });

        let mut pipeline = SimulationPipeline::new(StepConfig {
            dt: 1.0 / 120.0,
            velocity_iterations: 6,
            position_iterations: 4,
            enable_sleep: false,
        });
        let mut world = FakeWorld::with_outcomes([StepOutcome {
            revision: WorldRevision::from_raw(9),
            stats: StepStats {
                body_count: 2,
                collider_count: 3,
                joint_count: 1,
                active_body_count: 1,
                contact_count: 1,
                manifold_count: 1,
                sleep_transition_count: 1,
                ..StepStats::default()
            },
            events: vec![contact_started.clone(), sleep_changed.clone()],
        }]);

        let report = pipeline.step(&mut world);

        assert_eq!(world.seen_configs.len(), 1);
        assert_eq!(world.seen_configs[0], *pipeline.config());
        assert_eq!(report.step_index, 1);
        assert_eq!(report.stats.step_index, 1);
        assert_eq!(report.dt, 1.0 / 120.0);
        assert_eq!(report.simulated_time, f64::from(1.0_f32 / 120.0_f32));
        assert_eq!(report.revision, WorldRevision::from_raw(9));
        assert_eq!(report.stats.velocity_iterations, 6);
        assert_eq!(report.stats.position_iterations, 4);
        assert_eq!(report.events, vec![contact_started, sleep_changed]);
    }

    #[test]
    fn step_accumulates_index_and_time_across_multiple_steps() {
        let mut pipeline = SimulationPipeline::new(StepConfig::default());
        let mut world = FakeWorld::with_outcomes([
            StepOutcome {
                revision: WorldRevision::from_raw(1),
                stats: StepStats::default(),
                events: Vec::new(),
            },
            StepOutcome {
                revision: WorldRevision::from_raw(2),
                stats: StepStats::default(),
                events: Vec::new(),
            },
        ]);

        let first = pipeline.step(&mut world);
        let second = pipeline.step(&mut world);

        assert_eq!(first.step_index, 1);
        assert_eq!(second.step_index, 2);
        assert_eq!(first.simulated_time, f64::from(StepConfig::default().dt));
        assert_eq!(
            second.simulated_time,
            f64::from(StepConfig::default().dt) * 2.0
        );
    }
}
