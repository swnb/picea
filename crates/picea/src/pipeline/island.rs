use std::collections::BTreeMap;

use crate::{
    handles::BodyHandle,
    joint::{DistanceJointDesc, JointDesc, WorldAnchorJointDesc},
    pipeline::sleep::SolverIsland,
};

#[derive(Clone, Debug)]
pub(crate) struct IslandSolvePlan {
    pub(crate) islands: Vec<IslandSolveBatch>,
}

#[derive(Clone, Debug)]
pub(crate) struct IslandSolveBatch {
    #[allow(dead_code)]
    pub(crate) island_id: u32,
    pub(crate) body_slots: Vec<BodyHandle>,
    pub(crate) contact_rows: Vec<ContactSolvePlanRow>,
    pub(crate) joint_rows: Vec<JointSolvePlanRow>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct SolverStepStats {
    pub(crate) island_count: usize,
    pub(crate) active_island_count: usize,
    pub(crate) sleeping_island_skip_count: usize,
    pub(crate) body_slot_count: usize,
    pub(crate) contact_row_count: usize,
    pub(crate) joint_row_count: usize,
}

impl SolverStepStats {
    pub(crate) fn accumulate(&mut self, other: Self) {
        self.island_count += other.island_count;
        self.active_island_count += other.active_island_count;
        self.sleeping_island_skip_count += other.sleeping_island_skip_count;
        self.body_slot_count += other.body_slot_count;
        self.contact_row_count += other.contact_row_count;
        self.joint_row_count += other.joint_row_count;
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ContactSolvePlanRow {
    pub(crate) contact_index: usize,
    pub(crate) body_a_slot: usize,
    pub(crate) body_b_slot: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum JointSolvePlanRow {
    Distance {
        desc: DistanceJointDesc,
        body_a_slot: usize,
        body_b_slot: usize,
    },
    WorldAnchor {
        desc: WorldAnchorJointDesc,
        body_slot: usize,
    },
}

pub(crate) fn build_island_solve_plan<I, J>(
    islands: &[SolverIsland],
    contact_rows: I,
    joint_descs: J,
) -> IslandSolvePlan
where
    I: IntoIterator<Item = (usize, BodyHandle, BodyHandle)>,
    J: IntoIterator<Item = JointDesc>,
{
    let mut builders = islands
        .iter()
        .filter(|island| island.active)
        .map(|island| IslandSolveBatchBuilder::new(island.id, island.bodies.clone()))
        .collect::<Vec<_>>();
    let active_island_ids = builders
        .iter()
        .enumerate()
        .map(|(index, island)| (island.island_id, index))
        .collect::<BTreeMap<_, _>>();
    let body_islands = builders
        .iter()
        .flat_map(|island| {
            island
                .body_slots
                .iter()
                .map(move |body| (*body, island.island_id))
        })
        .collect::<BTreeMap<_, _>>();

    for (contact_index, body_a, body_b) in contact_rows {
        let Some(builder) = contact_target_island(body_a, body_b, &body_islands)
            .and_then(|island_id| active_island_ids.get(&island_id).copied())
            .and_then(|index| builders.get_mut(index))
        else {
            continue;
        };
        let body_a_slot = builder.ensure_slot(body_a);
        let body_b_slot = builder.ensure_slot(body_b);
        builder.contact_rows.push(ContactSolvePlanRow {
            contact_index,
            body_a_slot,
            body_b_slot,
        });
    }

    for desc in joint_descs {
        match desc {
            JointDesc::Distance(desc) => {
                let Some(builder) = contact_target_island(desc.body_a, desc.body_b, &body_islands)
                    .and_then(|island_id| active_island_ids.get(&island_id).copied())
                    .and_then(|index| builders.get_mut(index))
                else {
                    continue;
                };
                let body_a_slot = builder.ensure_slot(desc.body_a);
                let body_b_slot = builder.ensure_slot(desc.body_b);
                builder.joint_rows.push(JointSolvePlanRow::Distance {
                    desc,
                    body_a_slot,
                    body_b_slot,
                });
            }
            JointDesc::WorldAnchor(desc) => {
                let Some(builder) = body_islands
                    .get(&desc.body)
                    .and_then(|island_id| active_island_ids.get(island_id).copied())
                    .and_then(|index| builders.get_mut(index))
                else {
                    continue;
                };
                let body_slot = builder.ensure_slot(desc.body);
                builder
                    .joint_rows
                    .push(JointSolvePlanRow::WorldAnchor { desc, body_slot });
            }
        }
    }

    IslandSolvePlan {
        islands: builders
            .into_iter()
            .map(IslandSolveBatchBuilder::build)
            .collect(),
    }
}

fn contact_target_island(
    body_a: BodyHandle,
    body_b: BodyHandle,
    body_islands: &BTreeMap<BodyHandle, u32>,
) -> Option<u32> {
    match (
        body_islands.get(&body_a).copied(),
        body_islands.get(&body_b).copied(),
    ) {
        (Some(a), Some(b)) if a == b => Some(a),
        (Some(a), None) => Some(a),
        (None, Some(b)) => Some(b),
        _ => None,
    }
}

struct IslandSolveBatchBuilder {
    island_id: u32,
    body_slots: Vec<BodyHandle>,
    slot_by_handle: BTreeMap<BodyHandle, usize>,
    contact_rows: Vec<ContactSolvePlanRow>,
    joint_rows: Vec<JointSolvePlanRow>,
}

impl IslandSolveBatchBuilder {
    fn new(island_id: u32, body_slots: Vec<BodyHandle>) -> Self {
        let slot_by_handle = body_slots
            .iter()
            .enumerate()
            .map(|(index, body)| (*body, index))
            .collect();
        Self {
            island_id,
            body_slots,
            slot_by_handle,
            contact_rows: Vec::new(),
            joint_rows: Vec::new(),
        }
    }

    fn ensure_slot(&mut self, body: BodyHandle) -> usize {
        if let Some(slot) = self.slot_by_handle.get(&body).copied() {
            return slot;
        }
        // Public handles remain the stable external identity; the dense slot is
        // only an island-local execution index for this step's hot solver rows.
        let slot = self.body_slots.len();
        self.body_slots.push(body);
        self.slot_by_handle.insert(body, slot);
        slot
    }

    fn build(self) -> IslandSolveBatch {
        IslandSolveBatch {
            island_id: self.island_id,
            body_slots: self.body_slots,
            contact_rows: self.contact_rows,
            joint_rows: self.joint_rows,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        handles::BodyHandle,
        joint::{DistanceJointDesc, JointDesc},
        math::point::Point,
    };

    #[test]
    fn dense_plan_keeps_deterministic_island_order_and_slots() {
        let dynamic_a = BodyHandle::from_raw_parts(10, 0);
        let dynamic_b = BodyHandle::from_raw_parts(20, 0);
        let static_body = BodyHandle::from_raw_parts(1, 0);
        let dynamic_c = BodyHandle::from_raw_parts(30, 0);
        let plan = build_island_solve_plan(
            &[
                SolverIsland {
                    id: 7,
                    bodies: vec![dynamic_a, dynamic_b],
                    active: true,
                },
                SolverIsland {
                    id: 9,
                    bodies: vec![dynamic_c],
                    active: true,
                },
            ],
            [
                (4usize, dynamic_b, static_body),
                (2usize, dynamic_a, dynamic_b),
                (8usize, dynamic_c, static_body),
            ],
            [JointDesc::Distance(DistanceJointDesc {
                body_a: dynamic_a,
                body_b: static_body,
                local_anchor_a: Point::default(),
                local_anchor_b: Point::default(),
                rest_length: 1.0,
                stiffness: 1.0,
                damping: 0.0,
                user_data: 0,
            })],
        );

        assert_eq!(plan.islands.len(), 2);
        assert_eq!(plan.islands[0].island_id, 7);
        assert_eq!(
            plan.islands[0].body_slots,
            vec![dynamic_a, dynamic_b, static_body]
        );
        assert_eq!(
            plan.islands[0].contact_rows,
            vec![
                ContactSolvePlanRow {
                    contact_index: 4,
                    body_a_slot: 1,
                    body_b_slot: 2,
                },
                ContactSolvePlanRow {
                    contact_index: 2,
                    body_a_slot: 0,
                    body_b_slot: 1,
                },
            ]
        );
        assert_eq!(plan.islands[1].island_id, 9);
        assert_eq!(plan.islands[1].body_slots, vec![dynamic_c, static_body]);
        assert_eq!(
            plan.islands[1].contact_rows,
            vec![ContactSolvePlanRow {
                contact_index: 8,
                body_a_slot: 0,
                body_b_slot: 1,
            }]
        );
        assert_eq!(plan.islands[1].joint_rows.len(), 0);
        assert_eq!(plan.islands[0].joint_rows.len(), 1);
    }

    #[test]
    fn sleeping_islands_do_not_allocate_hot_rows() {
        let sleeping_body = BodyHandle::from_raw_parts(40, 0);
        let awake_body = BodyHandle::from_raw_parts(50, 0);
        let static_body = BodyHandle::from_raw_parts(1, 0);

        let plan = build_island_solve_plan(
            &[
                SolverIsland {
                    id: 1,
                    bodies: vec![sleeping_body],
                    active: false,
                },
                SolverIsland {
                    id: 2,
                    bodies: vec![awake_body],
                    active: true,
                },
            ],
            [
                (0usize, sleeping_body, static_body),
                (1usize, awake_body, static_body),
            ],
            [JointDesc::WorldAnchor(WorldAnchorJointDesc {
                body: sleeping_body,
                local_anchor: Point::default(),
                world_anchor: Point::default(),
                stiffness: 1.0,
                damping: 0.0,
                user_data: 0,
            })],
        );

        assert_eq!(plan.islands.len(), 1);
        assert_eq!(plan.islands[0].island_id, 2);
        assert_eq!(plan.islands[0].contact_rows.len(), 1);
        assert!(plan.islands[0].joint_rows.is_empty());
        assert_eq!(plan.islands[0].body_slots, vec![awake_body, static_body]);
    }
}
