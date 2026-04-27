//! Typed world events emitted by the v1 simulation pipeline.

use serde::{Deserialize, Serialize};

use crate::{
    handles::{BodyHandle, ColliderHandle, ContactFeatureId, ContactId, JointHandle, ManifoldId},
    math::{point::Point, vector::Vector, FloatNum},
};

/// Compact trace facts for a continuous collision detection clamp.
///
/// CCD (continuous collision detection) sweeps a fast collider from the
/// previous step pose to the current pre-contact pose. `toi` is the first time
/// of impact as a normalized 0..1 fraction of that sweep; `advancement` is the
/// final clamped fraction after adding a tiny overlap slop so the regular
/// contact manifold can still be generated.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct CcdTrace {
    /// Dynamic body that was swept.
    #[serde(default)]
    pub moving_body: BodyHandle,
    /// Static body hit by the sweep.
    #[serde(default)]
    pub static_body: BodyHandle,
    /// Dynamic circle collider that was swept.
    #[serde(default)]
    pub moving_collider: ColliderHandle,
    /// Static convex collider hit by the sweep.
    #[serde(default)]
    pub static_collider: ColliderHandle,
    /// Circle-center start point at the beginning of the step.
    #[serde(default)]
    pub swept_start: Point,
    /// Circle-center end point before contact generation.
    #[serde(default)]
    pub swept_end: Point,
    /// First time of impact as a fraction of the sweep.
    #[serde(default)]
    pub toi: FloatNum,
    /// Final fraction used to clamp the moving body for contact generation.
    #[serde(default)]
    pub advancement: FloatNum,
    /// World-space rollback distance from the integrated end pose to the clamp.
    #[serde(default)]
    pub clamp: FloatNum,
    /// Small world-space overlap allowed at the clamp.
    #[serde(default)]
    pub slop: FloatNum,
    /// World-space point where the circle first touched the static convex.
    #[serde(default)]
    pub toi_point: Point,
}

/// Stable explanation for how a contact manifold was reduced to exported points.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContactReductionReason {
    /// A shape pair naturally produced one contact point.
    #[default]
    SinglePoint,
    /// Edge clipping produced the exported 1-2 manifold points.
    Clipped,
    /// Near-duplicate clipped points were merged to keep the manifold stable.
    DuplicateReduced,
    /// The pair is intentionally outside the M2 convex path and used the legacy fallback.
    NonM2Fallback,
    /// A convex pair outside the specialized SAT/analytic paths used GJK/EPA.
    GenericConvexFallback,
}

/// Stable termination fact for the internal GJK convex query.
///
/// GJK searches the Minkowski difference with a small simplex: a point, line, or
/// triangle that is moved toward the origin to decide separation or overlap.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GjkTerminationReason {
    /// Older serialized payloads did not include a reason.
    #[default]
    Unknown,
    /// The simplex could not pass the origin, so the shapes are separated.
    Separated,
    /// The closest simplex distance collapsed to zero without a stable penetration triangle.
    Touching,
    /// A triangle simplex contained the origin.
    Intersect,
    /// The search direction was too small or ambiguous to continue safely.
    DegenerateDirection,
    /// The iteration budget was exhausted and the query was contained.
    MaxIterations,
    /// A shape could not produce a finite support point.
    InvalidSupport,
}

/// Stable termination fact for the internal EPA penetration query.
///
/// EPA expands an intersecting GJK simplex into a small convex polytope and
/// reads the closest face to the origin as a penetration normal/depth.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EpaTerminationReason {
    /// Older serialized payloads did not include a reason.
    #[default]
    Unknown,
    /// EPA converged on a finite penetration face.
    Converged,
    /// GJK did not provide an intersecting simplex.
    GjkDidNotIntersect,
    /// The polytope edge was too small to define a stable 2D face.
    DegenerateEdge,
    /// The iteration budget was exhausted and the failure was contained.
    MaxIterations,
    /// A shape could not produce a finite support point.
    InvalidSupport,
}

/// Why the generic convex fallback was selected or contained.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GenericConvexFallbackReason {
    /// No generic fallback was involved.
    #[default]
    None,
    /// Specialized SAT/analytic paths were unavailable, so GJK/EPA owned the answer.
    GenericConvexFallback,
    /// GJK intersected but EPA could not produce a stable face, so the failure stayed contained.
    EpaFailureContained,
}

/// Compact trace facts for generic convex fallback contacts.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct GenericConvexTrace {
    /// Why the generic convex path contributed this contact or contained failure.
    #[serde(default)]
    pub fallback_reason: GenericConvexFallbackReason,
    /// GJK termination reason.
    #[serde(default)]
    pub gjk_termination: GjkTerminationReason,
    /// EPA termination reason.
    #[serde(default)]
    pub epa_termination: EpaTerminationReason,
    /// Number of GJK iterations used.
    #[serde(default)]
    pub gjk_iterations: usize,
    /// Number of EPA iterations used.
    #[serde(default)]
    pub epa_iterations: usize,
    /// Final GJK simplex size.
    #[serde(default)]
    pub simplex_len: usize,
}

/// Why a contact did or did not reuse the previous step's impulse cache.
///
/// A warm-start cache stores the normal/tangent impulses solved for a contact
/// point in the previous step. Reusing them is only safe when the same
/// geometric feature is still touching and the contact normal/point have not
/// drifted enough to make the old impulse point at the wrong constraint.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WarmStartCacheReason {
    /// Previous cache entry matched this contact and passed the conservative validity checks.
    Hit,
    /// No previous contact for the normalized collider pair was active last step.
    #[default]
    MissNoPrevious,
    /// The pair existed last step, but the point-level feature id did not match.
    MissFeatureId,
    /// The previous matching contact was sensor-only and had no solver impulse cache.
    MissPreviousSensor,
    /// The contact is a sensor-only overlap, so solver impulses are intentionally ignored.
    SkippedSensor,
    /// The feature id matched, but the current normal points in a different direction.
    DroppedNormalMismatch,
    /// The feature id matched, but the contact point moved farther than the drift threshold.
    DroppedPointDrift,
    /// The previous cached impulse was non-finite and was not transferred.
    DroppedInvalidImpulse,
}

impl WarmStartCacheReason {
    /// Returns true when previous cached impulses were transferred to this step's facts.
    pub const fn is_hit(self) -> bool {
        matches!(self, Self::Hit)
    }

    /// Returns true when no matching cache entry existed for this contact.
    pub const fn is_miss(self) -> bool {
        matches!(
            self,
            Self::MissNoPrevious
                | Self::MissFeatureId
                | Self::MissPreviousSensor
                | Self::SkippedSensor
        )
    }

    /// Returns true when a matching cache entry was found but rejected as unsafe.
    pub const fn is_drop(self) -> bool {
        matches!(
            self,
            Self::DroppedNormalMismatch | Self::DroppedPointDrift | Self::DroppedInvalidImpulse
        )
    }
}

/// Stable reason attached to sleep/wake transitions.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SleepTransitionReason {
    /// Older serialized payloads did not include a reason.
    #[default]
    Unknown,
    /// The full island stayed below motion thresholds for the configured stability window.
    StabilityWindow,
    /// A non-sensor contact solver row produced a normal impulse against a sleeping body.
    Impact,
    /// Residual contact position correction moved a sleeping body.
    ContactImpulse,
    /// A joint constraint correction moved a sleeping body.
    JointCorrection,
    /// User code edited a sleeping body's transform.
    TransformEdit,
    /// User code edited a sleeping body's velocity.
    VelocityEdit,
    /// User code explicitly woke or otherwise patched a sleeping body.
    UserPatch,
    /// Sleep was disabled at the world or step level.
    SleepDisabled,
}

/// Contact lifecycle information exposed by the stable event stream.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ContactEvent {
    /// Stable contact identity for downstream consumers.
    pub contact_id: ContactId,
    /// Stable manifold identity for the owning contact cache entry.
    pub manifold_id: ManifoldId,
    /// Stable handle for the first body in the pair.
    pub body_a: BodyHandle,
    /// Stable handle for the second body in the pair.
    pub body_b: BodyHandle,
    /// Stable handle for the first collider in the pair.
    pub collider_a: ColliderHandle,
    /// Stable handle for the second collider in the pair.
    pub collider_b: ColliderHandle,
    /// Stable geometric feature identity for this contact point.
    pub feature_id: ContactFeatureId,
    /// World-space contact position.
    pub point: Point,
    /// World-space contact normal pointing toward body A.
    pub normal: Vector,
    /// Penetration depth or separation distance for this contact.
    pub depth: FloatNum,
    /// Why this point set was reduced to the exported manifold.
    pub reduction_reason: ContactReductionReason,
    /// Warm-start cache decision for this contact point.
    #[serde(default)]
    pub warm_start_reason: WarmStartCacheReason,
    /// Previous normal impulse transferred into this step, or zero when not trusted.
    #[serde(default)]
    pub warm_start_normal_impulse: FloatNum,
    /// Previous tangent impulse transferred into this step, or zero when not trusted.
    #[serde(default)]
    pub warm_start_tangent_impulse: FloatNum,
    /// Final normal impulse accumulated by the current step's contact solver.
    #[serde(default)]
    pub solver_normal_impulse: FloatNum,
    /// Final tangent impulse accumulated by the current step's contact solver.
    #[serde(default)]
    pub solver_tangent_impulse: FloatNum,
    /// Whether the normal row tried to go below zero and was clamped.
    #[serde(default)]
    pub normal_impulse_clamped: bool,
    /// Whether the tangent row hit the Coulomb friction clamp.
    #[serde(default)]
    pub tangent_impulse_clamped: bool,
    /// Closing-speed threshold below which restitution is suppressed.
    #[serde(default)]
    pub restitution_velocity_threshold: FloatNum,
    /// Whether restitution contributed bounce bias for this contact row.
    #[serde(default)]
    pub restitution_applied: bool,
    /// GJK/EPA trace facts when this contact came from the generic convex fallback.
    #[serde(default)]
    pub generic_convex_trace: Option<GenericConvexTrace>,
    /// CCD trace facts when this contact was created by a swept TOI clamp.
    #[serde(default)]
    pub ccd_trace: Option<CcdTrace>,
}

/// Sleep or wake transitions for a body.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SleepEvent {
    /// Stable handle for the affected body.
    pub body: BodyHandle,
    /// The body's sleep state after the step completed.
    pub is_sleeping: bool,
    /// Deterministic island id assigned during this step's sleep phase.
    #[serde(default)]
    pub island_id: u32,
    /// Why this body changed sleep state.
    #[serde(default)]
    pub reason: SleepTransitionReason,
}

/// Explicit notice that the pipeline detected and contained non-finite math.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct NumericsWarningEvent {
    /// Broad pipeline phase that detected the invalid numeric value.
    pub phase: String,
    /// Short stable reason string for downstream debugging.
    pub detail: String,
}

/// Stable event stream returned by `SimulationPipeline::step`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum WorldEvent {
    /// A new contact became active during the current step.
    ContactStarted(ContactEvent),
    /// An existing contact stayed active during the current step.
    ContactPersisted(ContactEvent),
    /// A previously active contact ended during the current step.
    ContactEnded(ContactEvent),
    /// A body entered or left the sleeping state.
    SleepChanged(SleepEvent),
    /// The step detected and contained a non-finite intermediate value.
    NumericsWarning(NumericsWarningEvent),
    /// A body was created in the authoritative world state.
    BodyCreated { body: BodyHandle },
    /// A body was removed from the authoritative world state.
    BodyRemoved { body: BodyHandle },
    /// A joint was created in the authoritative world state.
    JointCreated { joint: JointHandle },
    /// A joint was removed from the authoritative world state.
    JointRemoved { joint: JointHandle },
}

#[cfg(test)]
mod tests {
    use crate::{
        events::{
            ContactEvent, ContactReductionReason, NumericsWarningEvent, SleepEvent,
            SleepTransitionReason, WarmStartCacheReason, WorldEvent,
        },
        handles::{
            BodyHandle, ColliderHandle, ContactFeatureId, ContactId, JointHandle, ManifoldId,
        },
        math::{point::Point, vector::Vector},
    };

    #[test]
    fn world_event_variants_preserve_payloads() {
        let contact = ContactEvent {
            contact_id: ContactId::from_raw_parts(1, 0),
            manifold_id: ManifoldId::from_raw_parts(2, 0),
            body_a: BodyHandle::from_raw_parts(3, 0),
            body_b: BodyHandle::from_raw_parts(4, 0),
            collider_a: ColliderHandle::from_raw_parts(5, 0),
            collider_b: ColliderHandle::from_raw_parts(6, 0),
            feature_id: ContactFeatureId::from_raw_parts(7, 0),
            point: Point::new(7.0, 8.0),
            normal: Vector::new(0.0, 1.0),
            depth: 0.25,
            reduction_reason: ContactReductionReason::Clipped,
            warm_start_reason: WarmStartCacheReason::Hit,
            warm_start_normal_impulse: 1.0,
            warm_start_tangent_impulse: -0.25,
            solver_normal_impulse: 1.25,
            solver_tangent_impulse: -0.125,
            normal_impulse_clamped: false,
            tangent_impulse_clamped: true,
            restitution_velocity_threshold: 1.0,
            restitution_applied: true,
            generic_convex_trace: None,
            ccd_trace: None,
        };
        let sleep = SleepEvent {
            body: BodyHandle::from_raw_parts(9, 0),
            is_sleeping: true,
            island_id: 1,
            reason: SleepTransitionReason::StabilityWindow,
        };
        let numerics = NumericsWarningEvent {
            phase: "integrate".into(),
            detail: "body_state".into(),
        };

        let events = [
            WorldEvent::ContactStarted(contact),
            WorldEvent::ContactPersisted(contact),
            WorldEvent::ContactEnded(contact),
            WorldEvent::SleepChanged(sleep),
            WorldEvent::NumericsWarning(numerics.clone()),
            WorldEvent::BodyCreated {
                body: BodyHandle::from_raw_parts(10, 0),
            },
            WorldEvent::BodyRemoved {
                body: BodyHandle::from_raw_parts(11, 0),
            },
            WorldEvent::JointCreated {
                joint: JointHandle::from_raw_parts(12, 0),
            },
            WorldEvent::JointRemoved {
                joint: JointHandle::from_raw_parts(13, 0),
            },
        ];

        assert!(matches!(&events[0], WorldEvent::ContactStarted(event) if event == &contact));
        assert!(matches!(&events[1], WorldEvent::ContactPersisted(event) if event == &contact));
        assert!(matches!(&events[2], WorldEvent::ContactEnded(event) if event == &contact));
        assert!(matches!(&events[3], WorldEvent::SleepChanged(event) if event == &sleep));
        assert!(matches!(&events[4], WorldEvent::NumericsWarning(event) if event == &numerics));
        assert!(
            matches!(&events[5], WorldEvent::BodyCreated { body } if *body == BodyHandle::from_raw_parts(10, 0))
        );
        assert!(
            matches!(&events[6], WorldEvent::BodyRemoved { body } if *body == BodyHandle::from_raw_parts(11, 0))
        );
        assert!(
            matches!(&events[7], WorldEvent::JointCreated { joint } if *joint == JointHandle::from_raw_parts(12, 0))
        );
        assert!(
            matches!(&events[8], WorldEvent::JointRemoved { joint } if *joint == JointHandle::from_raw_parts(13, 0))
        );
    }
}
