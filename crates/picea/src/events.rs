//! Typed world events emitted by the v1 simulation pipeline.

use serde::{Deserialize, Serialize};

use crate::{
    handles::{BodyHandle, ColliderHandle, ContactId, JointHandle, ManifoldId},
    math::{point::Point, vector::Vector, FloatNum},
};

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
    /// World-space contact position.
    pub point: Point,
    /// World-space contact normal pointing toward body A.
    pub normal: Vector,
    /// Penetration depth or separation distance for this contact.
    pub depth: FloatNum,
}

/// Sleep or wake transitions for a body.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SleepEvent {
    /// Stable handle for the affected body.
    pub body: BodyHandle,
    /// The body's sleep state after the step completed.
    pub is_sleeping: bool,
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
        events::{ContactEvent, SleepEvent, WorldEvent},
        handles::{BodyHandle, ColliderHandle, ContactId, JointHandle, ManifoldId},
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
            point: Point::new(7.0, 8.0),
            normal: Vector::new(0.0, 1.0),
            depth: 0.25,
        };
        let sleep = SleepEvent {
            body: BodyHandle::from_raw_parts(9, 0),
            is_sleeping: true,
        };

        let events = [
            WorldEvent::ContactStarted(contact),
            WorldEvent::ContactPersisted(contact),
            WorldEvent::ContactEnded(contact),
            WorldEvent::SleepChanged(sleep),
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
        assert!(matches!(&events[4], WorldEvent::BodyCreated { body } if *body == BodyHandle::from_raw_parts(10, 0)));
        assert!(matches!(&events[5], WorldEvent::BodyRemoved { body } if *body == BodyHandle::from_raw_parts(11, 0)));
        assert!(matches!(&events[6], WorldEvent::JointCreated { joint } if *joint == JointHandle::from_raw_parts(12, 0)));
        assert!(matches!(&events[7], WorldEvent::JointRemoved { joint } if *joint == JointHandle::from_raw_parts(13, 0)));
    }
}
